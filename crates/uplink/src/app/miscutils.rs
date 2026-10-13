//! Port of source/app/miscutils.h + source/app/miscutils.cpp
//!
//! Misc string / file utils. `PrintStackTrace` is not ported (use `std::backtrace`).

#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

pub use crate::app::opengl_interface::set_colour;

/// `RAND_MAX` of the C runtime the game was written against (glibc).
pub const RAND_MAX: i32 = 0x7fff_ffff;

/// Replacement for C `rand()`: uniform in `0..=RAND_MAX`.
pub fn rand_i32() -> i32 {
    use rand::Rng;
    rand::thread_rng().gen_range(0..=RAND_MAX)
}

/// `LowerCaseString`
pub fn lower_case_string(thestring: &str) -> String {
    thestring.to_ascii_lowercase()
}

/// `StripCarriageReturns`: shortens the string at the first `\n`.
pub fn strip_carriage_returns(thestring: &str) -> String {
    match thestring.find('\n') {
        Some(i) => thestring[..i].to_string(),
        None => thestring.to_string(),
    }
}

/// `GetFilePath`: everything up to and including the last `/` (or `\`), or `./`.
pub fn get_file_path(filename: &str) -> String {
    match filename.rfind('/').or_else(|| filename.rfind('\\')) {
        Some(i) => filename[..=i].to_string(),
        None => "./".to_string(),
    }
}

/// `MakeDirectory`: creates the directory, ignoring errors (like `mkdir`).
pub fn make_directory(directory: &str) {
    // mkdir(dirname, 0700) on unix; _mkdir on Windows (no mode).
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let _ = std::fs::DirBuilder::new().mode(0o700).create(directory);
    }
    #[cfg(not(unix))]
    {
        let _ = std::fs::create_dir(directory);
    }
}

/// `DoesFileExist`
pub fn does_file_exist(filename: &str) -> bool {
    std::path::Path::new(filename).exists()
}

/// `ListDirectory`: archive listing plus local files whose name contains `filter`,
/// each prefixed with `directory`, duplicates removed.
pub fn list_directory(directory: &str, filter: &str) -> Vec<String> {
    // Start with a listing from Redshirt archives (RsListArchive strips the app path,
    // which is what `archive::list_files` returns).
    let mut result: Vec<String> = uplink_engine::archive::list_files(directory, filter);

    // Now add on all files found locally.
    // PORT: readdir also reports "." and ".."; `read_dir` skips them, so add them back
    // (they match only when the filter is contained in them, e.g. the empty filter).
    if let Ok(entries) = std::fs::read_dir(directory) {
        for name in [".".to_string(), "..".to_string()] {
            if name.contains(filter) {
                result.push(format!("{}{}", directory, name));
            }
        }
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.contains(filter) {
                result.push(format!("{}{}", directory, name));
            }
        }
    }

    // Now make absolutely sure there are no duplicates (the first occurrence is kept).
    let mut unique: Vec<String> = Vec::with_capacity(result.len());
    for name in result {
        if !unique.contains(&name) {
            unique.push(name);
        }
    }
    unique
}

/// `ListSubdirs`: names (not paths) of the sub-directories of `directory`, skipping dot-dirs.
pub fn list_subdirs(directory: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // stat() follows symlinks, as in the C++.
            let fullfilename = format!("{}/{}", directory, name);
            let is_dir = std::fs::metadata(&fullfilename)
                .map(|info| info.is_dir())
                .unwrap_or(false);
            if is_dir && !name.starts_with('.') {
                result.push(name);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::scratch_dir;

    #[test]
    fn rand_stays_in_the_c_range() {
        for _ in 0..1000 {
            let r = rand_i32();
            assert!((0..=RAND_MAX).contains(&r));
        }
    }

    #[test]
    fn string_helpers() {
        assert_eq!(lower_case_string("HeLLo WORLD 42"), "hello world 42");
        assert_eq!(strip_carriage_returns("line one\nline two"), "line one");
        assert_eq!(strip_carriage_returns("no newline"), "no newline");
        assert_eq!(strip_carriage_returns("\nstarts empty"), "");
    }

    #[test]
    fn file_path_keeps_the_trailing_separator() {
        assert_eq!(get_file_path("/usr/games/uplink"), "/usr/games/");
        assert_eq!(get_file_path("c:\\uplink\\uplink.exe"), "c:\\uplink\\");
        assert_eq!(get_file_path("uplink"), "./");
        assert_eq!(get_file_path("./uplink"), "./");
    }

    #[test]
    fn directories_and_listings() {
        let dir = scratch_dir("list");
        assert!(does_file_exist(&dir));
        assert!(!does_file_exist(&format!("{dir}missing")));

        make_directory(&format!("{dir}sub"));
        make_directory(&format!("{dir}.hidden"));
        // creating a directory twice is not an error
        make_directory(&format!("{dir}sub"));
        std::fs::write(format!("{dir}agent.usr"), b"x").unwrap();
        std::fs::write(format!("{dir}other.txt"), b"x").unwrap();

        assert_eq!(list_subdirs(dir.trim_end_matches('/')), vec!["sub".to_string()]);

        let mut usr = list_directory(&dir, ".usr");
        usr.sort();
        assert_eq!(usr, vec![format!("{dir}agent.usr")]);

        // no entry is listed twice
        let all = list_directory(&dir, "");
        let mut unique = all.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(all.len(), unique.len());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
