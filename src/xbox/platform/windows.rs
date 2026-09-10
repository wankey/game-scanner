use crate::error::{Error, ErrorKind, Result};
use std::{
    fs,
    io::Read,
    mem::size_of,
    path::{Component, Path, PathBuf},
};
use windows::{
    core::HSTRING,
    Management::Deployment::PackageManager,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        Storage::FileSystem::GetLogicalDrives,
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

const GAMING_ROOT_MAGIC: u32 = 0x5842_4752;

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

fn packages_for_current_scope(
    pm: &PackageManager,
) -> Result<impl IntoIterator<Item = windows::ApplicationModel::Package>> {
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
    packages.map_err(|e| Error::new(ErrorKind::IO, format!("{operation} failed: {e}")))
}

pub fn get_game_manifests() -> Vec<PathBuf> {
    let app_folders = get_app_folders_from_roots(root_directories());
    get_game_manifests_from_folders(&app_folders)
}

fn root_directories() -> Vec<PathBuf> {
    let drives = unsafe { GetLogicalDrives() };
    (0..26)
        .filter(|index| drives & (1 << index) != 0)
        .map(|index| PathBuf::from(format!("{}:\\", (b'A' + index as u8) as char)))
        .collect()
}

fn get_app_folders_from_roots(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    for root in roots {
        let modifiable_windows_apps = root.join("Program Files").join("ModifiableWindowsApps");
        if modifiable_windows_apps.is_dir() {
            folders.push(modifiable_windows_apps);
        }

        let gaming_root = root.join(".GamingRoot");
        if !gaming_root.is_file() {
            continue;
        }

        match parse_gaming_root(&gaming_root) {
            Ok(additional_folders) => folders.extend(additional_folders),
            Err(error) => crate::error::print_error(&error),
        }
    }
    folders
}

fn parse_gaming_root(path: &Path) -> Result<Vec<PathBuf>> {
    let mut file = fs::File::open(path).map_err(|e| {
        Error::new(
            ErrorKind::IO,
            format!("Unable to open .GamingRoot at {}: {e}", path.display()),
        )
    })?;
    let mut header = [0; 8];
    file.read_exact(&mut header).map_err(|e| {
        Error::new(
            ErrorKind::InvalidManifest,
            format!("Unable to read .GamingRoot at {}: {e}", path.display()),
        )
    })?;

    let magic = u32::from_le_bytes(header[..4].try_into().unwrap());
    if magic != GAMING_ROOT_MAGIC {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            format!("Invalid .GamingRoot magic at {}", path.display()),
        ));
    }

    let folder_count = u32::from_le_bytes(header[4..].try_into().unwrap());
    if folder_count >= u8::MAX as u32 {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            format!("Too many folders in .GamingRoot at {}", path.display()),
        ));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let mut folders = Vec::with_capacity(folder_count as usize);
    for _ in 0..folder_count {
        let mut units = Vec::new();
        loop {
            let mut bytes = [0; 2];
            file.read_exact(&mut bytes).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidManifest,
                    format!(
                        "Unable to read folder from .GamingRoot at {}: {e}",
                        path.display()
                    ),
                )
            })?;
            let unit = u16::from_le_bytes(bytes);
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        let folder = String::from_utf16(&units).map_err(|e| {
            Error::new(
                ErrorKind::InvalidManifest,
                format!(
                    "Invalid UTF-16 folder in .GamingRoot at {}: {e}",
                    path.display()
                ),
            )
        })?;
        let relative = Path::new(&folder);
        if relative.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        }) {
            return Err(Error::new(
                ErrorKind::InvalidManifest,
                format!("Invalid folder in .GamingRoot at {}", path.display()),
            ));
        }
        folders.push(parent.join(relative));
    }
    Ok(folders)
}

fn get_game_manifests_from_folders(app_folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    for app_folder in app_folders {
        let entries = match fs::read_dir(app_folder) {
            Ok(entries) => entries,
            Err(error) => {
                crate::error::print_error(&Error::new(
                    ErrorKind::IO,
                    format!(
                        "Unable to read Xbox app folder {}: {error}",
                        app_folder.display()
                    ),
                ));
                continue;
            }
        };
        for entry in entries.flatten() {
            let game_folder = entry.path();
            if !game_folder.is_dir() {
                continue;
            }
            let manifest = game_folder.join("AppxManifest.xml");
            if manifest.is_file() {
                manifests.push(manifest);
                continue;
            }
            let content_manifest = game_folder.join("Content").join("AppxManifest.xml");
            if content_manifest.is_file() {
                manifests.push(content_manifest);
            }
        }
    }
    manifests
}

pub fn get_launcher_executable() -> Result<PathBuf> {
    let pm = PackageManager::new()
        .map_err(|e| Error::new(ErrorKind::IO, format!("PackageManager::new failed: {e}")))?;
    let packages = packages_for_current_scope(&pm)?;
    for pkg in packages {
        let id = match pkg.Id() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let family_name = match id.FamilyName() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };

        if !family_name.starts_with("Microsoft.GamingApp") {
            continue;
        }
        let install_location = match pkg.InstalledPath() {
            Ok(s) => PathBuf::from(s.to_string_lossy()),
            Err(_) => continue,
        };
        let executable = install_location.join("XboxApp.exe");
        if executable.is_file() {
            return Ok(executable);
        }
    }
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox app is not installed (no Microsoft.GamingApp_* package found)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_gaming_root(path: &Path, folders: &[&str]) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(&GAMING_ROOT_MAGIC.to_le_bytes()).unwrap();
        file.write_all(&(folders.len() as u32).to_le_bytes())
            .unwrap();
        for folder in folders {
            for unit in folder.encode_utf16() {
                file.write_all(&unit.to_le_bytes()).unwrap();
            }
            file.write_all(&0u16.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn package_scope_uses_all_users_for_an_elevated_process() {
        assert_eq!(package_scope(true), PackageScope::AllUsers);
    }

    #[test]
    fn package_scope_uses_current_user_for_a_non_elevated_process() {
        assert_eq!(package_scope(false), PackageScope::CurrentUser);
    }

    #[test]
    fn finds_modifiable_windows_apps_and_gaming_root_folders() {
        let root = tempfile::tempdir().unwrap();
        let modifiable = root
            .path()
            .join("Program Files")
            .join("ModifiableWindowsApps");
        let xbox_games = root.path().join("XboxGames");
        fs::create_dir_all(&modifiable).unwrap();
        fs::create_dir_all(&xbox_games).unwrap();
        write_gaming_root(&root.path().join(".GamingRoot"), &["XboxGames"]);

        let folders = get_app_folders_from_roots(vec![root.path().to_path_buf()]);
        assert_eq!(folders, vec![modifiable, xbox_games]);
    }

    #[test]
    fn finds_manifests_in_game_folder_or_content_subfolder() {
        let root = tempfile::tempdir().unwrap();
        let direct = root.path().join("Direct");
        let content = root.path().join("Content").join("Content");
        fs::create_dir_all(&direct).unwrap();
        fs::create_dir_all(&content).unwrap();
        fs::write(direct.join("AppxManifest.xml"), "<Package />").unwrap();
        fs::write(content.join("AppxManifest.xml"), "<Package />").unwrap();

        let mut manifests = get_game_manifests_from_folders(&[root.path().to_path_buf()]);
        manifests.sort();
        let mut expected = vec![
            direct.join("AppxManifest.xml"),
            content.join("AppxManifest.xml"),
        ];
        expected.sort();
        assert_eq!(manifests, expected);
    }
}
