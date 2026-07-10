//! C ABI bindings for the `game-scanner` crate.
//!
//! All functions return an `i32` status code:
//!   `0` = success, output written to `*out` as a heap-allocated JSON string.
//!   `2` = backend error (e.g. launcher not installed), error message in JSON.
//!   `1` = FFI / serialization error (malformed JSON input or memory failure).
//!
//! Every string handed back to C must be released with [`gs_free`].
//!
//! The `launcher` argument is the snake-case name used by
//! `game_scanner::prelude::GameType::to_string`:
//! `amazongames`, `blizzard`, `epicgames`, `gog`, `origin`,
//! `riotgames`, `steam`, `ubisoft`, `xbox`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use game_scanner::{
    amazon, blizzard, epicgames, gog, origin,
    prelude::{Game, GameType},
    riotgames, steam, ubisoft, xbox,
};

/// Releases a string previously returned by this library.
#[no_mangle]
pub extern "C" fn gs_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Parses a snake-case launcher name. Returns `None` for unknown names.
fn parse_launcher(name: &str) -> Option<GameType> {
    Some(match name {
        "amazongames" => GameType::AmazonGames,
        "blizzard" => GameType::Blizzard,
        "epicgames" => GameType::EpicGames,
        "gog" => GameType::GOG,
        "origin" => GameType::Origin,
        "riotgames" => GameType::RiotGames,
        "steam" => GameType::Steam,
        "ubisoft" => GameType::Ubisoft,
        "xbox" => GameType::XboxGames,
        _ => return None,
    })
}

/// Lists every game the launcher can see. Writes a JSON array of `Game`
/// (or `{"error": "..."}` on backend failure) into `*out`.
#[no_mangle]
pub extern "C" fn gs_list(launcher: *const c_char, out: *mut *mut c_char) -> i32 {
    let Some(name) = read_str(launcher) else { return bad_input(out, "launcher is null or invalid UTF-8"); };
    let Some(l) = parse_launcher(&name) else { return bad_input(out, &format!("unknown launcher: {name}")); };

    let raw: Result<Vec<Game>, _> = match l {
        GameType::AmazonGames => amazon::games(),
        GameType::Blizzard => blizzard::games(),
        GameType::EpicGames => epicgames::games(),
        GameType::GOG => gog::games(),
        GameType::Origin => origin::games(),
        GameType::RiotGames => riotgames::games(),
        GameType::Steam => steam::games(),
        GameType::Ubisoft => ubisoft::games(),
        GameType::XboxGames => xbox::games(),
    };

    match raw {
        Ok(games) => write_json(out, &games),
        Err(e) => write_err(out, &e.to_string()),
    }
}

/// Looks up a single game by launcher-specific id (e.g. a Steam app id).
#[no_mangle]
pub extern "C" fn gs_find(
    launcher: *const c_char,
    id: *const c_char,
    out: *mut *mut c_char,
) -> i32 {
    let Some(name) = read_str(launcher) else { return bad_input(out, "launcher is null or invalid UTF-8"); };
    let Some(l) = parse_launcher(&name) else { return bad_input(out, &format!("unknown launcher: {name}")); };
    let Some(id) = read_str(id) else { return bad_input(out, "id is null or invalid UTF-8"); };

    let raw: Result<Game, _> = match l {
        GameType::AmazonGames => amazon::find(&id),
        GameType::Blizzard => blizzard::find(&id),
        GameType::EpicGames => epicgames::find(&id),
        GameType::GOG => gog::find(&id),
        GameType::Origin => origin::find(&id),
        GameType::RiotGames => riotgames::find(&id),
        GameType::Steam => steam::find(&id),
        GameType::Ubisoft => ubisoft::find(&id),
        GameType::XboxGames => xbox::find(&id),
    };

    match raw {
        Ok(g) => write_json(out, &g),
        Err(e) => write_err(out, &e.to_string()),
    }
}

/// Returns the path to the launcher's executable (or `{"error": "..."}`).
#[no_mangle]
pub extern "C" fn gs_executable(launcher: *const c_char, out: *mut *mut c_char) -> i32 {
    let Some(name) = read_str(launcher) else { return bad_input(out, "launcher is null or invalid UTF-8"); };
    let Some(l) = parse_launcher(&name) else { return bad_input(out, &format!("unknown launcher: {name}")); };

    let raw: Result<std::path::PathBuf, _> = match l {
        GameType::AmazonGames => amazon::executable(),
        GameType::Blizzard => blizzard::executable(),
        GameType::EpicGames => epicgames::executable(),
        GameType::GOG => gog::executable(),
        GameType::Origin => origin::executable(),
        GameType::RiotGames => riotgames::executable(),
        GameType::Steam => steam::executable(),
        GameType::Ubisoft => ubisoft::executable(),
        GameType::XboxGames => xbox::executable(),
    };

    match raw {
        Ok(p) => write_json(out, &p.to_string_lossy().into_owned()),
        Err(e) => write_err(out, &e.to_string()),
    }
}

/// Installs the game described by the supplied JSON `Game`.
#[no_mangle]
pub extern "C" fn gs_install(game_json: *const c_char, out: *mut *mut c_char) -> i32 {
    run_manager_op("install_game", game_json, out, |g| {
        let raw: Result<(), _> = game_scanner::manager::install_game(g);
        raw.map_err(|e| e.to_string()).map(|()| serde_json::json!(null))
    })
}

