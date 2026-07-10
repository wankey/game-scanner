use crate::error::{Error, ErrorKind, Result};
use std::{mem::size_of, path::PathBuf};
use windows::{
    core::HSTRING,
    Management::Deployment::PackageManager,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

const XBOX_FAMILY_PREFIXES: &[&str] = &[
    // Microsoft-first-party Xbox / Gaming clients and overlays
    "Microsoft.GamingApp",
    "Microsoft.XboxApp",
    "Microsoft.XboxGameOverlay",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.XboxGameCallableUI",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.Xbox.TCUI",
    // Microsoft Studios first-party IPs that ship as MSIX
    "Microsoft.MicrosoftSolitaireCollection",
    "Microsoft.MinecraftUWP",
    "Microsoft.Mojang",
    "Microsoft.Halo",
    "Microsoft.Forza",
    "Microsoft.AgeOfEmpires",
    "Microsoft.GearsOfWar",
    "Microsoft.FlightSimulator",
    "Microsoft.Oslo",
    // Third-party publishers commonly appearing in Xbox Game Pass
    "Bethesda",
    "2K",
    "Activision",
    "ElectronicArts",
    "Rockstar",
    "Ubisoft",
    "SEGA",
    "SquareEnix",
    "BandaiNamco",
    "ParadoxInteractive",
    "CoffeeStainStudios",
    "DevolverDigital",
    "505Games",
    "DeepSilver",
    "FocusHomeInteractive",
    "KalypsoMedia",
];

fn is_game(family_name: &str) -> bool {
    XBOX_FAMILY_PREFIXES
        .iter()
        .any(|prefix| family_name.starts_with(prefix))
}

pub struct XboxPackage {
    pub full_name: String,
    pub family_name: String,
    pub install_location: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageScope {
    AllUsers,
    CurrentUser,
}

fn package_scope(is_elevated: bool) -> PackageScope {
    if is_elevated {
        PackageScope::AllUsers
    } else {
        PackageScope::CurrentUser
    }
}

fn current_process_is_elevated() -> Result<bool> {
    let mut token = HANDLE::default();
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|e| Error::new(ErrorKind::IO, format!("OpenProcessToken failed: {e}")))?;

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0;
        let query_result = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );
        let close_result = CloseHandle(token);

        query_result.map_err(|e| {
            Error::new(
                ErrorKind::IO,
                format!("GetTokenInformation(TokenElevation) failed: {e}"),
            )
        })?;
        close_result.map_err(|e| Error::new(ErrorKind::IO, format!("CloseHandle failed: {e}")))?;

        Ok(elevation.TokenIsElevated != 0)
    }
}

pub fn get_packages() -> Result<Vec<XboxPackage>> {
    let pm = PackageManager::new()
        .map_err(|e| Error::new(ErrorKind::IO, format!("PackageManager::new failed: {e}")))?;

    // `FindPackages` needs elevation because it enumerates every user. An
    // empty SID selects only the current user through the SID-based overload.
    let (packages, operation) = match package_scope(current_process_is_elevated()?) {
        PackageScope::AllUsers => (pm.FindPackages(), "FindPackages"),
        PackageScope::CurrentUser => {
            let current_user = HSTRING::new();
            (
                pm.FindPackagesByUserSecurityId(&current_user),
                "FindPackagesByUserSecurityId",
            )
        }
    };
    let packages =
        packages.map_err(|e| Error::new(ErrorKind::IO, format!("{operation} failed: {e}")))?;

    let mut out = Vec::new();
    for pkg in packages {
        let id = match pkg.Id() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let family_name = match id.FamilyName() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        if !is_game(&family_name) {
            continue;
        }

        let full_name = match id.FullName() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        // The projection exposes `InstalledPath` (HSTRING) and
        // `InstalledLocation` (StorageFolder). `InstalledPath` is the
        // direct string and avoids the StorageFolder round-trip.
        let install_location = match pkg.InstalledPath() {
            Ok(s) => PathBuf::from(s.to_string_lossy()),
            Err(_) => continue,
        };

        out.push(XboxPackage {
            full_name,
            family_name,
            install_location,
        });
    }

    Ok(out)
}

