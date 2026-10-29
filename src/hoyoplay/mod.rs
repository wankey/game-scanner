//! Registered Windows CN installations. Only nap_cn/hkrpg_cn are currently recognized.
//! Download state comes from a validated current MMKV snapshot; update checks are unsupported.
#[cfg(any(windows, test))]
mod catalog;
#[cfg(any(windows, test))]
mod download;
#[cfg(any(windows, test))]
mod manifest;
#[cfg(windows)]
mod windows;

use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
};
use std::path::PathBuf;

pub fn executable() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        windows::executable()
    }
    #[cfg(not(windows))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "HoYoPlay detection is only supported on Windows",
        ))
    }
}

/// Returns registered installed games and current initial downloads.
/// This is not a full integrity or playability check. See docs/hoyoplay.md for coverage.
pub fn games() -> Result<Vec<Game>> {
    #[cfg(windows)]
    {
        let entries = windows::entries()?;
        let downloads = windows::downloads()?;
        Ok(scan(entries, windows::still_registered, &downloads))
    }
    #[cfg(not(windows))]
    {
        Err(Error::new(
            ErrorKind::LauncherNotFound,
            "HoYoPlay detection is only supported on Windows",
        ))
    }
}

pub fn find(id: &str) -> Result<Game> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(Error::new(
            ErrorKind::InvalidGame,
            "HoYoPlay ID must be a lowercase game biz, e.g. nap_cn",
        ));
    }
    games()?
        .into_iter()
        .find(|game| game.id == id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::GameNotFound,
                format!("HoYoPlay game {id} not found"),
            )
        })
}

#[cfg(any(windows, test))]
#[derive(Clone)]
struct Entry {
    biz: String,
    root: PathBuf,
}

