mod capability;
mod commands;

use std::{collections::HashMap, path::PathBuf};
use game_scanner::prelude::Game;

/// `launcher` is a snake_case string ("steam", "epicgames", etc.). We
/// parse it to `GameType` here so the IPC boundary stays JSON-friendly,
/// because `game_scanner::prelude::GameType` does not implement
/// `Deserialize` and the `error` module is private.
#[tauri::command]
fn list_games(launcher: String) -> Result<Vec<Game>, String> {
    let parsed = commands::parse_launcher(&launcher);
    commands::list_games(parsed)
}

#[tauri::command]
fn find_game(launcher: String, id: String) -> Result<Game, String> {
    let parsed = commands::parse_launcher(&launcher);
    commands::find_game(parsed, &id)
}

#[tauri::command]
fn launcher_executable(launcher: String) -> Result<PathBuf, String> {
    let parsed = commands::parse_launcher(&launcher);
    commands::launcher_executable(parsed)
}

#[tauri::command]
fn get_capabilities() -> HashMap<String, Vec<capability::Op>> {
    commands::get_capabilities()
}

#[tauri::command]
fn install_game(game: Game) -> Result<(), String> {
    commands::install_game(&game)
}

#[tauri::command]
fn uninstall_game(game: Game) -> Result<(), String> {
    commands::uninstall_game(&game)
}

#[tauri::command]
fn launch_game(game: Game) -> Result<(), String> {
    commands::launch_game(&game)
}

#[tauri::command]
fn close_game(game: Game) -> Result<(), String> {
    commands::close_game(&game)
}

#[tauri::command]
fn get_processes(game: Game) -> Option<Vec<u32>> {
    commands::get_processes(&game)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_games,
            find_game,
            launcher_executable,
            get_capabilities,
            install_game,
            uninstall_game,
            launch_game,
            close_game,
            get_processes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
