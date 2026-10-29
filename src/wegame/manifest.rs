use crate::error::{Error, ErrorKind, Result};
use serde::Deserialize;
use std::{fs, io::Read, path::Path};

#[derive(Deserialize)]
struct Identity {
    game_id: u64,
}

#[derive(Deserialize)]
struct Build {
    repo_build_id: String,
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let mut last_error = None;
    for _ in 0..2 {
        let attempt = || -> Result<T> {
            let mut file = fs::File::open(path)?;
            let before = file.metadata()?;
            if before.len() > 65_536 {
                return Err(Error::new(
                    ErrorKind::InvalidManifest,
                    "Rail metadata exceeds 64 KiB",
                ));
            }
            let mut bytes = Vec::new();
            file.by_ref().take(65_537).read_to_end(&mut bytes)?;
            let after = fs::metadata(path)?;
            if bytes.len() > 65_536
                || before.len() != after.len()
                || before.modified()? != after.modified()?
            {
                return Err(Error::new(
                    ErrorKind::InvalidManifest,
                    "Rail metadata changed during reading",
                ));
            }
            let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
            Ok(serde_json::from_slice(bytes)?)
        };
        match attempt() {
            Ok(value) => return Ok(value),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.expect("two read attempts"))
}

/// Checks committed metadata and some payload, not full file integrity.
pub(super) fn installed_id(root: &Path, uninstaller: Option<&Path>) -> Result<Option<String>> {
    let identity_path = root.join("rail_files/rail_game_identify.json");
    if !identity_path.try_exists()? {
        return Ok(None);
    }
    if root.join("rail_files/download_cache.dat").try_exists()? {
        return Ok(None);
    }
    let identity: Identity = read_json(&identity_path)?;
    let build_path = root.join("rail_files/rail_game_build_info.dat");
    if !build_path.try_exists()? {
        return Ok(None);
    }
    let build: Build = read_json(&build_path)?;
    if identity.game_id == 0 || build.repo_build_id.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            "Invalid Rail game ID or build ID",
        ));
    }
    let mut budget = 256;
    if has_payload(root, uninstaller, 0, &mut budget)? {
        Ok(Some(identity.game_id.to_string()))
    } else {
        Ok(None)
    }
}

fn has_payload(
    root: &Path,
    uninstaller: Option<&Path>,
    depth: usize,
    budget: &mut usize,
) -> Result<bool> {
    for item in fs::read_dir(root)? {
        if *budget == 0 {
            return Err(Error::new(
                ErrorKind::InvalidGame,
                "WeGame payload inspection limit reached",
            ));
        }
        *budget -= 1;
        let item = item?;
        let path = item.path();
        let name = item.file_name();
        if name.eq_ignore_ascii_case("rail_files")
            || name.eq_ignore_ascii_case("tiny_cache")
            || uninstaller == Some(path.as_path())
        {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                continue;
            }
        }
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() && metadata.len() > 0 {
            return Ok(true);
        }
        if metadata.is_dir() && depth < 2 && has_payload(&path, uninstaller, depth + 1, budget)? {
            return Ok(true);
        }
    }
    Ok(false)
}
