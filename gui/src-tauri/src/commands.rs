use crate::capability::{matrix as capability_matrix, Op};
use game_scanner::{
    amazon, blizzard, epicgames, gog, origin, prelude::{Game, GameType}, riotgames, steam, ubisoft,
};
use std::{collections::HashMap, path::PathBuf};

/// Dispatch a launcher to its `games()` function. Returns the same error
/// variant from the underlying crate on unsupported OSes (e.g. Amazon on
/// macOS). The error type is private, so we box and stringify it via
/// the `Display` impl that `game_scanner::error::Error` provides.
pub fn list_games(launcher: GameType) -> Result<Vec<Game>, String> {
    let raw: Result<Vec<Game>, _> = match launcher {
        GameType::AmazonGames => amazon::games(),
        GameType::Blizzard => blizzard::games(),
        GameType::EpicGames => epicgames::games(),
        GameType::GOG => gog::games(),
        GameType::Origin => origin::games(),
        GameType::RiotGames => riotgames::games(),
        GameType::Steam => steam::games(),
        GameType::Ubisoft => ubisoft::games(),
    };
    raw.map_err(|e| e.to_string())
}

pub fn find_game(launcher: GameType, id: &str) -> Result<Game, String> {
    let raw: Result<Game, _> = match launcher {
        GameType::AmazonGames => amazon::find(id),
        GameType::Blizzard => blizzard::find(id),
        GameType::EpicGames => epicgames::find(id),
        GameType::GOG => gog::find(id),
        GameType::Origin => origin::find(id),
        GameType::RiotGames => riotgames::find(id),
        GameType::Steam => steam::find(id),
        GameType::Ubisoft => ubisoft::find(id),
    };
    raw.map_err(|e| e.to_string())
}

pub fn launcher_executable(launcher: GameType) -> Result<PathBuf, String> {
    let raw: Result<PathBuf, _> = match launcher {
        GameType::AmazonGames => amazon::executable(),
        GameType::Blizzard => blizzard::executable(),
        GameType::EpicGames => epicgames::executable(),
        GameType::GOG => gog::executable(),
        GameType::Origin => origin::executable(),
        GameType::RiotGames => riotgames::executable(),
        GameType::Steam => steam::executable(),
        GameType::Ubisoft => ubisoft::executable(),
    };
    raw.map_err(|e| e.to_string())
}

pub fn get_capabilities() -> HashMap<String, Vec<Op>> {
    capability_matrix()
}

/// Parse a snake_case launcher name into a `GameType`. The frontend uses
/// strings because `GameType` does not implement `Deserialize` directly.
pub fn parse_launcher(name: &str) -> GameType {
    GameType::from(name.to_string())
}
