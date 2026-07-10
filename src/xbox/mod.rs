mod manifest;
mod platform;
mod process;

use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use self::platform::XboxPackage;

/// Returns the path to `XboxApp.exe` inside the `Microsoft.GamingApp_*`
/// package, or `LauncherNotFound` if the Xbox App is not installed.
pub fn executable() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return platform::get_launcher_executable();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Returns every installed Microsoft Store / Xbox App game this machine
/// exposes. The list is filtered by an internal allow-list of FamilyName
/// prefixes (see `platform::windows::XBOX_FAMILY_PREFIXES`).
pub fn games() -> Result<Vec<Game>> {
    #[cfg(target_os = "windows")]
    {
        return scan_all();
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Looks up a single Xbox game by its `PackageFullName`.
pub fn find(id: &str) -> Result<Game> {
    // TODO(perf): cache the WinRT scan if `find` is called frequently;
    // today each call re-runs `PackageManager::FindPackages` + N manifest parses.
    let all = games()?;
    all.into_iter().find(|g| g.id == id).ok_or_else(|| {
        Error::new(
            ErrorKind::GameNotFound,
            format!("Xbox game with id ({id}) not found"),
        )
    })
}

/// Uninstalls the supplied Xbox game via the WinRT `RemovePackageAsync` API.
pub fn uninstall(game: &Game) -> Result<()> {
    let cmd = game.commands.uninstall.as_ref().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall called with a Game that has no uninstall command",
        )
    })?;
    if cmd.first().map(|s| s.as_str()) != Some("__xbox__:remove_package") {
        return Err(Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall called with a non-xbox game (sentinel mismatch)",
        ));
    }
    let full_name = cmd.get(1).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidGame,
            "xbox::uninstall command missing the package full name",
        )
    })?;

    #[cfg(target_os = "windows")]
    {
        platform::remove_package(full_name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = full_name;
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "Xbox / Microsoft Store is not supported on this platform",
        ))
    }
}

/// Returns the PIDs of processes whose exe path or cwd falls under the
/// game's `install_location`. Pure-UWP processes whose host is
/// `svchost.exe` or a generic runtime broker are not detected — a known
/// v1 limitation.
pub fn processes(game: &Game) -> Option<Vec<u32>> {
    Some(process::pids(game))
}

#[cfg(target_os = "windows")]
fn scan_all() -> Result<Vec<Game>> {
    let packages = platform::get_packages()?;
    let mut out = Vec::new();
    for pkg in packages {
        match build_game(pkg) {
            Ok(g) => out.push(g),
            Err(e) => {
                // Per spec: one bad manifest must not abort the whole scan.
                crate::error::print_error(&e);
            }
        }
    }
    Ok(out)
}

#[cfg(target_os = "windows")]
fn build_game(pkg: XboxPackage) -> Result<Game> {
    let manifest_path = pkg.install_location.join("AppxManifest.xml");
    let parsed = manifest::parse(&manifest_path)?;

    let mut game = Game::default();
    game._type = String::from("xbox");
    game.id = pkg.full_name.clone();
    game.name = parsed.display_name;
    game.path = Some(pkg.install_location.clone());

    game.commands.install = None;
    game.commands.launch = Some(vec![
        String::from("explorer.exe"),
        format!(
            "shell:AppsFolder\\{}!{}",
            pkg.family_name, parsed.application_id
        ),
    ]);
    game.commands.uninstall = Some(vec![String::from("__xbox__:remove_package"), pkg.full_name]);

    game.state.installed = true;

    Ok(game)
}
