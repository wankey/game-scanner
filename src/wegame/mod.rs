//! Installed Rail games discovered from Windows uninstall registrations.
#[cfg(any(windows, test))]
mod manifest;
#[cfg(windows)]
mod windows;

use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
};
use std::path::PathBuf;

/// Locates WeGame independently of the installed game list.
pub fn executable() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        windows::executable()
    }
    #[cfg(not(windows))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "WeGame is only supported on Windows",
        ))
    }
}

/// Returns registered games with valid Rail metadata and payload.
/// Update/download detection is unsupported; their booleans remain false.
pub fn games() -> Result<Vec<Game>> {
    #[cfg(windows)]
    {
        Ok(scan(windows::entries()?))
    }
    #[cfg(not(windows))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "WeGame is only supported on Windows",
        ))
    }
}

pub fn find(id: &str) -> Result<Game> {
    let valid = id
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .is_some_and(|n| n.to_string() == id);
    if !valid {
        return Err(Error::new(
            ErrorKind::InvalidGame,
            "WeGame ID must be a canonical positive decimal integer",
        ));
    }
    games()?
        .into_iter()
        .find(|game| game.id == id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::GameNotFound,
                format!("WeGame game {id} not found"),
            )
        })
}

#[cfg(any(windows, test))]
#[derive(Default)]
struct Entry {
    priority: u8,
    key: String,
    name: String,
    roots: Vec<PathBuf>,
    uninstaller: Option<PathBuf>,
}

