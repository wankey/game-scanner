use crate::{
    error::{Error, ErrorKind, Result},
    prelude::Game,
    steam::types::{SteamAppState, SteamUpdateResult},
    utils::string::remove_quotes,
};
use std::path::{Path, PathBuf};

pub fn read(file: &Path, launcher_executable: &Path, library_path: &Path) -> Result<Game> {
    let manifest_data = std::fs::read_to_string(&file).map_err(|error| {
        Error::new(
            ErrorKind::InvalidManifest,
            format!(
                "Error on read the Steam manifest: {} {}",
                file.display().to_string(),
                error.to_string()
            ),
        )
    })?;

    parse(&manifest_data, launcher_executable, library_path)
}

/// Parses manifest text. `lines()` strips both `\n` and `\r\n`, so manifests
/// saved with Windows line endings do not leak `\r` into values.
fn parse(manifest_data: &str, launcher_executable: &Path, library_path: &Path) -> Result<Game> {
    let mut game = Game::default();
    game._type = String::from("steam");

    for file_line in manifest_data.lines() {
        let line = file_line
            .split("\t")
            .filter(|str| !str.trim().is_empty())
            .collect::<Vec<&str>>();

        if line.len() != 2 {
            continue;
        }

        let attr = remove_quotes(line[0].trim());
        let value = remove_quotes(line[1].trim());

        match attr.as_str() {
            "appid" => game.id = value,
            "name" => game.name = value,
            "StateFlags" => {
                // A malformed value leaves the default (not installed) state.
                let Ok(state) = value.parse::<i64>() else { continue };

                game.state.installed = has_app_state(state, SteamAppState::FullyInstalled);

                game.state.needs_update = has_app_state(state, SteamAppState::UpdateRequired);

                game.state.downloading = has_app_state(state, SteamAppState::Downloading);
            }
            "UpdateResult" => {
                let Ok(state) = value.parse::<i64>() else { continue };

                if game.state.needs_update && !game.state.downloading {
                    game.state.downloading =
                        has_update_result(state, SteamUpdateResult::Downloading);
                }
            }
            "BytesToDownload" => game.state.total_bytes = value.parse::<u64>().ok(),
            "BytesDownloaded" => game.state.received_bytes = value.parse::<u64>().ok(),
            "installdir" => {
                game.path = Some(PathBuf::from(library_path).join("common").join(value))
            }
            _ => {}
        }
    }

    if game.id == "228980" {
        return Err(Error::new(
            ErrorKind::IgnoredApp,
            format!("({}) {} is an invalid game", &game.id, &game.name),
        ));
    }

    game.commands.install = Some(vec![
        launcher_executable.display().to_string(),
        String::from("-silent"),
        format!("steam://install/{}", &game.id),
    ]);

    game.commands.launch = Some(vec![
        launcher_executable.display().to_string(),
        String::from("-silent"),
        format!("steam://run/{}", &game.id),
    ]);

    game.commands.uninstall = Some(vec![
        launcher_executable.display().to_string(),
        String::from("-silent"),
        format!("steam://uninstall/{}", &game.id),
    ]);

    return Ok(game);
}

fn has_app_state(state: i64, flag: SteamAppState) -> bool {
    if flag == SteamAppState::Invalid {
        return state == 0;
    }

    if flag == SteamAppState::Uninstalled {
        return state == 1;
    }

    (state & flag.get_code()) == flag.get_code()
}

fn has_update_result(state: i64, flag: SteamUpdateResult) -> bool {
    if flag == SteamUpdateResult::Downloading {
        return state == 0;
    }

    (state & flag.get_code()) == flag.get_code()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = concat!(
        "\"AppState\"\n{\n",
        "\t\"appid\"\t\t\"489830\"\n",
        "\t\"name\"\t\t\"The Elder Scrolls V: Skyrim Special Edition\"\n",
        "\t\"StateFlags\"\t\t\"4\"\n",
        "\t\"installdir\"\t\t\"Skyrim Special Edition\"\n",
        "\t\"UpdateResult\"\t\t\"0\"\n",
        "}\n",
    );

    fn parse_text(text: &str) -> Game {
        parse(text, Path::new("steam.exe"), Path::new("D:/lib/steamapps")).unwrap()
    }

    #[test]
    fn crlf_manifest_parses_like_lf() {
        let game = parse_text(&MANIFEST.replace('\n', "\r\n"));
        assert_eq!(game.id, "489830");
        assert_eq!(game.name, "The Elder Scrolls V: Skyrim Special Edition");
        assert!(game.state.installed);
        assert_eq!(
            game.path,
            Some(PathBuf::from("D:/lib/steamapps").join("common").join("Skyrim Special Edition"))
        );
    }

    #[test]
    fn malformed_state_flags_do_not_panic() {
        let game = parse_text(&MANIFEST.replace("\"4\"", "\"four\""));
        assert_eq!(game.id, "489830");
        assert!(!game.state.installed);
    }
}
