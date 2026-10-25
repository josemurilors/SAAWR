//! Read-only access to GTA SA `.img` archives (version 2: `VER2`, then 32-byte entries of
//! offset and size in 2048-byte sectors plus a 24-byte name).

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const SECTOR: u64 = 2048;

/// The bytes of entry `name` (case-insensitive, with extension), if the archive has it.
pub fn read_entry(img: &Path, name: &str) -> Option<Vec<u8>> {
    let mut f = File::open(img).ok()?;
    let mut head = [0u8; 8];
    f.read_exact(&mut head).ok()?;
    if &head[..4] != b"VER2" {
        return None;
    }
    let count = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let mut dir = vec![0u8; count.checked_mul(32)?];
    f.read_exact(&mut dir).ok()?;
    let e = dir.as_chunks::<32>().0.iter().find(|e| {
        let raw = &e[8..32];
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        raw[..end].eq_ignore_ascii_case(name.as_bytes())
    })?;
    let offset = u64::from(u32::from_le_bytes([e[0], e[1], e[2], e[3]]));
    let streaming = u16::from_le_bytes([e[4], e[5]]);
    let archive = u16::from_le_bytes([e[6], e[7]]);
    let sectors = u64::from(if streaming != 0 { streaming } else { archive });
    f.seek(SeekFrom::Start(offset * SECTOR)).ok()?;
    let mut out = vec![0u8; usize::try_from(sectors * SECTOR).ok()?];
    f.read_exact(&mut out).ok()?;
    Some(out)
}

/// A minimal archive with the given entries, for tests.
#[cfg(test)]
pub(crate) fn build(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let head = 8 + 32 * entries.len();
    let mut sector = (head as u64).div_ceil(SECTOR);
    let mut dir = b"VER2".to_vec();
    dir.extend((entries.len() as u32).to_le_bytes());
    let mut body = vec![];
    for (name, data) in entries {
        let n = (data.len() as u64).div_ceil(SECTOR).max(1);
        dir.extend((sector as u32).to_le_bytes());
        dir.extend((n as u16).to_le_bytes());
        dir.extend(0u16.to_le_bytes());
        let mut raw = [0u8; 24];
        raw[..name.len()].copy_from_slice(name.as_bytes());
        dir.extend(raw);
        let mut d = data.to_vec();
        d.resize((n * SECTOR) as usize, 0);
        body.extend(d);
        sector += n;
    }
    dir.resize(head.div_ceil(SECTOR as usize) * SECTOR as usize, 0);
    [dir, body].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_entries_by_name_ignoring_case() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("player.img");
        std::fs::write(&p, build(&[("vest.txd", b"abc"), ("Tshirt.dff", b"xyz")])).unwrap();
        assert_eq!(&read_entry(&p, "tshirt.DFF").unwrap()[..3], b"xyz");
        assert_eq!(read_entry(&p, "vest.txd").unwrap().len(), 2048);
        assert!(read_entry(&p, "nope.txd").is_none());
        assert!(read_entry(&d.path().join("missing.img"), "vest.txd").is_none());
    }
}
