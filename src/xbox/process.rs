use crate::prelude::Game;
use sysinfo::{Pid, System};

#[cfg(target_os = "windows")]
pub fn pids(game: &Game) -> Vec<u32> {
    let install_path = match game.path.as_ref() {
        Some(p) => p.to_string_lossy().to_string(),
        None => return Vec::new(),
    };

    let sys = System::new_all();
    sys.processes()
        .iter()
        .filter(|(_, p)| {
            let exe_contains = p
                .exe()
                .map(|e| e.to_string_lossy().contains(&install_path))
                .unwrap_or(false);
            let cwd_contains = p
                .cwd()
                .map(|c| c.to_string_lossy().contains(&install_path))
                .unwrap_or(false);
            exe_contains || cwd_contains
        })
        .map(|(pid, _)| pid.as_u32())
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn pids(_game: &Game) -> Vec<u32> {
    Vec::new()
}
