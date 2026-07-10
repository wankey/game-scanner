use crate::error::{Error, ErrorKind, Result};
use std::path::PathBuf;

#[allow(dead_code)] // Task 5 wires this up.
pub fn get_launcher_executable() -> Result<PathBuf> {
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox / Microsoft Store enumeration not yet implemented",
    ))
}