use super::Entry;
use crate::error::{print_error, Error, ErrorKind, Result};
use std::{
    ffi::{OsStr, OsString},
    io,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};
use winreg::{enums::*, RegKey};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

fn value(key: &RegKey, name: &str) -> Option<OsString> {
    let raw = key.get_raw_value(name).ok()?;
    let value: OsString = key.get_value(name).ok()?;
    Some(if raw.vtype == REG_EXPAND_SZ {
        expand(&value)
    } else {
        value
    })
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

fn local_file(path: &Path) -> bool {
    let text = path.as_os_str().encode_wide().collect::<Vec<_>>();
    // Registry inputs must be drive-qualified local paths, not UNC/device paths.
    text.len() >= 3
        && text[1] == b':' as u16
        && matches!(text[2], 47 | 92)
        && path.is_absolute()
        && path.is_file()
}

fn direct_executable(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    !matches!(
        name.as_str(),
        "msiexec.exe"
            | "rundll32.exe"
            | "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
    ) && local_file(path)
}

fn command_path(command: &OsStr) -> Option<PathBuf> {
    let wide: Vec<u16> = command.encode_wide().collect();
    let start = wide.iter().position(|c| !matches!(*c, 9 | 32))?;
    if wide[start] == b'"' as u16 {
        let rest = &wide[start + 1..];
        let end = rest.iter().position(|c| *c == b'"' as u16)?;
        let path = PathBuf::from(OsString::from_wide(&rest[..end]));
        return direct_executable(&path).then_some(path);
    }
    let mut candidates = Vec::new();
    for end in start + 4..=wide.len() {
        let suffix = &wide[end - 4..end];
        if suffix[0] == 46
            && suffix[1] | 32 == 101
            && suffix[2] | 32 == 120
            && suffix[3] | 32 == 101
            && (end == wide.len() || matches!(wide[end], 9 | 32))
        {
            let path = PathBuf::from(OsString::from_wide(&wide[start..end]));
            if direct_executable(&path) {
                candidates.push(path);
            }
        }
    }
    if candidates.len() == 1 {
        candidates.pop()
    } else {
        None
    }
}

fn icon_path(icon: &OsStr) -> Option<PathBuf> {
    let wide: Vec<u16> = icon.encode_wide().collect();
    let end = if let Some(comma) = wide.iter().rposition(|c| *c == b',' as u16) {
        let suffix = String::from_utf16_lossy(&wide[comma + 1..]);
        if suffix.trim().parse::<i32>().is_ok() {
            comma
        } else {
            wide.len()
        }
    } else {
        wide.len()
    };
    let mut text = &wide[..end];
    while text.first().is_some_and(|c| matches!(*c, 9 | 32)) {
        text = &text[1..];
    }
    while text.last().is_some_and(|c| matches!(*c, 9 | 32)) {
        text = &text[..text.len() - 1];
    }
    if text.first() == Some(&34) && text.last() == Some(&34) && text.len() >= 2 {
        text = &text[1..text.len() - 1];
    }
    let path = PathBuf::from(OsString::from_wide(text));
    local_file(&path).then_some(path)
}

fn scopes() -> [(winreg::HKEY, u8); 2] {
    [(HKEY_CURRENT_USER, 0), (HKEY_LOCAL_MACHINE, 1)]
}

pub(super) fn entries() -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    let mut successes = 0;
    let mut failure = None;
    for (hive, priority) in scopes() {
        for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
            let parent =
                match RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, KEY_READ | view) {
                    Ok(key) => key,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        successes += 1;
                        continue;
                    }
                    Err(error) => {
                        failure = Some(error);
                        continue;
                    }
                };
            let mut source_failed = false;
            for name in parent.enum_keys() {
                let name = match name {
                    Ok(name) => name,
                    Err(error) => {
                        source_failed = true;
                        failure = Some(error);
                        continue;
                    }
                };
                let key = match parent.open_subkey_with_flags(&name, KEY_READ | view) {
                    Ok(key) => key,
                    Err(error) => {
                        print_error(&Error::new(ErrorKind::WinReg, error));
                        continue;
                    }
                };
                let uninstaller =
                    value(&key, "UninstallString").and_then(|value| command_path(&value));
                let mut roots = Vec::new();
                if let Some(location) = value(&key, "InstallLocation") {
                    let path = PathBuf::from(location);
                    let text = path.as_os_str().encode_wide().collect::<Vec<_>>();
                    if text.len() >= 3
                        && text[1] == 58
                        && matches!(text[2], 47 | 92)
                        && path.is_absolute()
                    {
                        roots.push(path);
                    }
                }
                if let Some(path) = uninstaller.as_ref().and_then(|p| p.parent()) {
                    roots.push(path.into());
                }
                if let Some(path) = value(&key, "DisplayIcon").and_then(|value| icon_path(&value)) {
                    if let Some(parent) = path.parent() {
                        roots.push(parent.into());
                    }
                }
                if !roots.is_empty() {
                    out.push(Entry {
                        priority,
                        key: name,
                        name: key.get_value("DisplayName").unwrap_or_default(),
                        roots,
                        uninstaller,
                    });
                }
            }
            if !source_failed {
                successes += 1;
            }
        }
    }
    if let Some(error) = failure {
        let error = Error::new(ErrorKind::WinReg, error);
        if successes == 0 {
            return Err(error);
        }
        print_error(&error);
    }
    Ok(out)
}

