//! Deterministic pixel identicons.
//!
//! Two generators, both producing an [`Icon`] (a square grid of palette indices + three colours):
//!
//! * [`blockies`] – port of the classic `ethereum-blockies` algorithm (MetaMask's "Blockies"
//!   identicon). It reproduces the JavaScript number semantics exactly, so output matches the
//!   browser byte for byte, including non-ASCII seeds.
//! * [`pixel_id`] – "PixelID v1", the algorithm implemented on-chain by `src/PixelAvatar.sol`.
//!
//! [`Icon::to_svg`] emits the exact markup used by `web/index.html` and the Solidity renderer.

use std::fmt::Write as _;

use tiny_keccak::{Hasher, Keccak};

/// A square pixel grid. `cells[y * size + x]`: `0` = background, `1` = foreground, anything else = spot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub size: usize,
    pub cells: Vec<u8>,
    /// CSS colours in the order `[foreground, background, spot]`.
    pub colors: [String; 3],
}

impl Icon {
    /// Colour index used for a cell value: 0 = fg, 1 = bg, 2 = spot.
    fn color_of(v: u8) -> usize {
        match v {
            0 => 1,
            1 => 0,
            _ => 2,
        }
    }

    /// SVG markup with one `<rect>` per non-background cell.
    pub fn to_svg(&self) -> String {
        let n = self.size;
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {n} {n}" shape-rendering="crispEdges"><rect width="{n}" height="{n}" fill="{}"/>"#,
            self.colors[1]
        );
        for (i, &v) in self.cells.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let _ = write!(
                out,
                r#"<rect x="{}" y="{}" width="1" height="1" fill="{}"/>"#,
                i % n,
                i / n,
                self.colors[Self::color_of(v)]
            );
        }
        out.push_str("</svg>");
        out
    }

    /// RGBA8 pixels at an integer `scale` (each cell becomes a `scale`×`scale` block).
    pub fn to_rgba(&self, scale: usize) -> (usize, Vec<u8>) {
        let scale = scale.max(1);
        let side = self.size * scale;
        let palette: Vec<[u8; 3]> = self
            .colors
            .iter()
            .map(|c| parse_hsl(c).unwrap_or([0, 0, 0]))
            .collect();
        let mut buf = vec![0u8; side * side * 4];
        for y in 0..side {
            for x in 0..side {
                let v = self.cells[(y / scale) * self.size + x / scale];
                let rgb = palette[Self::color_of(v)];
                let o = (y * side + x) * 4;
                buf[o..o + 3].copy_from_slice(&rgb);
                buf[o + 3] = 255;
            }
        }
        (side, buf)
    }

    /// PNG bytes at an integer `scale`.
    #[cfg(feature = "png")]
    pub fn to_png(&self, scale: usize) -> Result<Vec<u8>, png::EncodingError> {
        let (side, rgba) = self.to_rgba(scale);
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, side as u32, side as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header()?;
            w.write_image_data(&rgba)?;
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------------------------
// Blockies
// ---------------------------------------------------------------------------------------------

/// xorshift128 seeded the way `ethereum-blockies` does it, with JS `ToInt32` semantics.
struct BlockiesRng {
    /// Seeds can exceed the int32 range after seeding (JS keeps them as doubles); i64 is exact.
    s: [i64; 4],
}

impl BlockiesRng {
    fn new(seed: &str) -> Self {
        let mut s = [0i64; 4];
        // JS iterates UTF-16 code units (`charCodeAt`).
        for (i, unit) in seed.encode_utf16().enumerate() {
            let r = s[i % 4];
            s[i % 4] = i64::from((r as i32).wrapping_shl(5)) - r + i64::from(unit);
        }
        BlockiesRng { s }
    }

    fn next(&mut self) -> f64 {
        let s0 = self.s[0] as i32;
        let t = s0 ^ s0.wrapping_shl(11);
        let s3 = self.s[3] as i32;
        self.s[0] = self.s[1];
        self.s[1] = self.s[2];
        self.s[2] = self.s[3];
        let v = s3 ^ (s3 >> 19) ^ t ^ (t >> 8);
        self.s[3] = i64::from(v);
        f64::from(v as u32) / 2_147_483_648.0
    }

    fn color(&mut self) -> String {
        let h = (self.next() * 360.0).floor();
        let s = self.next() * 60.0 + 40.0;
        let l = (self.next() + self.next() + self.next() + self.next()) * 25.0;
        format!("hsl({},{}%,{}%)", js_num(h), js_num(s), js_num(l))
    }
}

/// Blockies identicon. Seed with the lowercase address (`0xabc…`) for MetaMask parity.
pub fn blockies(seed: &str, size: usize) -> Icon {
    let size = size.max(1);
    let mut rng = BlockiesRng::new(seed);
    let fg = rng.color();
    let bg = rng.color();
    let spot = rng.color();
    let data_width = size.div_ceil(2);
    let mirror_width = size - data_width;
    let mut cells = Vec::with_capacity(size * size);
    for _ in 0..size {
        let row: Vec<u8> = (0..data_width)
            .map(|_| (rng.next() * 2.3).floor() as u8)
            .collect();
        cells.extend_from_slice(&row);
        cells.extend(row[..mirror_width].iter().rev());
    }
    Icon {
        size,
        cells,
        colors: [fg, bg, spot],
    }
}

/// Formats an f64 like JavaScript's `Number.prototype.toString` for the value ranges used here.
fn js_num(v: f64) -> String {
    // Both languages print the shortest round-trip digits. JS switches to exponent notation
    // below 1e-6 ("1.5e-7"), which Rust's `{:e}` matches; Blockies lightness only lands there
    // with negligible probability.
    if v != 0.0 && v.abs() < 1e-6 {
        return format!("{v:e}");
    }
    format!("{v}")
}

// ---------------------------------------------------------------------------------------------
// PixelID v1
// ---------------------------------------------------------------------------------------------

/// PixelID v1 for a 20-byte address. Identical to `PixelAvatar.svg(address)` in Solidity.
pub fn pixel_id(address: &[u8; 20]) -> Icon {
    let h = keccak256(address);
    let c = keccak256(&h);
    let light = [35 + h[9] % 25, 82 + h[10] % 12, 50 + h[11] % 25];
    let colors = std::array::from_fn(|i| {
        let hue = ((u16::from(h[2 * i]) << 8) | u16::from(h[2 * i + 1])) % 360;
        let sat = 40 + h[6 + i] % 50;
        format!("hsl({hue},{sat}%,{}%)", light[i])
    });
    let mut cells = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..4 {
            let v = c[y * 4 + x] % 100;
            let cell = if v < 40 {
                0
            } else if v < 82 {
                1
            } else {
                2
            };
            cells[y * 8 + x] = cell;
            cells[y * 8 + (7 - x)] = cell;
        }
    }
    Icon {
        size: 8,
        cells,
        colors,
    }
}

