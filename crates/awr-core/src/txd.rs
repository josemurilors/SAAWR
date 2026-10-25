//! Clothing `.txd`s: one uncompressed 32-bit texture (what both the game's and Proper Player
//! Retex's clothes use). Layout: TXD (0x16) > struct (texture count) > texture native (0x15) >
//! struct = platform, filter, name[32], mask[32], raster format, D3D format, width, height,
//! depth, levels, type, flags, then each level's size and pixels; extensions follow.
//!
//! The game blends clothes onto CJ's skin texture pixel by pixel, so a new item must be the
//! same size as the clothing textures in use.

/// Offsets from the file start (single-texture TXD); `*_SIZE` are chunk size fields.
const TXD_SIZE: usize = 4;
const COUNT: usize = 24;
const NATIVE_SIZE: usize = 32;
const STRUCT_SIZE: usize = 44;
const RASTER: usize = 124;
const WIDTH: usize = 132;
const DEPTH: usize = 136;
const LEVELS: usize = 137;
const DATA_SIZE: usize = 140;
const DATA: usize = 144;

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// Size of the (first) texture.
pub fn size(txd: &[u8]) -> Option<(u16, u16)> {
    if u32_at(txd, 0)? != 0x16 || u32_at(txd, NATIVE_SIZE - 4)? != 0x15 {
        return None;
    }
    Some((u16_at(txd, WIDTH)?, u16_at(txd, WIDTH + 2)?))
}

/// True when [`resize`] can handle it: one texture, 32-bit 8888/888, one level, uncompressed.
pub fn resizable(txd: &[u8]) -> bool {
    let ok = || -> Option<bool> {
        let (w, h) = size(txd)?;
        let raster = u32_at(txd, RASTER)? & 0xF00;
        Some(
            u16_at(txd, COUNT)? == 1
                && matches!(raster, 0x500 | 0x600)
                && txd.get(DEPTH) == Some(&32)
                && txd.get(LEVELS) == Some(&1)
                && u32_at(txd, DATA_SIZE)? as usize == usize::from(w) * usize::from(h) * 4,
        )
    };
    ok() == Some(true)
}

/// The texture scaled to `w`×`h` (area average when shrinking, bilinear when growing), with
/// every chunk size updated. None when it is not [`resizable`].
pub fn resize(txd: &[u8], w: u16, h: u16) -> Option<Vec<u8>> {
    if !resizable(txd) || w == 0 || h == 0 {
        return None;
    }
    let (ow, oh) = size(txd)?;
    let old_len = usize::from(ow) * usize::from(oh) * 4;
    let pixels = scale(
        txd.get(DATA..DATA + old_len)?,
        ow.into(),
        oh.into(),
        w.into(),
        h.into(),
    );
    let delta = pixels.len() as i64 - old_len as i64;
    let mut out = txd[..DATA].to_vec();
    out.extend(&pixels);
    out.extend(txd.get(DATA + old_len..)?);
    for at in [TXD_SIZE, NATIVE_SIZE, STRUCT_SIZE] {
        let new = u32::try_from(i64::from(u32_at(&out, at)?) + delta).ok()?;
        out[at..at + 4].copy_from_slice(&new.to_le_bytes());
    }
    out[WIDTH..WIDTH + 2].copy_from_slice(&w.to_le_bytes());
    out[WIDTH + 2..WIDTH + 4].copy_from_slice(&h.to_le_bytes());
    out[DATA_SIZE..DATA].copy_from_slice(&(pixels.len() as u32).to_le_bytes());
    Some(out)
}

