//! Port of `redshirt`: Uplink's trivial file "encryption".
//!
//! An encrypted file is the 9-byte marker `REDSHIRT\0` followed by the payload
//! with 128 added to every byte (wrapping).

use std::path::Path;

pub const MARKER: &[u8; 9] = b"REDSHIRT\0";

pub fn encrypt_buffer(buf: &mut [u8]) {
    for b in buf {
        *b = b.wrapping_add(128);
    }
}

pub fn decrypt_buffer(buf: &mut [u8]) {
    for b in buf {
        *b = b.wrapping_sub(128);
    }
}

pub fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.starts_with(MARKER)
}

/// Strip the header and decrypt if `bytes` is redshirt-encrypted; otherwise return as is.
pub fn decrypt_if_encrypted(mut bytes: Vec<u8>) -> Vec<u8> {
    if is_encrypted(&bytes) {
        bytes.drain(..MARKER.len());
        decrypt_buffer(&mut bytes);
    }
    bytes
}

/// Produce an encrypted copy of `plain` (header + shifted bytes).
pub fn encrypt(plain: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(plain.len() + MARKER.len());
    out.extend_from_slice(MARKER);
    let start = out.len();
    out.extend_from_slice(plain);
    encrypt_buffer(&mut out[start..]);
    out
}

/// `RsFileEncrypted`
pub fn file_encrypted(path: &Path) -> bool {
    std::fs::read(path).map(|b| is_encrypted(&b)).unwrap_or(false)
}

/// Read a file from disk, transparently decrypting it.
pub fn read_file(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path).map(decrypt_if_encrypted)
}

/// Write a file to disk, encrypted if `encrypted` is true.
pub fn write_file(path: &Path, plain: &[u8], encrypted: bool) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if encrypted {
        std::fs::write(path, encrypt(plain))
    } else {
        std::fs::write(path, plain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let plain = b"PK\x03\x04hello".to_vec();
        let enc = encrypt(&plain);
        assert!(is_encrypted(&enc));
        assert_eq!(enc[9], b'P'.wrapping_add(128));
        assert_eq!(decrypt_if_encrypted(enc), plain);
    }
}
