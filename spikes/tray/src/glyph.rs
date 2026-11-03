// Shared by build.rs (icon resources) and the app (toast icon PNG), so both draw the same glyph.
// A placeholder shape only: icon design is Phase 5.

/// Glyph colour for a dark taskbar.
pub const ON_DARK: [u8; 3] = [0xff, 0xff, 0xff];
/// Glyph colour for a light taskbar.
pub const ON_LIGHT: [u8; 3] = [0x1c, 0x1c, 0x1c];

/// Straight alpha RGBA, top-down rows. `filled` is the "holding" look, the ring is "idle".
pub fn glyph_rgba(size: u32, filled: bool, rgb: [u8; 3]) -> Vec<u8> {
    // 4x4 supersampling gives clean edges at 16 px without an image library.
    const N: u32 = 4;
    let s = size as f32;
    let centre = s / 2.0;
    let outer = s * 0.42;
    let inner = outer - (s / 7.0).max(1.5);
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut hits = 0;
            for sy in 0..N {
                for sx in 0..N {
                    let px = x as f32 + (sx as f32 + 0.5) / N as f32 - centre;
                    let py = y as f32 + (sy as f32 + 0.5) / N as f32 - centre;
                    let d = (px * px + py * py).sqrt();
                    if d <= outer && (filled || d >= inner) {
                        hits += 1;
                    }
                }
            }
            let alpha = (hits * 255 / (N * N)) as u8;
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    out
}
