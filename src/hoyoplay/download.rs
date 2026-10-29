//! Read-only unencrypted MMKV v3/v4 subset used by the observed gamedata.dat.
//! Format references: Tencent/MMKV Core/{MMKVMetaInfo.hpp,MMKV_IO.cpp,MiniPBCoder.cpp,CodedInputData.cpp}.
use crate::error::{Error, ErrorKind, Result};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const LIMIT: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
pub(super) struct Download {
    #[serde(rename = "gameBiz")]
    biz: String,
    #[serde(rename = "installPath")]
    path: PathBuf,
    downloading: bool,
}

impl Download {
    pub(super) fn matches(&self, biz: &str, path: &Path) -> bool {
        self.downloading
            && self.biz == biz
            && path
                .canonicalize()
                .ok()
                .zip(self.path.canonicalize().ok())
                .is_some_and(|(a, b)| a == b)
    }
}

fn invalid() -> Error {
    Error::new(
        ErrorKind::InvalidManifest,
        "Invalid or unsupported HoYoPlay MMKV download snapshot",
    )
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid());
    }
    Ok(bytes)
}

pub(super) fn load(path: &Path) -> Result<HashMap<String, Download>> {
    let crc_path = path.with_extension("dat.crc");
    for _ in 0..3 {
        // Read metadata on both sides; MMKV updates the size/checksum on each write.
        let before = read(&crc_path, 4096)?;
        let data = read(path, LIMIT)?;
        let after = read(&crc_path, 4096)?;
        if before == after {
            if let Ok(records) = decode(&data, &after) {
                return Ok(records);
            }
        }
    }
    Err(invalid())
}

fn uint32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_| invalid())?,
    ))
}

struct Input<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Input<'a> {
    fn varint(&mut self) -> Result<usize> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let byte = *self.bytes.get(self.pos).ok_or_else(invalid)?;
            self.pos += 1;
            if shift == 28 && byte > 15 {
                return Err(invalid());
            }
            value |= u32::from(byte & 127) << shift;
            if byte < 128 {
                return Ok(value as usize);
            }
        }
        Err(invalid())
    }
    fn data(&mut self) -> Result<&'a [u8]> {
        let size = self.varint()?;
        let end = self.pos.checked_add(size).ok_or_else(invalid)?;
        let value = self.bytes.get(self.pos..end).ok_or_else(invalid)?;
        self.pos = end;
        Ok(value)
    }
}

