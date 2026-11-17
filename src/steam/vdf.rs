use crate::{
    error::{Error, ErrorKind, Result},
    utils::string::remove_quotes,
};
use std::path::{Path, PathBuf};

pub fn read_library_folders(file: &Path) -> Result<Vec<PathBuf>> {
    let library_folders_data = std::fs::read_to_string(&file).map_err(|error| {
        Error::new(
            ErrorKind::InvalidLibrary,
            format!(
                "Invalid Steam library config, maybe this launcher is not installed: {}",
                error.to_string()
            ),
        )
    })?;

    Ok(parse_library_folders(&library_folders_data))
}

/// `lines()` strips `\r\n` too, so CRLF files do not yield paths ending in `\r`.
fn parse_library_folders(library_folders_data: &str) -> Vec<PathBuf> {
    let mut folders = Vec::new();

    for file_line in library_folders_data.lines() {
        let line: Vec<&str> = file_line
            .split("\t")
            .filter(|str| str.trim().len() != 0)
            .collect();

        if line.len() != 2 {
            continue;
        }

        let attr = remove_quotes(line[0].trim());
        let mut value = remove_quotes(line[1].trim());

        match attr.as_str() {
            "path" => {
                if cfg!(target_os = "windows") {
                    let double_separator = std::path::MAIN_SEPARATOR.to_string()
                        + &std::path::MAIN_SEPARATOR.to_string();

                    value =
                        value.replace(&double_separator, &std::path::MAIN_SEPARATOR.to_string());
                }

                folders.push(PathBuf::from(value))
            }
            _ => {}
        }
    }

    folders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crlf_library_paths_have_no_trailing_carriage_return() {
        let text = concat!(
            "\"libraryfolders\"\r\n{\r\n",
            "\t\"0\"\r\n\t{\r\n",
            "\t\t\"path\"\t\t\"D:\\360Downloads\"\r\n",
            "\t}\r\n}\r\n",
        );
        let folders = parse_library_folders(text);
        assert_eq!(folders.len(), 1);
        assert!(!folders[0].to_string_lossy().ends_with('\r'));
        assert!(folders[0].to_string_lossy().ends_with("360Downloads"));
    }
}
