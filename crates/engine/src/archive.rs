//! Port of `bungle` (zip access) + the archive half of `redshirt`.
//!
//! Uplink ships its assets as `.dat` files next to the executable. Each one is a
//! *stored* (uncompressed) zip archive, optionally wrapped in the redshirt
//! "encryption" (a `REDSHIRT\0` header followed by every byte shifted by 128).
//!
//! All archives are mounted into one case-insensitive virtual file system.
//! Later archives override earlier ones (so `patch3.dat` wins over `patch.dat`).
//! Loose files under the data directory override everything, which makes modding easy.
//!
//! Paths are the ones the C++ code used, e.g. `"graphics/close.tif"`,
//! `"sounds/login.wav"`, `"data/fornames.txt"`, `"fonts/dungeon.ttf"`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::redshirt;

/// The order the original game mounted its archives in (later = higher priority).
pub const DAT_FILES: &[&str] = &[
    "data.dat",
    "graphics.dat",
    "loading.dat",
    "sounds.dat",
    "music.dat",
    "fonts.dat",
    "patch.dat",
    "patch2.dat",
    "patch3.dat",
];

#[derive(Default)]
struct Vfs {
    data_dir: Option<PathBuf>,
    /// lowercase path -> file bytes
    files: BTreeMap<String, Vec<u8>>,
}

thread_local! {
    static VFS: RefCell<Vfs> = RefCell::new(Vfs::default());
}

fn normalise(name: &str) -> String {
    name.replace('\\', "/").trim_start_matches("./").trim_start_matches('/').to_ascii_lowercase()
}

/// Locate the directory holding the original game's `.dat` files.
///
/// Search order: `$UPLINK_DATA`, the executable's directory, the executable's
/// directory's `uplink/` subfolder, the current directory.
pub fn find_data_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("UPLINK_DATA") {
        candidates.push(PathBuf::from(dir));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
            candidates.push(dir.join("uplink"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    candidates.into_iter().find(|d| d.join("data.dat").is_file())
}

/// Mount every known `.dat` archive found in `data_dir`. Returns the number mounted.
pub fn init(data_dir: &Path) -> usize {
    VFS.with(|v| v.borrow_mut().data_dir = Some(data_dir.to_path_buf()));
    let mut mounted = 0;
    for dat in DAT_FILES {
        let path = data_dir.join(dat);
        if path.is_file() && open_zip_file(&path).is_ok() {
            mounted += 1;
        }
    }
    mounted
}

/// The data directory passed to [`init`], if any.
pub fn data_dir() -> Option<PathBuf> {
    VFS.with(|v| v.borrow().data_dir.clone())
}

/// Mount one archive (decrypting it first if it carries the redshirt header).
pub fn open_zip_file(path: &Path) -> Result<usize, String> {
    let raw = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bytes = redshirt::decrypt_if_encrypted(raw);
    let entries = parse_stored_zip(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let n = entries.len();
    VFS.with(|v| {
        let mut v = v.borrow_mut();
        for (name, data) in entries {
            v.files.insert(normalise(&name), data);
        }
    });
    Ok(n)
}

/// Parse a zip archive whose entries are all stored (method 0).
/// Directory entries are skipped.
pub fn parse_stored_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let u16_at = |o: usize| -> Result<u16, String> {
        bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| "truncated".to_string())
    };
    let u32_at = |o: usize| -> Result<u32, String> {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| "truncated".to_string())
    };

    let mut out = Vec::new();
    let mut pos = 0usize;
    // Walk local file headers (PK\x03\x04) sequentially, as bungle did.
    while pos + 30 <= bytes.len() && u32_at(pos)? == 0x0403_4b50 {
        let flags = u16_at(pos + 6)?;
        let method = u16_at(pos + 8)?;
        let mut comp_size = u32_at(pos + 18)? as usize;
        let name_len = u16_at(pos + 26)? as usize;
        let extra_len = u16_at(pos + 28)? as usize;
        let name_start = pos + 30;
        let name = String::from_utf8_lossy(bytes.get(name_start..name_start + name_len).ok_or("truncated name")?)
            .into_owned();
        let data_start = name_start + name_len + extra_len;
        if flags & 0x08 != 0 && comp_size == 0 {
            // Sizes are in a trailing data descriptor; look them up in the central directory.
            comp_size = central_size_for(bytes, &name).ok_or("missing central directory entry")?;
        }
        if method != 0 {
            return Err(format!("{name}: unsupported compression method {method}"));
        }
        let data = bytes.get(data_start..data_start + comp_size).ok_or("truncated data")?.to_vec();
        if !name.ends_with('/') {
            out.push((name, data));
        }
        pos = data_start + comp_size;
        if flags & 0x08 != 0 {
            // skip data descriptor (optional signature + crc + sizes)
            pos += if u32_at(pos).ok() == Some(0x0807_4b50) { 16 } else { 12 };
        }
    }
    Ok(out)
}

