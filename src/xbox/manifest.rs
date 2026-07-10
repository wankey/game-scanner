use crate::error::{Error, ErrorKind, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

pub struct ParsedManifest {
    pub application_id: String,
    pub display_name: String,
}

pub fn parse(path: &Path) -> Result<ParsedManifest> {
    let xml = std::fs::read_to_string(path).map_err(|e| {
        Error::new(
            ErrorKind::IO,
            format!("AppxManifest.xml missing at {}: {}", path.display(), e),
        )
    })?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut application_id = String::new();
    let mut display_name = String::new();
    let mut current_app_open = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"Application" {
                    current_app_open = true;
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Id" {
                            application_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if name_bytes == b"DisplayName" && display_name.is_empty() {
                    // capture text in the next Text event
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"Application" && application_id.is_empty() {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Id" {
                            application_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if current_app_open {
                    // ignore text inside <Application> — we only want attributes
                } else if display_name.is_empty() {
                    let raw = t.decode().unwrap_or_default();
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        display_name = trimmed.to_string();
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"Application" {
                    current_app_open = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::InvalidManifest,
                    format!("Failed to parse {}: {}", path.display(), e),
                ));
            }
            _ => {}
        }
        buf.clear();
    }

    if application_id.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            format!("No <Application Id=\"...\"> found in {}", path.display()),
        ));
    }

    Ok(ParsedManifest {
        application_id,
        display_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_manifest(body: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(body.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_extracts_application_id_and_display_name() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<Package>
  <Properties>
    <DisplayName>Forza Horizon 5</DisplayName>
  </Properties>
  <Applications>
    <Application Id="Forza" Executable="ForzaHorizon5.exe" />
  </Applications>
</Package>"#;
        let f = write_manifest(body);
        let parsed = parse(f.path()).unwrap();
        assert_eq!(parsed.application_id, "Forza");
        assert_eq!(parsed.display_name, "Forza Horizon 5");
    }

    #[test]
    fn parse_returns_error_when_application_missing() {
        let body = r#"<?xml version="1.0"?><Package><Properties><DisplayName>X</DisplayName></Properties></Package>"#;
        let f = write_manifest(body);
        assert!(parse(f.path()).is_err());
    }

    #[test]
    fn parse_returns_error_on_malformed_xml() {
        let body = "not xml at all";
        let f = write_manifest(body);
        assert!(parse(f.path()).is_err());
    }
}