fn decode(data: &[u8], meta: &[u8]) -> Result<HashMap<String, Download>> {
    let version = uint32(meta, 4)?;
    if !(3..=4).contains(&version)
        || meta
            .get(12..28)
            .ok_or_else(invalid)?
            .iter()
            .any(|b| *b != 0)
    {
        return Err(invalid()); // No encryption, expiry or future-format guessing.
    }
    if version == 4
        && meta
            .get(104..112)
            .ok_or_else(invalid)?
            .iter()
            .any(|b| *b != 0)
    {
        return Err(invalid());
    }
    let size = uint32(meta, 28)? as usize;
    if uint32(data, 0)? as usize != size {
        return Err(invalid());
    }
    let payload = data
        .get(4..4usize.checked_add(size).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    if crc32fast::hash(payload) != uint32(meta, 0)? {
        return Err(invalid());
    }
    if payload.is_empty() {
        return Ok(HashMap::new());
    }
    let mut input = Input {
        bytes: payload,
        pos: 0,
    };
    input.varint()?; // Container prefix is reserved; appended records exceed its initial size.
    let mut values = HashMap::new();
    while input.pos < payload.len() {
        let key = std::str::from_utf8(input.data()?).map_err(|_| invalid())?;
        if key.is_empty() {
            return Err(invalid());
        }
        let value = input.data()?;
        if value.is_empty() {
            values.remove(key);
        } else {
            values.insert(key, value);
        }
    }
    let mut downloads = HashMap::new();
    for (key, value) in values {
        if super::catalog::find(key).is_none() {
            continue;
        }
        let mut string = Input {
            bytes: value,
            pos: 0,
        };
        let json = string.data()?;
        if string.pos != value.len() {
            return Err(invalid());
        }
        let record: Download = serde_json::from_slice(json)?;
        if record.biz != key {
            return Err(invalid());
        }
        downloads.insert(key.to_owned(), record);
    }
    Ok(downloads)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn varint(mut value: usize, out: &mut Vec<u8>) {
        while value >= 128 {
            out.push((value as u8 & 127) | 128);
            value >>= 7;
        }
        out.push(value as u8);
    }
    fn blob(bytes: &[u8], out: &mut Vec<u8>) {
        varint(bytes.len(), out);
        out.extend(bytes);
    }
    fn snapshot(records: &[(&str, Option<&str>)]) -> (Vec<u8>, Vec<u8>) {
        let mut payload = vec![];
        varint(0xffffff, &mut payload);
        for (key, json) in records {
            blob(key.as_bytes(), &mut payload);
            let mut value = vec![];
            if let Some(json) = json {
                blob(json.as_bytes(), &mut value);
            }
            blob(&value, &mut payload);
        }
        let mut data = (payload.len() as u32).to_le_bytes().to_vec();
        data.extend(&payload);
        let mut meta = vec![0u8; 112];
        meta[..4].copy_from_slice(&crc32fast::hash(&payload).to_le_bytes());
        meta[4..8].copy_from_slice(&4u32.to_le_bytes());
        meta[28..32].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        (data, meta)
    }
    const ACTIVE: &str =
        r#"{"gameBiz":"hkrpg_cn","installPath":"F:\\Star Rail Game","downloading":true}"#;
    const PAUSED: &str =
        r#"{"gameBiz":"hkrpg_cn","installPath":"F:\\Star Rail Game","downloading":false}"#;

    #[test]
    fn latest_value_wins_and_unused_tail_is_ignored() {
        let (mut data, meta) = snapshot(&[("hkrpg_cn", Some(ACTIVE)), ("hkrpg_cn", Some(PAUSED))]);
        data.extend_from_slice(ACTIVE.as_bytes());
        assert!(!decode(&data, &meta).unwrap()["hkrpg_cn"].downloading);
    }
    #[test]
    fn tombstones_remove_old_downloads() {
        let (data, meta) = snapshot(&[("hkrpg_cn", Some(ACTIVE)), ("hkrpg_cn", None)]);
        assert!(decode(&data, &meta).unwrap().is_empty());
    }
    #[test]
    fn rejects_crc_size_future_flags_and_truncated_records() {
        let (data, meta) = snapshot(&[("hkrpg_cn", Some(ACTIVE))]);
        for index in [0, 4, 28, 104] {
            let mut bad = meta.clone();
            bad[index] ^= 1;
            assert!(decode(&data, &bad).is_err());
        }
        assert!(decode(&data[..data.len() - 1], &meta).is_err());
    }

    #[test]
    fn broken_latest_value_does_not_resurrect_old_download() {
        let (data, meta) = snapshot(&[("hkrpg_cn", Some(ACTIVE)), ("hkrpg_cn", Some("{}"))]);
        assert!(decode(&data, &meta).is_err());
    }

    #[test]
    fn bounded_reader_rejects_bad_varints_and_lengths() {
        for bytes in [
            &[0xff, 0xff, 0xff, 0xff, 0xff][..],
            &[20, 1][..],
            &[0x80][..],
        ] {
            assert!(Input { bytes, pos: 0 }.data().is_err());
        }
    }
    #[test]
    fn download_must_match_game_and_real_path() {
        let dir = tempfile::tempdir().unwrap();
        let record = Download {
            biz: "nap_cn".into(),
            path: dir.path().into(),
            downloading: true,
        };
        assert!(record.matches("nap_cn", dir.path()));
        assert!(!record.matches("hkrpg_cn", dir.path()));
        let other = tempfile::tempdir().unwrap();
        assert!(!record.matches("nap_cn", other.path()));
    }
}
