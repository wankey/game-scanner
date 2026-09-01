use crate::error::{Error, ErrorKind, Result};
use std::env;
use winreg;

pub fn get_local_machine_reg_key(sub_key: &str) -> Result<winreg::RegKey> {
    let reg = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);

    let keys = match env::consts::ARCH {
        // Most existing launchers are 32-bit, but newer launchers (including
        // Epic Games) register under the native 64-bit view.
        "x86_64" => vec![
            String::from("SOFTWARE\\WOW6432Node\\") + sub_key,
            String::from("SOFTWARE\\") + sub_key,
        ],
        _ => vec![String::from("SOFTWARE\\") + sub_key],
    };

    let mut last_error = None;
    for key in keys {
        match reg.open_subkey(key) {
            Ok(value) => return Ok(value),
            Err(error) => last_error = Some(error),
        }
    }

    Err(Error::new(
        ErrorKind::WinReg,
        last_error.expect("registry lookup always has a candidate"),
    ))
}

pub fn get_current_user_reg_key(sub_key: &str) -> Result<winreg::RegKey> {
    let reg = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);

    let key = match env::consts::ARCH {
        "x86_64" => String::from("SOFTWARE\\") + sub_key,
        _ => String::from("SOFTWARE\\") + sub_key,
    };

    return reg
        .open_subkey(key)
        .map_err(|error| Error::new(ErrorKind::WinReg, error));
}

pub fn get_sub_key(reg: &winreg::RegKey, key: &str) -> Result<winreg::RegKey> {
    return reg
        .open_subkey(key)
        .map_err(|error| Error::new(ErrorKind::WinReg, error));
}

pub fn get_value(reg: &winreg::RegKey, key: &str) -> Result<String> {
    return reg
        .get_value::<String, &str>(key)
        .map_err(|error| Error::new(ErrorKind::WinReg, error));
}
