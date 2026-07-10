use crate::error::{Error, ErrorKind, Result};
use std::path::PathBuf;

pub fn get_launcher_executable() -> Result<PathBuf> {
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox / Microsoft Store enumeration not yet implemented",
    ))
}