use crate::{
    error::{Error, ErrorKind, Result},
    prelude::{Game, GameCommands, GameState, GameType, MatchIdentity},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Debug)]
struct Manifest {
    #[serde(rename(deserialize = "bIsIncompleteInstall"))]
    is_incomplete_install: bool,
    #[serde(rename(deserialize = "AppName"))]
    app_name: String,
    #[serde(rename(deserialize = "CatalogNamespace"))]
    catalog_namespace: Option<String>,
    #[serde(rename(deserialize = "CatalogItemId"))]
    catalog_item_id: Option<String>,
    #[serde(rename(deserialize = "DisplayName"))]
    display_name: String,
    #[serde(rename(deserialize = "InstallLocation"))]
    install_location: String,
    #[serde(rename(deserialize = "LaunchExecutable"))]
    launch_executable: String,
}

pub fn read(file: &Path, launcher_executable: &Path) -> Result<Game> {
    let manifest_data = fs::read_to_string(&file).map_err(|error| {
        Error::new(
            ErrorKind::InvalidManifest,
            format!(
                "Invalid Epic Games manifest: {} {}",
                file.display().to_string(),
                error.to_string()
            ),
        )
    })?;

    let manifest = serde_json::from_str::<Manifest>(manifest_data.as_str()).map_err(|error| {
        Error::new(
            ErrorKind::InvalidManifest,
            format!(
                "Error on read the Epic Games manifest: {} {}",
                file.display().to_string(),
                error.to_string()
            ),
        )
    })?;

    if manifest.app_name.contains("UE") || manifest.app_name.contains("QuixelBridge") {
        return Err(Error::new(
            ErrorKind::IgnoredApp,
            format!(
                "({}) {} is an invalid game",
                &manifest.app_name, &manifest.display_name
            ),
        ));
    }

    return Ok(Game {
        _type: GameType::EpicGames.to_string(),
        id: manifest.app_name.clone(),
        match_identity: manifest
            .catalog_namespace
            .zip(manifest.catalog_item_id)
            .map(|(catalog_namespace, catalog_item_id)| MatchIdentity::Epic {
                catalog_namespace,
                catalog_item_id,
            }),
        name: manifest.display_name.clone(),
        path: Some(PathBuf::from(manifest.install_location)),
        commands: GameCommands {
            install: None,
            launch: Some(vec![
                launcher_executable.display().to_string(),
                format!(
                    "com.epicgames.launcher://apps/{}?action=launch&silent=true",
                    &manifest.app_name
                ),
            ]),
            uninstall: None,
        },
        state: GameState {
            installed: !manifest.is_incomplete_install,
            needs_update: false,
            downloading: false,
            total_bytes: None,
            received_bytes: None,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, path::Path};

    #[test]
    fn reads_catalog_match_identity() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(
            file,
            r#"{{
                "bIsIncompleteInstall": false,
                "AppName": "app-name",
                "CatalogNamespace": "catalog-namespace",
                "CatalogItemId": "catalog-item-id",
                "DisplayName": "Game",
                "InstallLocation": "C:\\Game",
                "LaunchExecutable": "game.exe"
            }}"#
        )
        .unwrap();

        let game = read(file.path(), Path::new("launcher.exe")).unwrap();

        assert!(matches!(
            game.match_identity,
            Some(MatchIdentity::Epic {
                catalog_namespace,
                catalog_item_id,
            }) if catalog_namespace == "catalog-namespace" && catalog_item_id == "catalog-item-id"
        ));
    }
}