fn central_size_for(bytes: &[u8], wanted: &str) -> Option<usize> {
    let mut i = 0;
    while i + 46 <= bytes.len() {
        if bytes[i..i + 4] == [0x50, 0x4b, 0x01, 0x02] {
            let comp = u32::from_le_bytes(bytes[i + 20..i + 24].try_into().ok()?) as usize;
            let nlen = u16::from_le_bytes(bytes[i + 28..i + 30].try_into().ok()?) as usize;
            let elen = u16::from_le_bytes(bytes[i + 30..i + 32].try_into().ok()?) as usize;
            let clen = u16::from_le_bytes(bytes[i + 32..i + 34].try_into().ok()?) as usize;
            let name = bytes.get(i + 46..i + 46 + nlen)?;
            if name == wanted.as_bytes() {
                return Some(comp);
            }
            i += 46 + nlen + elen + clen;
        } else {
            i += 1;
        }
    }
    None
}

/// Insert (or replace) a file in the virtual file system. Mostly for tests.
pub fn insert(name: &str, data: Vec<u8>) {
    VFS.with(|v| v.borrow_mut().files.insert(normalise(name), data));
}

/// Read a file. Loose files in the data directory win over archive contents.
/// Tries `name`, then `graphics/name` style fallbacks are the caller's job.
pub fn read(name: &str) -> Option<Vec<u8>> {
    let key = normalise(name);
    VFS.with(|v| {
        let v = v.borrow();
        if let Some(dir) = &v.data_dir {
            let loose = dir.join(&key);
            if loose.is_file() {
                if let Ok(b) = std::fs::read(loose) {
                    return Some(b);
                }
            }
        }
        v.files.get(&key).cloned()
    })
}

/// Read a text file (lossy UTF-8; the originals are Latin-1/ASCII).
pub fn read_text(name: &str) -> Option<String> {
    read(name).map(|b| b.iter().map(|&c| c as char).collect())
}

/// Equivalent of `RsArchiveFileLoaded` / `BglFileLoaded`.
pub fn exists(name: &str) -> bool {
    let key = normalise(name);
    VFS.with(|v| {
        let v = v.borrow();
        v.files.contains_key(&key) || v.data_dir.as_ref().is_some_and(|d| d.join(&key).is_file())
    })
}

/// Equivalent of `BglListFiles(path, directory, filter)`: every archived file whose
/// path starts with `directory` and contains `filter` (both case-insensitive).
pub fn list_files(directory: &str, filter: &str) -> Vec<String> {
    let dir = normalise(directory);
    let filter = filter.to_ascii_lowercase();
    VFS.with(|v| {
        v.borrow()
            .files
            .keys()
            .filter(|k| k.starts_with(&dir) && k.contains(&filter))
            .cloned()
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data) in entries {
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&10u16.to_le_bytes()); // version
            out.extend_from_slice(&0u16.to_le_bytes()); // flags
            out.extend_from_slice(&0u16.to_le_bytes()); // method
            out.extend_from_slice(&[0; 8]); // time, date, crc
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
        }
        out
    }

    #[test]
    fn parses_stored_zip() {
        let zip = make_zip(&[("data/", b""), ("data/a.txt", b"hello"), ("data/B.TXT", b"x")]);
        let files = parse_stored_zip(&zip).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].1, b"hello");
    }

    #[test]
    fn vfs_is_case_insensitive() {
        insert("Graphics/Close.TIF", vec![1, 2, 3]);
        assert!(exists("graphics/close.tif"));
        assert_eq!(read("GRAPHICS\\close.tif"), Some(vec![1, 2, 3]));
        assert_eq!(list_files("graphics/", ".tif").len(), 1);
    }
}