/// Parses `0x` + 40 hex characters (any case, checksum not enforced).
pub fn parse_address(s: &str) -> Option<[u8; 20]> {
    let body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))?;
    if body.len() != 40 {
        return None;
    }
    let mut out = [0u8; 20];
    for (i, pair) in body.as_bytes().chunks(2).enumerate() {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out[i] = (hi * 16 + lo) as u8;
    }
    Some(out)
}

pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256();
    k.update(data);
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    out
}

/// `hsl(h,s%,l%)` → sRGB, using the CSS Color 4 conversion with round-half-up, like browsers.
pub fn parse_hsl(css: &str) -> Option<[u8; 3]> {
    let inner = css.strip_prefix("hsl(")?.strip_suffix(')')?;
    let mut parts = inner
        .split(',')
        .map(|p| p.trim().trim_end_matches('%').parse::<f64>());
    let (h, s, l) = (
        parts.next()?.ok()?,
        parts.next()?.ok()? / 100.0,
        parts.next()?.ok()? / 100.0,
    );
    let f = |n: f64| {
        let k = (n + h / 30.0) % 12.0;
        let a = s * l.min(1.0 - l);
        let v = l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    Some([f(0.0), f(8.0), f(4.0)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsl_primary_colours() {
        assert_eq!(parse_hsl("hsl(0,100%,50%)"), Some([255, 0, 0]));
        assert_eq!(parse_hsl("hsl(120,100%,50%)"), Some([0, 255, 0]));
        assert_eq!(parse_hsl("hsl(240,100%,50%)"), Some([0, 0, 255]));
        assert_eq!(parse_hsl("hsl(0,0%,100%)"), Some([255, 255, 255]));
    }

    #[test]
    fn pixel_id_is_symmetric_and_three_coloured() {
        let icon = pixel_id(&parse_address("0x000000000000000000000000000000000000dEaD").unwrap());
        for y in 0..8 {
            for x in 0..4 {
                assert_eq!(icon.cells[y * 8 + x], icon.cells[y * 8 + 7 - x]);
            }
        }
        assert!(icon.colors.iter().all(|c| c.starts_with("hsl(")));
    }

    #[test]
    fn rgba_scales_exactly() {
        let icon = blockies("gm", 8);
        let (side, px) = icon.to_rgba(4);
        assert_eq!(side, 32);
        assert_eq!(px.len(), 32 * 32 * 4);
        // Top-left 4x4 block is uniform.
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(px[(y * 32 + x) * 4..][..4], px[..4]);
            }
        }
    }

    #[test]
    fn parse_address_rejects_garbage() {
        assert!(parse_address("0x123").is_none());
        assert!(parse_address("0xzz0000000000000000000000000000000000dead").is_none());
    }
}
