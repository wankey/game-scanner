use crate::prelude::Game;
use sysinfo::System;

#[cfg(target_os = "windows")]
pub fn pids(game: &Game) -> Vec<u32> {
    let install_path = match game.path.as_ref() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let sys = System::new_all();
    sys.processes()
        .iter()
        .filter(|(_, p)| {
            p.exe().is_some_and(|e| e.starts_with(install_path))
                || p.cwd().is_some_and(|c| c.starts_with(install_path))
        })
        .map(|(pid, _)| pid.as_u32())
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn pids(_game: &Game) -> Vec<u32> {
    Vec::new()
}
