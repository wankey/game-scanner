use super::Entry;
use crate::error::{print_error, Error, ErrorKind, Result};
use std::{
    ffi::{OsStr, OsString},
    io,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Component, Path, PathBuf},
};
use winreg::{enums::*, RegKey};

const ROOT: &str = r"Software\miHoYo\HYP\1_1";
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\HYP_1_1_cn";

fn path_value(key: &RegKey, name: &str) -> io::Result<PathBuf> {
    let raw = key.get_raw_value(name)?;
    if !matches!(raw.vtype, REG_SZ | REG_EXPAND_SZ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected REG_SZ path",
        ));
    }
    let value: OsString = key.get_value(name)?;
    let path = PathBuf::from(if raw.vtype == REG_EXPAND_SZ {
        expand(&value)
    } else {
        value
    });
    if !local_path(&path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected local absolute path",
        ));
    }
    Ok(path)
}

fn expand(value: &OsStr) -> OsString {
    let input: Vec<u16> = value.encode_wide().collect();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        if input[pos] == b'%' as u16 {
            if let Some(end) = input[pos + 1..].iter().position(|c| *c == b'%' as u16) {
                let end = pos + 1 + end;
                let name = OsString::from_wide(&input[pos + 1..end]);
                if let Some(replacement) = std::env::var_os(&name) {
                    out.extend(replacement.encode_wide());
                    pos = end + 1;
                    continue;
                }
            }
        }
        out.push(input[pos]);
        pos += 1;
    }
    OsString::from_wide(&out)
}

fn local_path(path: &Path) -> bool {
    path.is_absolute()
        && matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), std::path::Prefix::Disk(_)))
}

fn open_root(view: u32) -> io::Result<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(ROOT, KEY_READ | view)
}

pub(super) fn entries() -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    let mut readable = false;
    let mut failure = None;
    for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
        let root = match open_root(view) {
            Ok(root) => {
                readable = true;
                root
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                readable = true;
                continue;
            }
            Err(e) => {
                failure = Some(e);
                continue;
            }
        };
        for name in root.enum_keys() {
            let result = (|| -> io::Result<Option<Entry>> {
                let name = name?;
                if super::catalog::find(&name).is_none() {
                    return Ok(None);
                }
                let key = root.open_subkey_with_flags(&name, KEY_READ | view)?;
                let biz: String = key.get_value("GameBiz")?;
                if biz != name {
                    return Ok(None);
                }
                Ok(Some(Entry {
                    biz,
                    root: path_value(&key, "GameInstallPath")?,
                }))
            })();
            match result {
                Ok(Some(entry)) => out.push(entry),
                Ok(None) => {}
                Err(error) => print_error(&Error::new(ErrorKind::WinReg, error)),
            }
        }
    }
    if let Some(error) = failure {
        let error = Error::new(ErrorKind::WinReg, error);
        if !readable {
            return Err(error);
        }
        print_error(&error);
    }
    Ok(out)
}

pub(super) fn still_registered(entry: &Entry) -> bool {
    [KEY_WOW64_32KEY, KEY_WOW64_64KEY].into_iter().any(|view| {
        let Ok(root) = open_root(view) else {
            return false;
        };
        let Ok(key) = root.open_subkey_with_flags(&entry.biz, KEY_READ | view) else {
            return false;
        };
        key.get_value::<String, _>("GameBiz").ok().as_deref() == Some(entry.biz.as_str())
            && path_value(&key, "GameInstallPath").ok().as_ref() == Some(&entry.root)
    })
}

pub(super) fn executable() -> Result<PathBuf> {
    for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
        if let Ok(key) = open_root(view) {
            if let Ok(path) = path_value(&key, "InstallPath") {
                let exe = path.join("launcher.exe");
                if exe.is_file() {
                    return Ok(exe);
                }
            }
        }
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, KEY_READ | view)
            {
                if let Ok(path) = path_value(&key, "InstallLocation") {
                    let exe = path.join("launcher.exe");
                    if exe.is_file() {
                        return Ok(exe);
                    }
                }
                if let Ok(path) = path_value(&key, "DisplayIcon") {
                    if path.is_file() {
                        return Ok(path);
                    }
                }
            }
        }
    }
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "miHoYo Launcher not found",
    ))
}

pub(super) fn downloads() -> Result<std::collections::HashMap<String, super::download::Download>> {
    let Some(base) = directories::BaseDirs::new() else {
        return Err(Error::new(
            ErrorKind::IO,
            "Cannot locate HoYoPlay application data",
        ));
    };
    let path = base.data_dir().join("miHoYo/HYP/1_1/data/gamedata.dat");
    if !path.try_exists()? {
        return Ok(Default::default());
    }
    super::download::load(&path)
}