pub fn get_launcher_executable() -> Result<PathBuf> {
    let packages = get_packages()?;
    let gaming_app = packages
        .iter()
        .find(|p| p.family_name.starts_with("Microsoft.GamingApp"))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::LauncherNotFound,
                "Xbox app is not installed (no Microsoft.GamingApp_* package found)",
            )
        })?;
    let exe = gaming_app.install_location.join("XboxApp.exe");
    if !exe.exists() {
        return Err(Error::new(
            ErrorKind::LauncherNotFound,
            format!("XboxApp.exe missing at {}", exe.display()),
        ));
    }
    Ok(exe)
}

pub fn remove_package(full_name: &str) -> Result<()> {
    let pm = PackageManager::new()
        .map_err(|e| Error::new(ErrorKind::IO, format!("PackageManager::new failed: {e}")))?;
    let hn = HSTRING::from(full_name);
    let op = pm
        .RemovePackageAsync(&hn)
        .map_err(|e| Error::new(ErrorKind::IO, format!("RemovePackageAsync failed: {e}")))?;
    // `IAsyncOperationWithProgress::GetResults` blocks (internally
    // joins) until the WinRT operation completes and returns the
    // final `DeploymentResult`. We don't need to inspect it here —
    // a non-error result means the removal succeeded.
    op.GetResults()
        .map_err(|e| Error::new(ErrorKind::IO, format!("RemovePackageAsync wait failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_scope_uses_all_users_for_an_elevated_process() {
        assert_eq!(package_scope(true), PackageScope::AllUsers);
    }

    #[test]
    fn package_scope_uses_current_user_for_a_non_elevated_process() {
        assert_eq!(package_scope(false), PackageScope::CurrentUser);
    }

    #[test]
    fn is_game_matches_first_party_xbox_clients() {
        for name in [
            "Microsoft.GamingApp_8wekyb3d8bbwe",
            "Microsoft.XboxApp_8wekyb3d8bbwe",
            "Microsoft.MinecraftUWP_8wekyb3d8bbwe",
            "Microsoft.Halo_8wekyb3d8bbwe",
            "Microsoft.MicrosoftSolitaireCollection_8wekyb3d8bbwe",
        ] {
            assert!(is_game(name), "should match: {name}");
        }
    }

    #[test]
    fn is_game_matches_third_party_publishers() {
        for name in [
            "Bethesda.SomeGame_abc123",
            "2K.NBA2K25_def456",
            "Activision.CallOfDuty_ghi789",
            "Ubisoft.AssassinsCreed_jkl012",
        ] {
            assert!(is_game(name), "should match: {name}");
        }
    }

    #[test]
    fn is_game_rejects_non_games() {
        for name in [
            "Microsoft.MicrosoftOfficeHub_8wekyb3d8bbwe",
            "Microsoft.WindowsStore_8wekyb3d8bbwe",
            "Microsoft.ScreenSketch_8wekyb3d8bbwe",
            "Microsoft.YourPhone_8wekyb3d8bbwe",
            "Microsoft.BingNews_8wekyb3d8bbwe",
        ] {
            assert!(!is_game(name), "should NOT match: {name}");
        }
    }

    #[test]
    fn is_game_empty_string_returns_false() {
        assert!(!is_game(""));
    }

    #[test]
    fn is_game_matches_every_listed_prefix() {
        // Regression guard: a typo or accidental removal of any entry
        // in XBOX_FAMILY_PREFIXES should fail this test.
        for prefix in XBOX_FAMILY_PREFIXES {
            let synthetic = format!("{prefix}.SomeGame_abc123");
            assert!(is_game(&synthetic), "prefix `{prefix}` should match");
        }
    }
}
