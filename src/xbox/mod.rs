mod manifest;
mod platform;
mod process;

use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
};
use std::path::PathBuf;

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

/// Returns Xbox games from the folders declared by `ModifiableWindowsApps`
/// and `.GamingRoot`, following the GameFinder discovery model.
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

/// Looks up a single Xbox game by its manifest `Identity.Name`.
pub fn find(id: &str) -> Result<Game> {
    let all = games()?;
    all.into_iter().find(|g| g.id == id).ok_or_else(|| {
        Error::new(
            ErrorKind::GameNotFound,
            format!("Xbox game with id ({id}) not found"),
        )
    })
}

/// GameFinder-style disk discovery does not provide a package full name,
/// so these results cannot be uninstalled through `RemovePackageAsync`.
pub fn uninstall(_game: &Game) -> Result<()> {
    Err(Error::new(
        ErrorKind::InvalidGame,
        "Xbox games discovered from disk do not expose an uninstall command",
    ))
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
    let mut out = Vec::new();
    for manifest_path in platform::get_game_manifests() {
        match build_game(&manifest_path) {
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
fn build_game(manifest_path: &std::path::Path) -> Result<Game> {
    let parsed = manifest::parse(manifest_path)?;
    let game_path = manifest_path.parent().ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidManifest,
            format!(
                "Manifest has no parent directory: {}",
                manifest_path.display()
            ),
        )
    })?;

    let mut game = Game::default();
    game._type = String::from("xbox");
    game.id = parsed.identity_name;
    game.name = parsed.display_name;
    game.path = Some(game_path.to_path_buf());

    game.state.installed = true;

    Ok(game)
}