/// Uninstalls the game described by the supplied JSON `Game`.
#[no_mangle]
pub extern "C" fn gs_uninstall(game_json: *const c_char, out: *mut *mut c_char) -> i32 {
    run_manager_op("uninstall_game", game_json, out, |g| {
        let raw: Result<(), _> = game_scanner::manager::uninstall_game(g);
        raw.map_err(|e| e.to_string()).map(|()| serde_json::json!(null))
    })
}

/// Launches the game described by the supplied JSON `Game`.
#[no_mangle]
pub extern "C" fn gs_launch(game_json: *const c_char, out: *mut *mut c_char) -> i32 {
    run_manager_op("launch_game", game_json, out, |g| {
        let raw: Result<(), _> = game_scanner::manager::launch_game(g);
        raw.map_err(|e| e.to_string()).map(|()| serde_json::json!(null))
    })
}

/// Closes the running game described by the supplied JSON `Game`.
#[no_mangle]
pub extern "C" fn gs_close(game_json: *const c_char, out: *mut *mut c_char) -> i32 {
    run_manager_op("close_game", game_json, out, |g| {
        let raw: Result<(), _> = game_scanner::manager::close_game(g);
        raw.map_err(|e| e.to_string()).map(|()| serde_json::json!(null))
    })
}

/// Returns the PIDs currently running the supplied game, or `null` if
/// the launcher cannot introspect its processes (Steam can, others cannot).
#[no_mangle]
pub extern "C" fn gs_get_processes(game_json: *const c_char, out: *mut *mut c_char) -> i32 {
    run_manager_op("get_processes", game_json, out, |g| {
        Ok(serde_json::to_value(game_scanner::manager::get_processes(g))
            .unwrap_or(serde_json::Value::Null))
    })
}

// ──────────────────────────── helpers ────────────────────────────

fn read_str(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok().map(|s| s.to_string())
}

fn parse_game(json: &str) -> Result<Game, String> {
    serde_json::from_str::<Game>(json).map_err(|e| e.to_string())
}

/// Run any manager op by deserializing JSON, applying `f`, and writing
/// either JSON `null` or `{"error": "..."}` into `*out`.
fn run_manager_op<F>(
    _name: &'static str,
    game_json: *const c_char,
    out: *mut *mut c_char,
    f: F,
) -> i32
where
    F: FnOnce(&Game) -> Result<serde_json::Value, String>,
{
    let Some(raw) = read_str(game_json) else {
        return bad_input(out, "game_json is null or invalid UTF-8");
    };
    let game = match parse_game(&raw) {
        Ok(g) => g,
        Err(e) => return bad_input(out, &format!("invalid game JSON: {e}")),
    };
    match f(&game) {
        Ok(v) => write_json(out, &v),
        Err(e) => write_err(out, &e),
    }
}

fn write_json<T: serde::Serialize>(out: *mut *mut c_char, v: &T) -> i32 {
    match serde_json::to_string(v) {
        Ok(s) => {
            match CString::new(s) {
                Ok(cs) => {
                    unsafe { *out = CString::into_raw(cs); }
                    0
                }
                Err(_) => 1,
            }
        }
        Err(_) => 1,
    }
}

fn write_err(out: *mut *mut c_char, msg: &str) -> i32 {
    let body = serde_json::json!({ "error": msg }).to_string();
    if let Ok(cs) = CString::new(body) {
        unsafe { *out = CString::into_raw(cs); }
        2
    } else {
        1
    }
}

fn bad_input(out: *mut *mut c_char, msg: &str) -> i32 {
    write_err(out, msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_launcher_round_trip() {
        // The GameType names map to themselves via to_string/from.
        for name in [
            "amazongames", "blizzard", "epicgames", "gog",
            "origin", "riotgames", "steam", "ubisoft", "xbox",
        ] {
            let l = parse_launcher(name).unwrap();
            assert_eq!(l.to_string(), name);
        }
    }

    #[test]
    fn parse_launcher_unknown_returns_none() {
        assert!(parse_launcher("nope").is_none());
        assert!(parse_launcher("Steam").is_none()); // case-sensitive
    }

    #[test]
    fn parse_game_valid_json() {
        let json = r#"{
            "_type": "Steam",
            "id": "945360",
            "name": "Example",
            "path": null,
            "commands": { "install": null, "launch": null, "uninstall": null },
            "state": {
                "installed": false,
                "needs_update": false,
                "downloading": false,
                "total_bytes": null,
                "received_bytes": null
            }
        }"#;
        let g = parse_game(json).expect("valid Game JSON");
        assert_eq!(g.id, "945360");
        assert_eq!(g.name, "Example");
    }

    #[test]
    fn parse_game_invalid_json_returns_err() {
        assert!(parse_game("not json").is_err());
    }

    #[test]
    fn write_json_then_free_roundtrip() {
        let mut p: *mut c_char = std::ptr::null_mut();
        let rc = write_json(&mut p as &mut *mut c_char, &"hello");
        assert_eq!(rc, 0);
        assert!(!p.is_null());
        let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap();
        assert_eq!(s, "\"hello\"");
        gs_free(p);
    }

    #[test]
    fn gs_free_null_is_safe() {
        gs_free(std::ptr::null_mut());
    }
}