pub(super) fn executable() -> Result<PathBuf> {
    for (hive, _) in scopes() {
        for view in [KEY_WOW64_32KEY, KEY_WOW64_64KEY] {
            let root = RegKey::predef(hive);
            if let Ok(key) = root.open_subkey_with_flags(
                r"Software\Classes\wegame\shell\open\command",
                KEY_READ | view,
            ) {
                if let Some(path) = value(&key, "").and_then(|v| command_path(&v)) {
                    return Ok(path);
                }
            }
            if let Ok(key) =
                root.open_subkey_with_flags(format!(r"{UNINSTALL}\WeGame"), KEY_READ | view)
            {
                if let Some(path) = value(&key, "DisplayIcon").and_then(|v| icon_path(&v)) {
                    return Ok(path);
                }
                if let Some(path) = value(&key, "UninstallString").and_then(|v| command_path(&v)) {
                    if let Some(parent) = path.parent() {
                        let exe = parent.join("wegame.exe");
                        if local_file(&exe) {
                            return Ok(exe);
                        }
                    }
                }
            }
        }
    }
    Err(Error::new(
        ErrorKind::LauncherNotFound,
        "WeGame executable not found",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unicode_unquoted_and_quoted_commands_and_icons() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("崩坏 3 (test).exe");
        std::fs::write(&path, b"test").unwrap();
        assert_eq!(
            command_path(OsStr::new(&format!("{} /uninstall", path.display()))),
            Some(path.clone())
        );
        assert_eq!(
            command_path(OsStr::new(&format!("\"{}\" /uninstall", path.display()))),
            Some(path.clone())
        );
        assert_eq!(
            icon_path(OsStr::new(&format!("\"{}\",0", path.display()))),
            Some(path)
        );
    }

    #[test]
    fn ignores_public_uninstall_hosts_and_missing_files() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("msiexec.exe");
        std::fs::write(&host, b"test").unwrap();
        assert!(command_path(host.as_os_str()).is_none());
        assert!(command_path(OsStr::new(r"C:\missing\uninstall.exe")).is_none());
    }

    #[test]
    fn expands_environment_without_changing_unknown_variables() {
        let expected = std::env::var_os("SystemRoot").unwrap();
        assert_eq!(expand(OsStr::new("%SystemRoot%")), expected);
        assert_eq!(
            expand(OsStr::new("%GAME_SCANNER_NONEXISTENT_VARIABLE%")),
            OsString::from("%GAME_SCANNER_NONEXISTENT_VARIABLE%")
        );
    }

    #[test]
    fn refuses_ambiguous_unquoted_executable_paths() {
        let dir = tempfile::tempdir().unwrap();
        let short = dir.path().join("game.exe");
        let long = dir.path().join("game.exe other.exe");
        std::fs::write(&short, b"test").unwrap();
        std::fs::write(&long, b"test").unwrap();
        assert!(command_path(long.as_os_str()).is_none());
        assert_eq!(
            command_path(OsStr::new(&format!("\"{}\"", long.display()))),
            Some(long)
        );
    }
}