#[cfg(any(windows, test))]
fn scan(
    mut entries: Vec<Entry>,
    still_registered: impl Fn(&Entry) -> bool,
    downloads: &std::collections::HashMap<String, download::Download>,
) -> Vec<Game> {
    use std::collections::BTreeMap;
    entries.sort_by(|a, b| (&a.biz, &a.root).cmp(&(&b.biz, &b.root)));
    let mut games = BTreeMap::new();
    for entry in entries {
        let Some(config) = catalog::find(&entry.biz) else {
            continue;
        };
        let downloading = downloads
            .get(&entry.biz)
            .is_some_and(|d| d.matches(&entry.biz, &entry.root));
        let installed = match manifest::installed(&entry.root, config.exe) {
            Ok(installed) => installed,
            Err(error) => {
                use std::error::Error as _;
                let missing = error
                    .source()
                    .and_then(|e| e.downcast_ref::<std::io::Error>())
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound);
                if !missing {
                    crate::error::print_error(&error);
                    continue;
                }
                false
            }
        };
        if !(installed || downloading) || !still_registered(&entry) {
            continue;
        }
        if let Some(previous) = games.get(config.biz) {
            let previous: &Game = previous;
            if previous.path.as_ref().and_then(|p| p.canonicalize().ok())
                != entry.root.canonicalize().ok()
            {
                crate::error::print_error(&Error::new(
                    ErrorKind::InvalidGame,
                    format!("Multiple HoYoPlay installations for {}", config.biz),
                ));
            }
            continue;
        }
        let mut game = Game {
            _type: "hoyoplay".into(),
            id: config.biz.into(),
            name: config.name.into(),
            path: Some(entry.root),
            ..Game::default()
        };
        game.state.installed = installed;
        game.state.downloading = downloading;
        games.insert(config.biz.to_owned(), game);
    }
    games.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(biz: &str, ini: &str, exe: bool) -> (tempfile::TempDir, Entry) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("游戏 with spaces");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("config.ini"), ini).unwrap();
        if exe {
            fs::write(
                root.join(catalog::find(biz).unwrap().exe),
                b"fixture payload",
            )
            .unwrap();
        }
        (
            dir,
            Entry {
                biz: biz.into(),
                root,
            },
        )
    }

    #[test]
    fn real_download_and_completed_snapshots_use_same_scan_path() {
        let (_a, a) = fixture(
            "hkrpg_cn",
            include_str!("fixtures/starrail-downloading.ini"),
            false,
        );
        let (_b, b) = fixture("nap_cn", include_str!("fixtures/zzz-installed.ini"), true);
        let games = scan(vec![a, b.clone(), b], |_| true, &Default::default());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "nap_cn");
        assert_eq!(games[0]._type, "hoyoplay");
        assert!(games[0].state.installed);
        assert!(games[0].commands.launch.is_none());
        assert!(!games[0].state.downloading);
    }

    #[test]
    fn version_executable_and_channel_are_all_required() {
        for ini in [
            "[General]\ngame_version=\nchannel=1\nsub_channel=1",
            "[General]\ngame_version=3.3.0\nchannel=2\nsub_channel=1",
            "[General]\ngame_version=3.3.0\nchannel=1",
        ] {
            let (_dir, entry) = fixture("nap_cn", ini, true);
            assert!(scan(vec![entry], |_| true, &Default::default()).is_empty());
        }
        let (_dir, entry) = fixture(
            "nap_cn",
            "[General]\ngame_version=3.3.0\nchannel=1\nsub_channel=1",
            false,
        );
        assert!(scan(vec![entry], |_| true, &Default::default()).is_empty());
    }

    #[test]
    fn malformed_candidate_does_not_hide_valid_installation() {
        let (_a, a) = fixture(
            "nap_cn",
            "[General]\ngame_version=1\ngame_version=2\nchannel=1\nsub_channel=1",
            true,
        );
        let (_b, b) = fixture(
            "hkrpg_cn",
            "\u{feff}[General]\r\ngame_version=4.6.0\r\nchannel=1\r\nsub_channel=1\r\nfuture=true",
            true,
        );
        let games = scan(vec![a, b], |_| true, &Default::default());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "hkrpg_cn");
    }

    #[test]
    fn changing_registration_and_unknown_games_are_not_returned() {
        let (_dir, mut entry) = fixture(
            "nap_cn",
            "[General]\ngame_version=3.3.0\nchannel=1\nsub_channel=1",
            true,
        );
        assert!(scan(vec![entry.clone()], |_| false, &Default::default()).is_empty());
        entry.biz = "unknown_cn".into();
        assert!(scan(vec![entry], |_| true, &Default::default()).is_empty());
    }

    #[test]
    fn invalid_ids_are_rejected_before_registry_access() {
        for id in ["", "NAP_CN", "../nap_cn", "nap cn", "原神"] {
            assert_eq!(find(id).unwrap_err().kind(), ErrorKind::InvalidGame);
        }
    }

    #[test]
    fn initial_download_and_installed_download_have_independent_flags() {
        let (_a, a) = fixture(
            "hkrpg_cn",
            include_str!("fixtures/starrail-downloading.ini"),
            false,
        );
        let (_b, b) = fixture("nap_cn", include_str!("fixtures/zzz-installed.ini"), true);
        let mut downloads = std::collections::HashMap::new();
        for entry in [&a, &b] {
            downloads.insert(
                entry.biz.clone(),
                serde_json::from_value(serde_json::json!({
                    "gameBiz":entry.biz,"installPath":entry.root,"downloading":true
                }))
                .unwrap(),
            );
        }
        let games = scan(vec![a.clone(), b.clone()], |_| true, &downloads);
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].id, "hkrpg_cn");
        assert!(!games[0].state.installed);
        assert!(games[0].state.downloading);
        assert!(games[1].state.installed);
        assert!(games[1].state.downloading);
        // Clearing a task must not remove an already committed installation.
        let games = scan(vec![a, b], |_| true, &Default::default());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "nap_cn");
        assert!(games[0].state.installed);
        assert!(!games[0].state.downloading);
    }

    #[test]
    fn rejects_oversized_config_empty_payload_and_directory_payload() {
        let (_dir, entry) = fixture("nap_cn", &"x".repeat(65_537), true);
        assert!(scan(vec![entry.clone()], |_| true, &Default::default()).is_empty());
        fs::write(
            entry.root.join("config.ini"),
            "[General]\ngame_version=3.3.0\nchannel=1\nsub_channel=1",
        )
        .unwrap();
        let exe = entry.root.join("ZenlessZoneZero.exe");
        fs::write(&exe, b"").unwrap();
        assert!(scan(vec![entry.clone()], |_| true, &Default::default()).is_empty());
        fs::remove_file(&exe).unwrap();
        fs::create_dir(&exe).unwrap();
        assert!(scan(vec![entry], |_| true, &Default::default()).is_empty());
    }

    #[test]
    fn rejects_configured_path_traversal() {
        let (_dir, entry) = fixture("nap_cn", "", false);
        assert!(manifest::installed(&entry.root, "../other.exe").is_err());
        assert!(manifest::installed(&entry.root, "").is_err());
    }
}