/// Resamples 4-byte pixels (any channel order: each channel is treated the same).
fn scale(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<u8> {
    let px = |x: usize, y: usize, c: usize| f32::from(src[(y * sw + x) * 4 + c]);
    let mut out = vec![0u8; dw * dh * 4];
    for y in 0..dh {
        for x in 0..dw {
            for c in 0..4 {
                let v = if dw <= sw && dh <= sh {
                    // the source rectangle this pixel covers
                    let (x0, x1) = (
                        x * sw / dw,
                        ((x + 1) * sw).div_ceil(dw).max(x * sw / dw + 1),
                    );
                    let (y0, y1) = (
                        y * sh / dh,
                        ((y + 1) * sh).div_ceil(dh).max(y * sh / dh + 1),
                    );
                    let mut sum = 0.0;
                    for sy in y0..y1 {
                        for sx in x0..x1 {
                            sum += px(sx, sy, c);
                        }
                    }
                    sum / ((x1 - x0) * (y1 - y0)) as f32
                } else {
                    let fx = ((x as f32 + 0.5) * sw as f32 / dw as f32 - 0.5)
                        .clamp(0.0, (sw - 1) as f32);
                    let fy = ((y as f32 + 0.5) * sh as f32 / dh as f32 - 0.5)
                        .clamp(0.0, (sh - 1) as f32);
                    let (x0, y0) = (fx as usize, fy as usize);
                    let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
                    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
                    let top = px(x0, y0, c) * (1.0 - tx) + px(x1, y0, c) * tx;
                    let bottom = px(x0, y1, c) * (1.0 - tx) + px(x1, y1, c) * tx;
                    top * (1.0 - ty) + bottom * ty
                };
                out[(y * dw + x) * 4 + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// A single-texture TXD like the game's clothes, for tests.
#[cfg(test)]
pub(crate) fn sample(w: u16, h: u16, fill: impl Fn(usize) -> [u8; 4]) -> Vec<u8> {
    let pixels: Vec<u8> = (0..usize::from(w) * usize::from(h))
        .flat_map(fill)
        .collect();
    let ext = [3u32, 0, 0x1803_FFFF];
    let mut native_struct = vec![9, 0, 0, 0, 0x06, 0x11, 0, 0];
    let mut name = [0u8; 64];
    name[..4].copy_from_slice(b"vest");
    native_struct.extend(name);
    native_struct.extend(0x500u32.to_le_bytes());
    native_struct.extend(0x15u32.to_le_bytes());
    native_struct.extend(w.to_le_bytes());
    native_struct.extend(h.to_le_bytes());
    native_struct.extend([32, 1, 4, 1]);
    native_struct.extend((pixels.len() as u32).to_le_bytes());
    native_struct.extend(&pixels);
    let chunk = |t: u32, body: &[u8]| -> Vec<u8> {
        [
            &t.to_le_bytes()[..],
            &(body.len() as u32).to_le_bytes(),
            &0x1803_FFFFu32.to_le_bytes(),
            body,
        ]
        .concat()
    };
    let ext_bytes: Vec<u8> = ext.iter().flat_map(|v| v.to_le_bytes()).collect();
    let native = chunk(
        0x15,
        &[chunk(1, &native_struct), ext_bytes.clone()].concat(),
    );
    chunk(0x16, &[chunk(1, &[1, 0, 9, 0]), native, ext_bytes].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_scales_up_and_down_keeping_the_chunks_valid() {
        let t = sample(4, 2, |i| [i as u8 * 10, 0, 0, 255]);
        assert_eq!(size(&t), Some((4, 2)));
        assert!(resizable(&t));
        let big = resize(&t, 16, 8).unwrap();
        assert_eq!(size(&big), Some((16, 8)));
        assert_eq!(big.len(), t.len() + (16 * 8 - 4 * 2) * 4);
        assert_eq!(
            u32_at(&big, TXD_SIZE).unwrap() as usize,
            big.len() - 12,
            "TXD chunk size"
        );
        assert_eq!(u32_at(&big, DATA_SIZE), Some(16 * 8 * 4));
        // corners keep their colour, alpha stays opaque
        assert_eq!(big[DATA], 0);
        assert!(big[DATA..DATA + 16 * 8 * 4].chunks(4).all(|p| p[3] == 255));
        let back = resize(&big, 4, 2).unwrap();
        assert_eq!(back.len(), t.len());
        assert_eq!(size(&back), Some((4, 2)));
        // a uniform texture stays uniform both ways
        let flat = sample(8, 8, |_| [10, 20, 30, 255]);
        let small = resize(&flat, 2, 2).unwrap();
        assert!(small[DATA..DATA + 16]
            .chunks(4)
            .all(|p| p == [10, 20, 30, 255]));
    }

    #[test]
    fn compressed_or_odd_textures_are_left_alone() {
        let mut t = sample(4, 4, |_| [0; 4]);
        t[DEPTH] = 16; // e.g. DXT
        assert!(!resizable(&t));
        assert!(resize(&t, 8, 8).is_none());
        assert_eq!(size(b"not a txd"), None);
    }

    #[test]
    fn real_retex_and_original_clothes_have_this_layout() {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"));
        let Ok(t) = std::fs::read(dir.join("tshirtwhite.txd")) else {
            eprintln!("pulado: fixtures/tshirtwhite.txd ausente");
            return;
        };
        assert!(resizable(&t));
        let (w, h) = size(&t).unwrap();
        let other = resize(&t, w * 2, h).unwrap();
        assert_eq!(size(&other), Some((w * 2, h)));
    }
}
