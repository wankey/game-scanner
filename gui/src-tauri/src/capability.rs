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
