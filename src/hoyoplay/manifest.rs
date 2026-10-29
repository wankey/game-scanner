use crate::error::{Error, ErrorKind, Result};
use std::{collections::HashMap, fs, io::Read, path::Path};

const LIMIT: u64 = 65_536;

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::InvalidManifest, message)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(invalid("HoYoPlay config exceeds 64 KiB"));
    }
    Ok(bytes)
}

fn committed(bytes: &[u8]) -> Result<bool> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("Invalid config encoding"))?;
    let mut general = false;
    let mut seen_general = false;
    let mut values = HashMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            general = line == "[General]";
            if general && seen_general {
                return Err(invalid("Duplicate General section"));
            }
            seen_general |= general;
            continue;
        }
        if !general {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| invalid("Malformed General entry"))?;
        let key = key.trim();
        if matches!(key, "game_version" | "channel" | "sub_channel")
            && values.insert(key, value.trim()).is_some()
        {
            return Err(invalid("Duplicate installation field"));
        }
    }
    let version = values.get("game_version").copied().unwrap_or_default();
    if version.len() > 128 || version.chars().any(char::is_control) {
        return Err(invalid("Invalid local version"));
    }
    if values.get("channel") != Some(&"1") || values.get("sub_channel") != Some(&"1") {
        return Err(invalid("Unsupported HoYoPlay installation channel"));
    }
    Ok(!version.is_empty())
}

/// Checks local committed metadata and the official configured file, not file integrity.
pub(super) fn installed(root: &Path, exe: &str) -> Result<bool> {
    let relative = Path::new(exe);
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || relative.as_os_str().is_empty()
    {
        return Err(invalid("Invalid configured executable path"));
    }
    let config = root.join("config.ini");
    for _ in 0..2 {
        let before = read(&config)?;
        if !committed(&before)? {
            return Ok(false);
        }
        let canonical_root = root.canonicalize()?;
        let file = root.join(relative).canonicalize()?;
        if !file.starts_with(&canonical_root) {
            return Err(invalid("Executable escapes game directory"));
        }
        let metadata = fs::metadata(&file)?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Ok(false);
        }
        let after = read(&config)?;
        let latest = fs::metadata(&file)?;
        if before == after
            && latest.len() == metadata.len()
            && latest.modified()? == metadata.modified()?
        {
            return Ok(true);
        }
    }
    Err(invalid("HoYoPlay installation changed during scanning"))
}
