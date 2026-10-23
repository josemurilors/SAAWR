//! All game-file I/O goes through here: bytes in, bytes out, never re-encoded.

use crate::error::{AwrError, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| AwrError::io(path, e))
}

/// Writes to a temp file in the same directory, then renames over the target,
/// so a crash leaves either the old file or the new one, never half of each.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = sibling(path, ".awr-tmp");
    let mut f = fs::File::create(&tmp).map_err(|e| AwrError::io(&tmp, e))?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| AwrError::io(&tmp, e))?;
    drop(f);
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AwrError::io(path, e)
    })
}

/// Keeps the first `.bak` forever: it is the pre-AWR original, not the last edit.
pub fn backup_once(path: &Path) -> Result<PathBuf> {
    let bak = sibling(path, ".bak");
    if !bak.exists() {
        fs::copy(path, &bak).map_err(|e| AwrError::io(path, e))?;
    }
    Ok(bak)
}

/// For display and logs only; game files are never written from a String.
pub fn cp1252_to_string(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POISONED: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/poisoned_cp1252.ini"
    );

    #[test]
    fn atomic_write_round_trips_cp1252_bytes_exactly() {
        let original = read(Path::new(POISONED)).unwrap();
        assert!(
            original.contains(&0xE7),
            "fixture must hold raw CP-1252 'ç'"
        );
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("x.ini");
        write_atomic(&target, &original).unwrap();
        assert_eq!(read(&target).unwrap(), original);
        assert!(!sibling(&target, ".awr-tmp").exists());
    }

    #[test]
    fn backup_keeps_the_first_original() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.cfg");
        fs::write(&target, b"v1").unwrap();
        let bak = backup_once(&target).unwrap();
        write_atomic(&target, b"v2").unwrap();
        backup_once(&target).unwrap();
        assert_eq!(fs::read(bak).unwrap(), b"v1");
    }

    #[test]
    fn cp1252_decodes_for_display() {
        // 0xA3 is the '£' melee prefix of weapon.dat
        assert_eq!(cp1252_to_string(&[0xE7, 0xE3, 0xA3]), "çã£");
    }

    #[test]
    fn fixtures_are_crlf() {
        let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"));
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            let lf = bytes.iter().filter(|&&b| b == b'\n').count();
            let crlf = bytes.windows(2).filter(|w| w == b"\r\n").count();
            assert!(lf > 0 && lf == crlf, "{} must be CRLF only", path.display());
        }
    }
}
