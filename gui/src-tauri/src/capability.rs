use game_scanner::prelude::GameType;
use serde::Serialize;
use std::collections::HashMap;

/// Operation types that the demo UI exposes per launcher.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    List,
    Find,
    Executable,
    Install,
    Launch,
    Uninstall,
    Processes,
    Close,
}

/// Returns true if `launcher` supports `op` per the README capability matrix.
///
/// Single source of truth — both `commands::get_capabilities` and the test
/// suite consume this table.
pub fn supports(launcher: GameType, op: Op) -> bool {
    use GameType::*;
    use Op::*;
    matches!(
        (launcher, op),
        // Every launcher supports list / find / executable / launch.
        (_, List) | (_, Find) | (_, Executable) | (_, Launch)
        // Install: Origin, Steam, Ubisoft.
        | (Origin, Install) | (Steam, Install) | (Ubisoft, Install)
        // Uninstall: Riot, Steam, Ubisoft.
        | (RiotGames, Uninstall) | (Steam, Uninstall) | (Ubisoft, Uninstall)
        // Get Processes & Close: Steam only.
        | (Steam, Processes) | (Steam, Close)
    )
}

/// Full matrix as a `HashMap<String, Vec<Op>>` (launcher name → supported
/// ops), ready for JSON serialization to the frontend. We key on the
/// string form because `game_scanner::prelude::GameType` does not derive
/// `Serialize`.
pub fn matrix() -> HashMap<String, Vec<Op>> {
    use GameType::*;
    let all_ops = [Op::List, Op::Find, Op::Executable, Op::Install, Op::Launch, Op::Uninstall, Op::Processes, Op::Close];
    let mut out = HashMap::new();
    for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
        let ops = all_ops.iter().copied().filter(|op| supports(launcher, *op)).collect();
        out.insert(launcher.to_string(), ops);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use GameType::*;

    // --- Every launcher: list / find / executable / launch ---
    #[test]
    fn every_launcher_supports_list() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
            assert!(supports(launcher, Op::List), "{:?} should support List", launcher);
        }
    }

    #[test]
    fn every_launcher_supports_find() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
            assert!(supports(launcher, Op::Find), "{:?} should support Find", launcher);
        }
    }

    #[test]
    fn every_launcher_supports_executable() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
            assert!(supports(launcher, Op::Executable), "{:?} should support Executable", launcher);
        }
    }

    #[test]
    fn every_launcher_supports_launch() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
            assert!(supports(launcher, Op::Launch), "{:?} should support Launch", launcher);
        }
    }

    // --- Install: Origin, Steam, Ubisoft only ---
    #[test]
    fn install_supported_origin_steam_ubisoft() {
        assert!(supports(Origin, Op::Install));
        assert!(supports(Steam, Op::Install));
        assert!(supports(Ubisoft, Op::Install));
    }

    #[test]
    fn install_unsupported_for_others() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, RiotGames] {
            assert!(!supports(launcher, Op::Install), "{:?} should NOT support Install", launcher);
        }
    }

    // --- Uninstall: Riot, Steam, Ubisoft ---
    #[test]
    fn uninstall_supported_riot_steam_ubisoft() {
        assert!(supports(RiotGames, Op::Uninstall));
        assert!(supports(Steam, Op::Uninstall));
        assert!(supports(Ubisoft, Op::Uninstall));
    }

    #[test]
    fn uninstall_unsupported_for_others() {
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin] {
            assert!(!supports(launcher, Op::Uninstall), "{:?} should NOT support Uninstall", launcher);
        }
    }

    // --- Processes / Close: Steam only ---
    #[test]
    fn processes_supported_steam_only() {
        assert!(supports(Steam, Op::Processes));
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Ubisoft] {
            assert!(!supports(launcher, Op::Processes), "{:?} should NOT support Processes", launcher);
        }
    }

    #[test]
    fn close_supported_steam_only() {
        assert!(supports(Steam, Op::Close));
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Ubisoft] {
            assert!(!supports(launcher, Op::Close), "{:?} should NOT support Close", launcher);
        }
    }

    // --- Matrix shape ---
    #[test]
    fn matrix_contains_all_eight_launchers() {
        let m = matrix();
        for launcher in [AmazonGames, Blizzard, EpicGames, GOG, Origin, RiotGames, Steam, Ubisoft] {
            let key = launcher.to_string();
            assert!(m.contains_key(&key), "matrix missing {:?}", launcher);
        }
        assert_eq!(m.len(), 8);
    }

    #[test]
    fn matrix_steam_has_eight_ops() {
        let m = matrix();
        assert_eq!(m.get("steam").unwrap().len(), 8);
    }

    #[test]
    fn matrix_amazon_has_four_ops() {
        // List + Find + Executable + Launch.
        let m = matrix();
        let ops = m.get("amazongames").unwrap();
        assert_eq!(ops.len(), 4);
        assert!(ops.contains(&Op::List));
        assert!(ops.contains(&Op::Find));
        assert!(ops.contains(&Op::Executable));
        assert!(ops.contains(&Op::Launch));
    }
}
