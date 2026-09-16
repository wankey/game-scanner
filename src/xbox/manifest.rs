use crate::error::{Error, ErrorKind, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

pub struct ParsedManifest {
    pub identity_name: String,
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

    let mut identity_name = String::new();
    let mut display_name = String::new();
    let mut in_display_name = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"Identity" && identity_name.is_empty() {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Name" {
                            identity_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if name_bytes == b"DisplayName" && display_name.is_empty() {
                    in_display_name = true;
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();
                if name_bytes == b"Identity" && identity_name.is_empty() {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Name" {
                            identity_name = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if in_display_name && display_name.is_empty() {
                    let raw = t.decode().unwrap_or_default();
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        display_name = trimmed.to_string();
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"DisplayName" {
                    in_display_name = false;
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

    if identity_name.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidManifest,
            format!("No <Identity Name=\"...\"> found in {}", path.display()),
        ));
    }

    Ok(ParsedManifest {
        identity_name,
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
    fn parse_extracts_identity_name_and_display_name() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<Package>
  <Identity Name="ForzaHorizon5" />
  <Properties>
    <DisplayName>Forza Horizon 5</DisplayName>
  </Properties>
  <Applications>
    <Application Id="Forza" Executable="ForzaHorizon5.exe" />
  </Applications>
</Package>"#;
        let f = write_manifest(body);
        let parsed = parse(f.path()).unwrap();
        assert_eq!(parsed.identity_name, "ForzaHorizon5");
        assert_eq!(parsed.display_name, "Forza Horizon 5");
    }

    #[test]
    fn parse_returns_error_when_identity_missing() {
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

    #[test]
    fn parse_handles_namespaced_manifest_elements() {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="MinecraftUWP" />
  <Properties>
    <DisplayName>Minecraft</DisplayName>
  </Properties>
  <Applications>
    <Application Id="Minecraft" Executable="Minecraft.exe">
      <uap:VisualElements DisplayName="Should be ignored" />
    </Application>
  </Applications>
</Package>"#;
        let f = write_manifest(body);
        let parsed = parse(f.path()).unwrap();
        assert_eq!(parsed.identity_name, "MinecraftUWP");
        assert_eq!(parsed.display_name, "Minecraft");
    }
}
