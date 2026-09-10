#[cfg(not(target_os = "windows"))]
use crate::error::{Error, ErrorKind, Result};
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use self::linux::*;
#[cfg(target_os = "macos")]
pub use self::macos::*;
#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(not(target_os = "windows"))]
pub fn get_launcher_executable() -> Result<PathBuf> {
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "Xbox / Microsoft Store is not supported on this platform",
    ))
}