#[cfg(any(windows, test))]
fn scan(mut entries: Vec<Entry>) -> Vec<Game> {
    use std::collections::{BTreeMap, HashSet};
    entries.sort_by(|a, b| (a.priority, &a.key).cmp(&(b.priority, &b.key)));
    let mut paths = HashSet::new();
    let mut games = BTreeMap::<String, Game>::new();
    for entry in entries {
        for root in entry.roots {
            let display_root = root.clone();
            let root = match root.canonicalize() {
                Ok(path) => path,
                Err(error) => {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        crate::error::print_error(&Error::new(ErrorKind::IO, error));
                    }
                    continue;
                }
            };
            if !paths.insert(root.clone()) {
                continue;
            }
            let uninstaller = entry
                .uninstaller
                .as_ref()
                .and_then(|p| p.canonicalize().ok());
            match manifest::installed_id(&root, uninstaller.as_deref()) {
                Ok(Some(id)) => {
                    let mut game = Game::default();
                    game._type = "wegame".into();
                    game.id = id.clone();
                    game.name = if entry.name.trim().is_empty() {
                        root.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| format!("WeGame {id}"))
                    } else {
                        entry.name.clone()
                    };
                    game.path = Some(display_root);
                    game.state.installed = true;
                    if let Some(previous) = games.get(&id) {
                        crate::error::print_error(&Error::new(
                            ErrorKind::InvalidGame,
                            format!("Multiple WeGame installations for {id}"),
                        ));
                        if previous
                            .path
                            .as_ref()
                            .and_then(|p| p.canonicalize().ok())
                            .as_ref()
                            <= Some(&root)
                        {
                            continue;
                        }
                    }
                    games.insert(id, game);
                }
                Ok(None) => {}
                Err(error) => crate::error::print_error(&Error::new(
                    error.kind(),
                    format!("{}: {error}", root.display()),
                )),
            }
        }
    }
    games.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(id: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("rail_files")).unwrap();
        fs::write(
            dir.path().join("rail_files/rail_game_identify.json"),
            format!("{{\"game_id\":{id}}}"),
        )
        .unwrap();
        fs::write(
            dir.path().join("rail_files/rail_game_build_info.dat"),
            r#"{"repo_build_id":"222817","version_name":"V9.1.0"}"#,
        )
        .unwrap();
        fs::write(dir.path().join("game.exe"), b"payload").unwrap();
        dir
    }

    fn entry(root: &std::path::Path) -> Entry {
        Entry {
            name: "Game".into(),
            roots: vec![root.into()],
            ..Entry::default()
        }
    }

    #[test]
    fn installed_metadata_maps_to_existing_game_contract() {
        let dir = fixture("2001481");
        let games = scan(vec![entry(dir.path())]);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "2001481");
        assert_eq!(games[0]._type, "wegame");
        assert!(games[0].state.installed);
        assert!(games[0].commands.launch.is_none());
        assert!(!games[0].state.downloading);
        assert_eq!(games[0].path.as_deref(), Some(dir.path()));
    }

    #[test]
    fn accepts_bom_unknown_fields_and_large_ids_without_precision_loss() {
        let dir = fixture("18446744073709551615");
        fs::write(
            dir.path().join("rail_files/rail_game_identify.json"),
            b"\xef\xbb\xbf{\"game_id\":18446744073709551615,\"future_field\":true}",
        )
        .unwrap();
        let games = scan(vec![entry(dir.path())]);
        assert_eq!(games[0].id, "18446744073709551615");
    }

    #[test]
    fn rejects_invalid_identity_types_and_empty_builds() {
        let dir = fixture("2001481");
        let identity = dir.path().join("rail_files/rail_game_identify.json");
        for json in [
            r#"{"game_id":0}"#,
            r#"{"game_id":"2001481"}"#,
            r#"{"game_id":-1}"#,
        ] {
            fs::write(&identity, json).unwrap();
            assert!(scan(vec![entry(dir.path())]).is_empty());
        }
        fs::write(&identity, r#"{"game_id":2001481}"#).unwrap();
        fs::write(
            dir.path().join("rail_files/rail_game_build_info.dat"),
            r#"{"repo_build_id":" "}"#,
        )
        .unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
    }

    #[test]
    fn no_registry_entry_means_no_discovery_and_cache_is_not_payload() {
        let dir = fixture("2001481");
        assert!(scan(Vec::new()).is_empty());
        fs::remove_file(dir.path().join("game.exe")).unwrap();
        fs::create_dir(dir.path().join("tiny_cache")).unwrap();
        fs::write(dir.path().join("tiny_cache/game.exe"), b"cache").unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
        fs::create_dir(dir.path().join("content")).unwrap();
        fs::write(dir.path().join("content/data.bin"), b"game content").unwrap();
        assert_eq!(scan(vec![entry(dir.path())]).len(), 1);
    }

    #[test]
    fn duplicate_id_selection_is_stable_and_user_name_has_priority() {
        let first = fixture("2001481");
        let second = fixture("2001481");
        let selected = scan(vec![entry(first.path()), entry(second.path())]);
        let reversed = scan(vec![entry(second.path()), entry(first.path())]);
        assert_eq!(selected[0].path, reversed[0].path);
        let mut machine = entry(first.path());
        machine.priority = 1;
        machine.name = "Machine name".into();
        let mut user = entry(first.path());
        user.name = "User name".into();
        assert_eq!(scan(vec![machine, user])[0].name, "User name");
    }

    #[test]
    fn rejects_download_marker_missing_build_and_metadata_only() {
        let dir = fixture("2001481");
        let marker = dir.path().join("rail_files/download_cache.dat");
        fs::write(&marker, b"").unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
        fs::remove_file(marker).unwrap();
        fs::remove_file(dir.path().join("game.exe")).unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
        fs::write(dir.path().join("game.exe"), b"payload").unwrap();
        fs::remove_file(dir.path().join("rail_files/rail_game_build_info.dat")).unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
    }

    #[test]
    fn bad_candidate_does_not_hide_valid_game_and_duplicates_are_removed() {
        let good = fixture("2001481");
        let bad = fixture("2001918");
        fs::write(
            bad.path().join("rail_files/rail_game_identify.json"),
            b"invalid",
        )
        .unwrap();
        let games = scan(vec![
            entry(bad.path()),
            entry(good.path()),
            entry(good.path()),
        ]);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "2001481");
    }

    #[test]
    fn rejects_uninstaller_only_and_oversized_metadata() {
        let dir = fixture("2001481");
        let mut candidate = entry(dir.path());
        candidate.uninstaller = Some(dir.path().join("game.exe"));
        assert!(scan(vec![candidate]).is_empty());
        fs::write(
            dir.path().join("rail_files/rail_game_identify.json"),
            vec![b' '; 65_537],
        )
        .unwrap();
        assert!(scan(vec![entry(dir.path())]).is_empty());
    }

    #[test]
    fn rejects_invalid_ids_before_scanning() {
        for id in ["0", "01", "-1", "abc", ""] {
            assert!(find(id).is_err());
        }
    }
}
