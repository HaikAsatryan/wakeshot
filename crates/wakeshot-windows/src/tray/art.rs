//! Icon art as code: the held light as tray glyphs and as the app icon, and the ICO and PNG writers.
//!
//! Owner: `p5-icon-art`, then `p5-haik-check`, `p6-brand-cleanup` and `p6-brand-apply-art`.
//!
//! No Windows API here, so the file could move to the core. Shapes and colours use plain arithmetic and `sqrt` only,
//! never `sin`, `cos`, `exp`, `powf` or `ln`: those may differ in the last bit between x64 and ARM64, and the golden
//! tests compare exactly. The soft shadow is integer box blurs, exact everywhere.
//!
//! The mark is the held light in Moss (Haik, 2026-11-05): a round light held over a line. In the tray the light is
//! filled while holding, the same light as an outline when idle, and the outline struck through when off, as Windows
//! marks its own off states. The tray icon draws `tray_mask` at the size the taskbar wants and colours it with
//! `colorize`: the line and the outline in Windows' glyph colour, the light in Moss while holding, every part in the
//! text colour in high contrast. The app icon is the same mark lit, on a Moss plate. This file is the one source of the
//! drawing: its writer makes the ICO, the notice picture, the favicon and the page's SVG icon and lockups, and the
//! media's port (`media/src/art/heldLight.ts`) must draw the same pixels (`media/scripts/check-art.mjs`). The brand:
//! `docs/brand.md`; what was measured and how: `docs/research.md`, "Brand apply: the held light in art.rs" and "Brand
//! apply review fixes".

use wakeshot_core::view::IconState;

/// A tray glyph: alpha only, `size` by `size`, top-down rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub size: u32,
    /// The whole glyph.
    pub alpha: Vec<u8>,
    /// The part of `alpha` that is the filled light, which takes its own colour while holding. Never above `alpha`.
    pub light: Vec<u8>,
}

/// Straight (not premultiplied) RGBA, top-down rows, 4 bytes per pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The colours of a tray glyph: `ink` for the line and the outline, `light` for the filled light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphColours {
    pub ink: [u8; 3],
    pub light: [u8; 3],
}

impl GlyphColours {
    /// Every part in one colour, as high contrast asks.
    pub const fn one(rgb: [u8; 3]) -> Self {
        Self {
            ink: rgb,
            light: rgb,
        }
    }
}

/// The sizes Windows asks for in the tray, 100% to 400% scale.
pub const TRAY_SIZES: [u32; 10] = [16, 20, 24, 28, 32, 36, 40, 48, 56, 64];

/// The frames of `assets/wakeshot.ico`: every target size in Microsoft's "Construct your Windows app's icon" (updated
/// 2026-08-05), so Windows finds an exact frame for the title bar, tray, taskbar, Start and its lists at every scale
/// from 100% to 400% instead of scaling a larger one down. The 256 frame is a PNG, the rest 32-bit bitmaps, as
/// Microsoft's icon guide asks.
pub const APP_ICON_SIZES: [u32; 14] = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];

/// The frames of `site/favicon.ico`: a browser tab at 100% and 200%, and 48 for the places that want a larger one.
pub const FAVICON_SIZES: [u32; 3] = [16, 32, 48];

// Measured in this laptop's screenshots (research "Tray UX: icon art"). The taskbar is translucent, so its colour
// moves a few steps with the wallpaper.

/// Windows' own tray glyph colour on a dark taskbar.
pub const GLYPH_ON_DARK: [u8; 3] = [0xff, 0xff, 0xff];
/// Windows' own glyph colour on a light background (its glyphs there match black at alpha 0xe4; solid 1a1a1a looks the
/// same and does not need a second alpha).
pub const GLYPH_ON_LIGHT: [u8; 3] = [0x1a, 0x1a, 0x1a];
/// The light while holding, on a dark taskbar: Moss's bright green (`docs/brand.md`).
pub const HOLDING_ON_DARK: [u8; 3] = [0xb6, 0xe0, 0x5a];
/// The light while holding, on a light taskbar: Moss's deep green.
pub const HOLDING_ON_LIGHT: [u8; 3] = [0x3b, 0x7a, 0x2a];
/// The dark taskbar behind the tray.
pub const TASKBAR_DARK: [u8; 3] = [0x20, 0x20, 0x21];
/// The light taskbar behind the tray.
pub const TASKBAR_LIGHT: [u8; 3] = [0xd8, 0xda, 0xdc];
/// The plate under a tray icon the mouse rests on, dark taskbar.
pub const HOVER_DARK: [u8; 3] = [0x2d, 0x2d, 0x2e];
/// The plate under a tray icon the mouse rests on, light taskbar.
pub const HOVER_LIGHT: [u8; 3] = [0xec, 0xed, 0xee];

/// The glyph on a dark taskbar: white, the light in Moss while holding.
pub const ON_DARK: GlyphColours = GlyphColours {
    ink: GLYPH_ON_DARK,
    light: HOLDING_ON_DARK,
};
/// The glyph on a light taskbar: near black, the light in deep Moss while holding.
pub const ON_LIGHT: GlyphColours = GlyphColours {
    ink: GLYPH_ON_LIGHT,
    light: HOLDING_ON_LIGHT,
};

// The app icon's colours: Moss (`docs/research/brand-directions-2.md`, the Moss column; Haik's pick 2026-11-05).

/// The plate's ramp runs from this leaf green at the top left to `PLATE_DARK` at the bottom right.
const PLATE_LIGHT: [u8; 3] = [0x62, 0x8c, 0x2b];
const PLATE_DARK: [u8; 3] = [0x3e, 0x7c, 0x3b];
/// The shade that deepens the plate's bottom.
const PLATE_DEEP: [u8; 3] = [0x2c, 0x5a, 0x26];
/// The light is near white in the middle and warm at its edge; its glow on the plate is the edge colour.
const LIGHT_CORE: [u8; 3] = [0xff, 0xfb, 0xe8];
const LIGHT_EDGE: [u8; 3] = [0xff, 0xea, 0x9c];
/// The lit light's rim, round 2's `#F7DC7F`, so the lit light keeps a warm edge.
const LIGHT_RIM: [u8; 3] = [0xf7, 0xdc, 0x7f];
const WHITE: [u8; 3] = [0xff; 3];

/// From this size up the app icon is lit: a glow round the light on the plate, a bloom, a shade at the bottom and a
/// drop shadow. Below it the frames are flat drawings of the same mark, since blur and shadows turn to mud there
/// (Microsoft's old icon guide: no shadows at 24 px or smaller).
const LIT_FROM: u32 = 30;

/// The empty border a tray glyph keeps on each side, in pixels: 1 px at 100% scale, 4 px at 400%.
pub fn padding(size: u32) -> u32 {
    (size / 16).max(1)
}

/// Stroke width in whole pixels, about 1 px per 16 px of icon as Windows' own glyphs, so a straight stroke covers
/// whole pixels.
fn stroke(size: u32) -> u32 {
    (size as f32 / 16.0).round().max(1.0) as u32
}

/// The held light at one tray size, in whole pixels.
#[derive(Clone, Copy, Debug)]
struct Held {
    /// The stroke of the line and of the outline.
    t: u32,
    /// The light's diameter and its top.
    d: u32,
    top: u32,
    /// The line's top, left and right.
    line_y: u32,
    x0: u32,
    x1: u32,
}

impl Held {
    fn at(size: u32) -> Self {
        let t = stroke(size);
        // Half the box, of the box's parity, so both sides of the light land on pixel edges.
        let mut d = size / 2;
        if (d + size) % 2 == 1 {
            d -= 1;
        }
        let gap = (size + 4) / 8;
        let width = size - 2 * (size / 8);
        // The light goes where the filled glyph's ink centre sits at 0.455 of the box, level with the next app's icon
        // on the taskbar (`brand-review.md` T4). The outline keeps that place, so the light does not jump between
        // states; its own ink centre sits lower, since a ring weighs less than a disc.
        let (fd, ft) = (d as f32, t as f32);
        let disc = std::f32::consts::FRAC_PI_4 * fd * fd;
        let line = (width * t) as f32;
        let below = (disc * fd / 2.0 + line * (fd + gap as f32 + ft / 2.0)) / (disc + line);
        let top = (0.455 * size as f32 - below).round().max(0.0) as u32;
        let x0 = (size - width) / 2;
        Held {
            t,
            d,
            top,
            line_y: top + d + gap,
            x0,
            x1: x0 + width,
        }
    }

    fn line(self) -> Shape {
        let t = self.t as f32;
        Shape::Rect {
            x0: self.x0 as f32,
            y0: self.line_y as f32,
            x1: self.x1 as f32,
            y1: (self.line_y + self.t) as f32,
            // Round ends once the stroke is 2 px, as the app icon's bar has.
            r: [if self.t >= 2 { t / 2.0 } else { 0.0 }; 4],
        }
    }

    fn disc(self, size: u32, inset: u32) -> Shape {
        let r = self.d as f32 / 2.0;
        Shape::Disc {
            cx: size as f32 / 2.0,
            cy: self.top as f32 + r,
            r: r - inset as f32,
        }
    }

    /// The outline drawn by hand pixel by pixel, where there is one for this light.
    fn hand_ring(self) -> Option<&'static [&'static str]> {
        match (self.t, self.d) {
            (1, 8) => Some(&RING_8),
            (1, 10) => Some(&RING_10),
            _ => None,
        }
    }
}

/// The 1 px outlines of the 8 and 10 px lights (16 and 20 px glyphs), drawn by hand: a curve on its own leaves grey
/// pixels that make the ring look fainter than the line (`brand-review.md` T2). Whole pixels (`X`), and a
/// quarter-covered pixel (`s`) softening each step so it still reads round.
const RING_8: [&str; 8] = [
    ".sXXXXs.", "sX....Xs", "X......X", "X......X", "X......X", "X......X", "sX....Xs", ".sXXXXs.",
];
const RING_10: [&str; 10] = [
    "..sXXXXs..",
    ".sX....Xs.",
    "sX......Xs",
    "X........X",
    "X........X",
    "X........X",
    "X........X",
    "sX......Xs",
    ".sX....Xs.",
    "..sXXXXs..",
];

/// The tray glyph for `state`.
pub fn tray_mask(state: IconState, size: u32) -> Mask {
    let h = Held::at(size);
    let n = (size * size) as usize;
    let outline = Shape::Any(vec![h.line(), minus(h.disc(size, 0), h.disc(size, h.t))]);
    let (mut alpha, light) = match state {
        IconState::Holding => (
            raster(size, size, &h.line()),
            raster(size, size, &h.disc(size, 0)),
        ),
        IconState::Idle => (raster(size, size, &outline), vec![0; n]),
        IconState::Off if h.t >= 2 => (raster(size, size, &struck(h, size, outline)), vec![0; n]),
        IconState::Off => (raster(size, size, &outline), vec![0; n]),
    };
    if state != IconState::Holding {
        if let Some(rows) = h.hand_ring() {
            let left = (size - h.d) / 2;
            for (j, row) in rows.iter().enumerate() {
                for (i, c) in row.bytes().enumerate() {
                    let at = ((h.top + j as u32) * size + left + i as u32) as usize;
                    alpha[at] = match c {
                        b'X' => 255,
                        b's' => 64,
                        _ => 0,
                    };
                }
            }
        }
        if state == IconState::Off && h.t == 1 {
            strike_pixels(&mut alpha, h, size);
        }
    }
    // The light and the line never share a pixel: a gap of 2 px at least runs between them.
    for (a, l) in alpha.iter_mut().zip(&light) {
        *a = a.saturating_add(*l);
    }
    Mask { size, alpha, light }
}

/// The square the off slash crosses: from the light's top left corner through its centre, down to a stroke above the
/// line. So the slash strikes the light, stays inside the glyph's own box (`brand-review.md` T7) and leaves the line
/// whole: a slash that ran on to the line's end cut a gap into it, which read as the slash running through the line
/// (the brand screen session, 20 px).
fn strike_box(h: Held, size: u32) -> (u32, u32, u32, u32) {
    let left = (size - h.d) / 2;
    let bottom = h.line_y - h.t;
    (left, h.top, left + (bottom - h.top), bottom)
}

/// Off at 2 px strokes and up: the outline struck through from top left to bottom right, with a stroke-wide gap on
/// each side of the slash, as Windows draws its own off glyphs.
fn struck(h: Held, size: u32, base: Shape) -> Shape {
    let t = h.t as f32;
    let (x0, y0, x1, y1) = strike_box(h, size);
    let slash = |w: f32| Shape::Line {
        ax: x0 as f32 + t / 2.0,
        ay: y0 as f32 + t / 2.0,
        bx: x1 as f32 - t / 2.0,
        by: y1 as f32 - t / 2.0,
        w,
    };
    Shape::Any(vec![minus(base, slash(3.0 * t)), slash(t)])
}

/// Off at 1 px strokes: a 1 px slash at 45 degrees is a row of grey pairs when drawn as a line, so it is whole pixels
/// stepping down the diagonal. Every pixel touching a step, corners included, is cleared as the gap: with only the
/// four beside each step cleared, the ring and the line still touched the slash at its corners.
fn strike_pixels(alpha: &mut [u8], h: Held, size: u32) {
    let (x0, y0, _, y1) = strike_box(h, size);
    let steps = y1 - y0;
    let mut set = |x: u32, y: u32, v: u8| {
        if x < size && y < size {
            alpha[(y * size + x) as usize] = v;
        }
    };
    for i in 0..steps {
        let (x, y) = (x0 + i, y0 + i);
        set(x + 1, y, 0);
        set(x, y + 1, 0);
        set(x.wrapping_sub(1), y, 0);
        set(x, y.wrapping_sub(1), 0);
        set(x + 1, y.wrapping_sub(1), 0);
        set(x.wrapping_sub(1), y + 1, 0);
    }
    for i in 0..steps {
        set(x0 + i, y0 + i, 255);
    }
}

/// The glyph in its colours; the caller picks them from the theme.
pub fn colorize(mask: &Mask, colours: GlyphColours) -> Rgba {
    let pixels = mask
        .alpha
        .iter()
        .zip(&mask.light)
        .flat_map(|(&a, &l)| {
            let rgb = if l == 0 {
                colours.ink
            } else if l >= a {
                colours.light
            } else {
                let (a, l) = (u32::from(a), u32::from(l));
                let one = |ink: u8, light: u8| {
                    ((l * u32::from(light) + (a - l) * u32::from(ink) + a / 2) / a) as u8
                };
                [
                    one(colours.ink[0], colours.light[0]),
                    one(colours.ink[1], colours.light[1]),
                    one(colours.ink[2], colours.light[2]),
                ]
            };
            [rgb[0], rgb[1], rgb[2], a]
        })
        .collect();
    Rgba {
        width: mask.size,
        height: mask.size,
        pixels,
    }
}

/// The app icon's layout at one size, in pixels: the plate, the light and the bar under it.
#[derive(Clone, Copy, Debug)]
struct Plated {
    size: u32,
    lit: bool,
    /// The plate's inset from the icon's edge, its side and its corner radius.
    margin: f32,
    side: f32,
    corner: f32,
    /// The light's centre and radius.
    cx: f32,
    cy: f32,
    r: f32,
    /// The bar: left, top, right, bottom.
    bar: [f32; 4],
}

impl Plated {
    fn at(size: u32, lit: bool) -> Self {
        let s = size as f32;
        // A margin of size / 24 (2 px at 48), so the plate does not look larger than other apps' icons. A design
        // call: Microsoft's pages give no margin number.
        let margin = ((size + 12) / 24) as f32;
        let side = s - 2.0 * margin;
        let r = (s * 3.0 / 16.0).round();
        let bar_h = (s / 16.0).round().max(1.0);
        // At least the bar's height and a pixel, so the light never looks set on the bar.
        let gap = (s * 0.082).round().max(bar_h + 1.0);
        // The light and the bar sit a little above the middle, as a picture centred by eye does.
        let top = ((s - (2.0 * r + gap + bar_h)) / 2.0 - 0.014 * s).round();
        let bar_w = 2.0 * (s * 0.2735).round();
        let bar_y = top + 2.0 * r + gap;
        let bar_x = (s - bar_w) / 2.0;
        Plated {
            size,
            lit,
            margin,
            side,
            corner: side * 3.0 / 16.0,
            cx: s / 2.0,
            cy: top + r,
            r,
            bar: [bar_x, bar_y, bar_x + bar_w, bar_y + bar_h],
        }
    }

    fn plate(self, dy: f32) -> Shape {
        let far = self.size as f32 - self.margin;
        Shape::Rect {
            x0: self.margin,
            y0: self.margin + dy,
            x1: far,
            y1: far + dy,
            r: [self.corner; 4],
        }
    }

    fn bar_shape(self) -> Shape {
        let [x0, y0, x1, y1] = self.bar;
        Shape::Rect {
            x0,
            y0,
            x1,
            y1,
            r: [self.bar_corner(); 4],
        }
    }

    /// Round ends once the bar is 2 px high.
    fn bar_corner(self) -> f32 {
        let h = self.bar[3] - self.bar[1];
        if h >= 2.0 { h / 2.0 } else { 0.0 }
    }

    fn light_shape(self) -> Shape {
        Shape::Disc {
            cx: self.cx,
            cy: self.cy,
            r: self.r,
        }
    }

    /// The plate's colour at a point, without the bloom: the ramp, then when lit the light's glow and the shade at the
    /// bottom.
    fn plate_colour(self, x: f32, y: f32) -> [f32; 3] {
        let (m, w) = (self.margin, self.side);
        // Microsoft's default gradient angle, 120 degrees: lightest at the top left, as if lit from there.
        let (dx, dy) = (3f32.sqrt() / 2.0, 0.5);
        let along = ((x - m) * dx + (y - m) * dy) / (w * (dx + dy));
        let mut rgb = mixf(f(PLATE_LIGHT), f(PLATE_DARK), along.clamp(0.0, 1.0));
        if self.lit {
            let (reach, glow) = self.glow();
            let t = self.distance(x, y) / (reach * w);
            rgb = mixf(rgb, f(LIGHT_EDGE), stops(&glow, t));
            let down = (y - m) / w;
            let shade = (SHADE * (down - SHADE_FROM) / (1.0 - SHADE_FROM)).clamp(0.0, SHADE);
            rgb = mixf(rgb, f(PLATE_DEEP), shade);
        }
        rgb
    }

    /// The glow round the light: how far it reaches, as a share of the plate's side, and its alpha along that reach.
    /// Kept modest below 64 px: the glow lightens the plate, and Microsoft asks that at least half the icon keep 3:1
    /// on light Start, measured at 48 px.
    fn glow(self) -> (f32, [(f32, f32); 3]) {
        let (reach, strength) = if self.size >= 64 {
            (0.62, 0.6)
        } else {
            (0.5, 0.42)
        };
        (
            reach,
            [(0.0, strength), (0.45, 0.35 * strength), (1.0, 0.0)],
        )
    }

    fn distance(self, x: f32, y: f32) -> f32 {
        let (dx, dy) = (x - self.cx, y - self.cy);
        (dx * dx + dy * dy).sqrt()
    }

    /// The light's colour at a point: a radial ramp from a spot up and left of the centre, near white over its middle
    /// and warm only at the edge, so it reads as lit rather than as a ball.
    fn light_colour(self, x: f32, y: f32) -> [f32; 3] {
        let r = self.r;
        let (fx, fy) = self.focus();
        let (vx, vy) = (x - fx, y - fy);
        let (ex, ey) = (self.cx - fx, self.cy - fy);
        // The ramp's circles run from the spot (t = 0) to the light's edge (t = 1); solve for the one through (x, y).
        let a = ex * ex + ey * ey - r * r;
        let b = vx * ex + vy * ey;
        let c = vx * vx + vy * vy;
        let t = ((b - (b * b - a * c).sqrt()) / a).clamp(0.0, 1.0);
        if self.lit {
            let [core, edge] = LIT_RAMP;
            let rgb = mixf(
                f(LIGHT_CORE),
                f(LIGHT_EDGE),
                ((t - core) / (edge - core)).clamp(0.0, 1.0),
            );
            // 0.14 written out: 1.0 - 0.86 in f32 is one bit off it, and the ports copy these sums exactly.
            mixf(rgb, f(LIGHT_RIM), ((t - edge) / 0.14).clamp(0.0, 1.0))
        } else {
            mixf(
                f(LIGHT_CORE),
                f(LIGHT_EDGE),
                ((t - 0.6) / 0.4).clamp(0.0, 1.0),
            )
        }
    }

    /// The spot the light's ramp starts from, up and left of its centre.
    fn focus(self) -> (f32, f32) {
        (self.cx - 0.22 * self.r, self.cy - 0.28 * self.r)
    }

    /// The light's soft bloom round its edge, as an alpha.
    fn bloom(self, x: f32, y: f32) -> f32 {
        stops(&BLOOM, self.distance(x, y) / self.r)
    }

    /// The drop shadow's alpha under the plate: the plate moved down by a 96th of the icon and blurred to a deviation
    /// of a 64th, so most of it fades inside the margin (5% black on the icon's last row at 48 px, 1% at 256). Round
    /// 2's larger shadow (a 48th down, deviation 1.2 px at 48 px) ran past the icon's edge and was cut there, 9% black
    /// on the last row, a hard grey line under the icon on light Start.
    fn shadow(self) -> Vec<u8> {
        let mut alpha = raster(self.size, self.size, &self.plate(self.shadow_drop()));
        let (width, passes) = self.shadow_blur();
        for _ in 0..passes {
            box_blur(&mut alpha, self.size, width);
        }
        alpha
    }

    fn shadow_drop(self) -> f32 {
        self.size as f32 / 96.0
    }

    /// The shadow's box blur, its width and passes. Box blurs one after another come close to a Gaussian: a 3 px box
    /// adds a variance of 2/3, so small shadows take that box as often as their variance asks, and large ones three
    /// wider boxes.
    fn shadow_blur(self) -> (u32, u32) {
        let sigma = self.size as f32 / 64.0;
        if sigma < 1.2 {
            (3, ((1.5 * sigma * sigma).round() as u32).max(1))
        } else {
            let w = ((4.0 * sigma * sigma + 1.0).sqrt() / 2.0).round() as u32 * 2 + 1;
            (w, 3)
        }
    }
}

/// How dark the drop shadow is at its darkest.
const SHADOW: f32 = 0.3;

/// The lit plate's shade deepens it from this share of its height down to its bottom, where it reaches `SHADE`.
const SHADE_FROM: f32 = 0.55;
const SHADE: f32 = 0.35;

/// The lit light's ramp from its focus (0) to its edge (1): the core colour up to the first stop, the edge colour at
/// the second, the rim at the edge.
const LIT_RAMP: [f32; 2] = [0.5, 0.86];

/// The light's bloom, alpha by distance from its centre in light radii: round 2's bloom, 0.44 at the light's edge,
/// with its tail ending at `BLOOM_END` instead of 1.9. The longer tail lay over the plate just above the bar and took
/// the bar under 3:1 from 64 px up.
const BLOOM: [(f32, f32); 4] = [(0.0, 0.8), (1.0, 0.44), (1.28, 0.144), (BLOOM_END, 0.0)];

/// Where the bloom ends. The bar's top sits 1.42 to 1.5 radii out, where the bloom is down to 0.08 or less, so the
/// bar keeps 3:1 against the plate above it as drawn.
const BLOOM_END: f32 = 1.6;

/// The app icon: the held light, lit, over its bar on a Moss plate. Frames below `LIT_FROM` are flat.
pub fn app_icon(size: u32) -> Rgba {
    draw_plated(Plated::at(size, size >= LIT_FROM))
}

/// The app icon drawn flat at any size: the plate's ramp, the light and the bar, with no glow, bloom, shade or
/// shadow. The notice header's picture, which the shell scales down to about 16 logical px.
pub fn app_icon_flat(size: u32) -> Rgba {
    draw_plated(Plated::at(size, false))
}

fn draw_plated(p: Plated) -> Rgba {
    let size = p.size;
    let plate = raster(size, size, &p.plate(0.0));
    let bar = raster(size, size, &p.bar_shape());
    let light = raster(size, size, &p.light_shape());
    let shadow = if p.lit {
        p.shadow()
    } else {
        vec![0; (size * size) as usize]
    };
    let [bar_x0, _, bar_x1, _] = p.bar;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) as usize;
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let cover = |m: &[u8]| f32::from(m[i]) / 255.0;
            let mut px = Paint::CLEAR;
            px.over([0.0; 3], SHADOW * cover(&shadow));
            px.over(p.plate_colour(fx, fy), cover(&plate));
            let along = ((fx - bar_x0) / (bar_x1 - bar_x0)).clamp(0.0, 1.0);
            px.over(mixf(f(WHITE), f(LIGHT_CORE), along), cover(&bar));
            if p.lit {
                px.over(f(LIGHT_EDGE), p.bloom(fx, fy) * cover(&plate));
            }
            px.over(p.light_colour(fx, fy), cover(&light));
            pixels.extend(px.bytes());
        }
    }
    Rgba {
        width: size,
        height: size,
        pixels,
    }
}

/// A pixel being painted: straight colour, 0 to 255, and alpha, 0 to 1.
#[derive(Clone, Copy)]
struct Paint {
    rgb: [f32; 3],
    a: f32,
}

impl Paint {
    const CLEAR: Paint = Paint {
        rgb: [0.0; 3],
        a: 0.0,
    };

    /// Lays `rgb` at alpha `a` over the pixel, as a browser does in sRGB.
    fn over(&mut self, rgb: [f32; 3], a: f32) {
        if a <= 0.0 {
            return;
        }
        let under = self.a * (1.0 - a);
        let out = a + under;
        for (c, top) in self.rgb.iter_mut().zip(rgb) {
            *c = (top * a + *c * under) / out;
        }
        self.a = out;
    }

    fn bytes(self) -> [u8; 4] {
        let a = (self.a * 255.0).round() as u8;
        if a == 0 {
            return [0; 4];
        }
        let c = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        [c(self.rgb[0]), c(self.rgb[1]), c(self.rgb[2]), a]
    }
}

fn f(rgb: [u8; 3]) -> [f32; 3] {
    [f32::from(rgb[0]), f32::from(rgb[1]), f32::from(rgb[2])]
}

fn mixf(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// A value along straight segments between `(t, value)` stops, flat past both ends.
fn stops(points: &[(f32, f32)], t: f32) -> f32 {
    let (first, last) = (points[0], points[points.len() - 1]);
    if t <= first.0 {
        return first.1;
    }
    for pair in points.windows(2) {
        let ((t0, v0), (t1, v1)) = (pair[0], pair[1]);
        if t <= t1 {
            return v0 + (v1 - v0) * (t - t0) / (t1 - t0);
        }
    }
    last.1
}

/// One box blur pass across, then one down, `width` pixels wide (odd); outside the image counts as clear.
fn box_blur(alpha: &mut [u8], size: u32, width: u32) {
    let (n, half) = (size as usize, (width / 2) as usize);
    let mut line = vec![0u8; n];
    for across in [true, false] {
        for k in 0..n {
            let at = |j: usize| if across { k * n + j } else { j * n + k };
            for (j, slot) in line.iter_mut().enumerate() {
                let (lo, hi) = (j.saturating_sub(half), (j + half).min(n - 1));
                let sum: u32 = (lo..=hi).map(|q| u32::from(alpha[at(q)])).sum();
                *slot = ((sum + width / 2) / width) as u8;
            }
            for (j, v) in line.iter().enumerate() {
                alpha[at(j)] = *v;
            }
        }
    }
}

/// The bytes of `assets/wakeshot.ico`.
pub fn app_icon_file() -> Vec<u8> {
    let frames: Vec<Rgba> = APP_ICON_SIZES.iter().map(|&s| app_icon(s)).collect();
    ico(&frames)
}

/// The bytes of `site/favicon.ico`: the same art as the app icon, so the page's tab shows the app.
pub fn favicon_file() -> Vec<u8> {
    let frames: Vec<Rgba> = FAVICON_SIZES.iter().map(|&s| app_icon(s)).collect();
    ico(&frames)
}

/// One shape of a glyph, in pixels of the canvas, y down.
#[derive(Clone, Debug)]
enum Shape {
    Disc {
        cx: f32,
        cy: f32,
        r: f32,
    },
    /// Corner radii top left, top right, bottom right, bottom left.
    Rect {
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        r: [f32; 4],
    },
    /// Round caps.
    Line {
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        w: f32,
    },
    Any(Vec<Shape>),
    Minus(Box<Shape>, Box<Shape>),
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Shape::Disc { cx, cy, r } => {
                let (dx, dy) = (x - cx, y - cy);
                dx * dx + dy * dy <= r * r
            }
            Shape::Rect { x0, y0, x1, y1, r } => {
                if x < *x0 || x > *x1 || y < *y0 || y > *y1 {
                    return false;
                }
                let (left, top) = (x < (x0 + x1) / 2.0, y < (y0 + y1) / 2.0);
                let r = match (top, left) {
                    (true, true) => r[0],
                    (true, false) => r[1],
                    (false, false) => r[2],
                    (false, true) => r[3],
                };
                let cx = if left { x0 + r } else { x1 - r };
                let cy = if top { y0 + r } else { y1 - r };
                let in_corner =
                    (if left { x < cx } else { x > cx }) && (if top { y < cy } else { y > cy });
                let (dx, dy) = (x - cx, y - cy);
                !in_corner || dx * dx + dy * dy <= r * r
            }
            Shape::Line { ax, ay, bx, by, w } => {
                let (vx, vy) = (bx - ax, by - ay);
                let len2 = vx * vx + vy * vy;
                let t = if len2 > 0.0 {
                    (((x - ax) * vx + (y - ay) * vy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (dx, dy) = (x - (ax + t * vx), y - (ay + t * vy));
                dx * dx + dy * dy <= w * w / 4.0
            }
            Shape::Any(all) => all.iter().any(|s| s.contains(x, y)),
            Shape::Minus(a, b) => a.contains(x, y) && !b.contains(x, y),
        }
    }
}

fn minus(a: Shape, b: Shape) -> Shape {
    Shape::Minus(Box::new(a), Box::new(b))
}

/// Samples per pixel on each axis: 64 per pixel give smooth curves, and sample points never sit on a pixel edge, so a
/// straight edge on a whole pixel covers it fully or not at all.
const SAMPLES: u32 = 8;

fn raster(width: u32, height: u32, shape: &Shape) -> Vec<u8> {
    let n = SAMPLES * SAMPLES;
    let step = 1.0 / SAMPLES as f32;
    let mut out = Vec::with_capacity((width * height) as usize);
    for py in 0..height {
        for px in 0..width {
            let mut hits = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = px as f32 + (sx as f32 + 0.5) * step;
                    let y = py as f32 + (sy as f32 + 0.5) * step;
                    hits += u32::from(shape.contains(x, y));
                }
            }
            out.push(((hits * 255 + n / 2) / n) as u8);
        }
    }
    out
}

/// An ICO file of 32-bit frames in the order given, each square, 1 to 256 px. A 256 frame is stored as a PNG and the
/// others as bitmaps with an AND mask, as Microsoft's icon guide asks ("only the 256x256 pixel image should be
/// compressed").
pub fn ico(frames: &[Rgba]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = frames
        .iter()
        .map(|f| if f.width == 256 { png_rgba(f) } else { dib(f) })
        .collect();
    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend((frames.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * frames.len();
    for (frame, image) in frames.iter().zip(&images) {
        // A side of 256 is written as 0.
        out.extend([frame.width as u8, frame.height as u8, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((image.len() as u32).to_le_bytes());
        out.extend((offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in images {
        out.extend(image);
    }
    out
}

/// One ICO frame: a BITMAPINFOHEADER of twice the height, bottom-up BGRA, then the AND mask.
fn dib(img: &Rgba) -> Vec<u8> {
    let (w, h) = (img.width, img.height);
    assert!(
        w == h && (1..=256).contains(&w),
        "an ICO frame is square, 1 to 256 px: {w}x{h}"
    );
    let mask_row = w.div_ceil(32) * 4;
    let mut out = Vec::with_capacity((40 + w * h * 4 + mask_row * h) as usize);
    out.extend(40u32.to_le_bytes());
    out.extend(w.to_le_bytes());
    out.extend((2 * h).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend((w * h * 4 + mask_row * h).to_le_bytes());
    out.extend([0u8; 16]);
    let pixel = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        &img.pixels[i..i + 4]
    };
    for y in (0..h).rev() {
        for x in 0..w {
            let p = pixel(x, y);
            // A pixel the mask hides keeps no colour, so an old renderer that XORs it leaves the screen as it is.
            if p[3] == 0 {
                out.extend([0, 0, 0, 0]);
            } else {
                out.extend([p[2], p[1], p[0], p[3]]);
            }
        }
    }
    for y in (0..h).rev() {
        let mut row = vec![0u8; mask_row as usize];
        for x in 0..w {
            if pixel(x, y)[3] == 0 {
                row[(x / 8) as usize] |= 0x80 >> (x % 8);
            }
        }
        out.extend(row);
    }
    out
}

/// A PNG file, 8 bits per channel: a palette when the image has at most 256 colours (the review sheets), else RGBA.
pub fn png(img: &Rgba) -> Vec<u8> {
    match palette(img) {
        Some((colours, indices)) => {
            let mut out = png_header(img, 3);
            let rgb: Vec<u8> = colours.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
            chunk(&mut out, b"PLTE", &rgb);
            if colours.iter().any(|c| c[3] < 255) {
                let alpha: Vec<u8> = colours.iter().map(|c| c[3]).collect();
                chunk(&mut out, b"tRNS", &alpha);
            }
            png_end(out, &scanlines(&indices, img.width as usize, 1))
        }
        None => png_rgba(img),
    }
}

/// A PNG file in RGBA whatever the colours: the form an ICO's 256 frame takes.
fn png_rgba(img: &Rgba) -> Vec<u8> {
    let out = png_header(img, 6);
    png_end(out, &scanlines(&img.pixels, img.width as usize * 4, 4))
}

fn png_header(img: &Rgba, colour_type: u8) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::with_capacity(13);
    header.extend(img.width.to_be_bytes());
    header.extend(img.height.to_be_bytes());
    header.extend([8, colour_type, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    out
}

fn png_end(mut out: Vec<u8>, filtered: &[u8]) -> Vec<u8> {
    chunk(&mut out, b"IDAT", &zlib(filtered));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// The image's colours in first-seen order and each pixel's index, or `None` past 256 colours.
fn palette(img: &Rgba) -> Option<(Vec<[u8; 4]>, Vec<u8>)> {
    let mut colours: Vec<[u8; 4]> = Vec::new();
    let mut seen = std::collections::HashMap::new();
    let mut indices = Vec::with_capacity(img.pixels.len() / 4);
    for p in img.pixels.chunks(4) {
        let c = [p[0], p[1], p[2], p[3]];
        let i = match seen.get(&c) {
            Some(&i) => i,
            None => {
                let i = u8::try_from(colours.len()).ok()?;
                colours.push(c);
                seen.insert(c, i);
                i
            }
        };
        indices.push(i);
    }
    Some((colours, indices))
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend(kind);
    out.extend(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

/// Each row with the filter that leaves the smallest bytes (libpng's rule): Sub turns a flat run into zeros, Up a
/// repeated row, Paeth most of a gradient.
fn scanlines(data: &[u8], stride: usize, bpp: usize) -> Vec<u8> {
    let rows = data.len().checked_div(stride).unwrap_or(0);
    let mut out = Vec::with_capacity((stride + 1) * rows);
    let zero = vec![0u8; stride];
    let mut previous: &[u8] = &zero;
    for row in data.chunks(stride.max(1)) {
        let left = |i: usize| if i >= bpp { row[i - bpp] } else { 0 };
        let corner = |i: usize| if i >= bpp { previous[i - bpp] } else { 0 };
        let filters: [Vec<u8>; 5] = [
            row.to_vec(),
            (0..stride).map(|i| row[i].wrapping_sub(left(i))).collect(),
            (0..stride)
                .map(|i| row[i].wrapping_sub(previous[i]))
                .collect(),
            (0..stride)
                .map(|i| {
                    row[i].wrapping_sub(((u16::from(left(i)) + u16::from(previous[i])) / 2) as u8)
                })
                .collect(),
            (0..stride)
                .map(|i| row[i].wrapping_sub(paeth(left(i), previous[i], corner(i))))
                .collect(),
        ];
        let cost = |v: &Vec<u8>| {
            v.iter()
                .map(|&b| u32::from((b as i8).unsigned_abs()))
                .sum::<u32>()
        };
        let (kind, bytes) = filters
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| cost(v))
            .unwrap_or((0, &filters[0]));
        out.push(kind as u8);
        out.extend(bytes);
        previous = row;
    }
    out
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn zlib(data: &[u8]) -> Vec<u8> {
    // Deflate, 32 KB window, no dictionary, best compression; 0x78da is a multiple of 31 as the header check needs.
    let mut out = vec![0x78, 0xda];
    out.extend(deflate(data));
    out.extend(adler32(data).to_be_bytes());
    out
}

#[derive(Default)]
struct BitWriter {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl BitWriter {
    /// Deflate packs values from the least significant bit.
    fn put(&mut self, value: u32, count: u32) {
        self.acc |= value << self.n;
        self.n += count;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    /// Huffman codes go most significant bit first.
    fn code(&mut self, code: u32, len: u8) {
        let len = u32::from(len);
        self.put(code.reverse_bits() >> (32 - len), len);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

// RFC 1951 section 3.2.5: the base and extra bits of each length and distance code.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// The order the code length code's lengths are sent in (RFC 1951 section 3.2.7).
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

const WINDOW: usize = 32 * 1024;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
/// How many earlier places with the same three bytes a match search tries: zlib's level 9 tries 4096; 128 gets within
/// a few percent of it on these images at a fraction of the time.
const MAX_CHAIN: usize = 128;
/// Symbols per block, so each part of an image gets its own codes.
const BLOCK: usize = 16 * 1024;

#[derive(Clone, Copy, Debug)]
enum Token {
    Literal(u8),
    Match { len: u16, dist: u16 },
}

/// Deflate (RFC 1951): matches found through hash chains with one step of lazy matching, as zlib does; each block in
/// the fixed codes or its own, whichever is smaller. Row filters leave images as small differences, which the codes
/// pack, and flat areas and repeated rows as matches.
fn deflate(data: &[u8]) -> Vec<u8> {
    let tokens = lz77(data);
    let mut w = BitWriter::default();
    let blocks: Vec<&[Token]> = if tokens.is_empty() {
        vec![&[]]
    } else {
        tokens.chunks(BLOCK).collect()
    };
    for (i, block) in blocks.iter().enumerate() {
        write_block(&mut w, block, i + 1 == blocks.len());
    }
    w.finish()
}

struct Matcher<'a> {
    data: &'a [u8],
    head: Vec<u32>,
    prev: Vec<u32>,
}

const HASH_BITS: u32 = 15;
const NONE: u32 = u32::MAX;

impl Matcher<'_> {
    fn hash(&self, i: usize) -> usize {
        let d = self.data;
        let v = u32::from(d[i]) << 16 | u32::from(d[i + 1]) << 8 | u32::from(d[i + 2]);
        (v.wrapping_mul(0x9e37_79b1) >> (32 - HASH_BITS)) as usize
    }

    fn insert(&mut self, i: usize) {
        if i + MIN_MATCH <= self.data.len() {
            let h = self.hash(i);
            self.prev[i] = self.head[h];
            self.head[h] = i as u32;
        }
    }

    /// The longest earlier match of the bytes at `i`, as length and distance; length 0 when there is none of
    /// `MIN_MATCH` or more.
    fn longest(&self, i: usize) -> (usize, usize) {
        let d = self.data;
        if i + MIN_MATCH > d.len() {
            return (0, 0);
        }
        let limit = (d.len() - i).min(MAX_MATCH);
        let (mut best, mut dist) = (0, 0);
        let mut at = self.head[self.hash(i)];
        let mut tries = MAX_CHAIN;
        while at != NONE && tries > 0 {
            let j = at as usize;
            if i - j > WINDOW {
                break;
            }
            let len = d[j..]
                .iter()
                .zip(&d[i..i + limit])
                .take_while(|(a, b)| a == b)
                .count();
            if len > best {
                (best, dist) = (len, i - j);
                if len == limit {
                    break;
                }
            }
            at = self.prev[j];
            tries -= 1;
        }
        if best >= MIN_MATCH {
            (best, dist)
        } else {
            (0, 0)
        }
    }
}

fn lz77(data: &[u8]) -> Vec<Token> {
    let mut m = Matcher {
        data,
        head: vec![NONE; 1 << HASH_BITS],
        prev: vec![NONE; data.len()],
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let (len, dist) = m.longest(i);
        m.insert(i);
        if len == 0 || m.longest(i + 1).0 > len {
            // No match, or a longer one starts at the next byte: this byte goes alone.
            out.push(Token::Literal(data[i]));
            i += 1;
            continue;
        }
        out.push(Token::Match {
            len: len as u16,
            dist: dist as u16,
        });
        for j in i + 1..i + len {
            m.insert(j);
        }
        i += len;
    }
    out
}

/// The code of `value` in a base table: its index, extra bits' value and their count.
fn base_code(value: u16, base: &[u16], extra: &[u8]) -> (usize, u32, u32) {
    let k = base.iter().rposition(|&b| b <= value).unwrap_or(0);
    (k, u32::from(value - base[k]), u32::from(extra[k]))
}

fn write_block(w: &mut BitWriter, tokens: &[Token], last: bool) {
    let mut lit = [0u32; 286];
    let mut dist = [0u32; 30];
    for t in tokens {
        match *t {
            Token::Literal(b) => lit[usize::from(b)] += 1,
            Token::Match { len, dist: d } => {
                lit[257 + base_code(len, &LENGTH_BASE, &LENGTH_EXTRA).0] += 1;
                dist[base_code(d, &DIST_BASE, &DIST_EXTRA).0] += 1;
            }
        }
    }
    lit[256] = 1;
    let fixed_lit: Vec<u8> = (0..288)
        .map(|s| match s {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        })
        .collect();
    let fixed_dist = vec![5u8; 30];
    let lit_len = code_lengths(&lit, 15);
    let dist_len = code_lengths(&dist, 15);
    let header = dynamic_header(&lit_len, &dist_len);
    let cost = |ll: &[u8], dl: &[u8]| -> u64 {
        let l: u64 = lit
            .iter()
            .zip(ll)
            .map(|(&c, &n)| u64::from(c) * u64::from(n))
            .sum();
        let d: u64 = dist
            .iter()
            .zip(dl)
            .map(|(&c, &n)| u64::from(c) * u64::from(n))
            .sum();
        l + d
    };
    let dynamic = header.bits + cost(&lit_len, &dist_len) < cost(&fixed_lit, &fixed_dist);
    w.put(u32::from(last), 1);
    let (ll, dl) = if dynamic {
        w.put(2, 2);
        header.write(w);
        (lit_len, dist_len)
    } else {
        w.put(1, 2);
        (fixed_lit, fixed_dist)
    };
    let (lc, dc) = (canonical(&ll), canonical(&dl));
    for t in tokens {
        match *t {
            Token::Literal(b) => w.code(lc[usize::from(b)], ll[usize::from(b)]),
            Token::Match { len, dist: d } => {
                let (k, v, n) = base_code(len, &LENGTH_BASE, &LENGTH_EXTRA);
                w.code(lc[257 + k], ll[257 + k]);
                w.put(v, n);
                let (k, v, n) = base_code(d, &DIST_BASE, &DIST_EXTRA);
                w.code(dc[k], dl[k]);
                w.put(v, n);
            }
        }
    }
    w.code(lc[256], ll[256]);
}

/// A dynamic block's header: how many codes, then all code lengths, run-length coded by the code length code.
struct Header {
    lit: usize,
    dist: usize,
    order: usize,
    cl_len: Vec<u8>,
    runs: Vec<(usize, u32, u32)>,
    bits: u64,
}

impl Header {
    fn write(&self, w: &mut BitWriter) {
        w.put((self.lit - 257) as u32, 5);
        w.put((self.dist - 1) as u32, 5);
        w.put((self.order - 4) as u32, 4);
        for &s in &CODE_LENGTH_ORDER[..self.order] {
            w.put(u32::from(self.cl_len[s]), 3);
        }
        let codes = canonical(&self.cl_len);
        for &(sym, value, count) in &self.runs {
            w.code(codes[sym], self.cl_len[sym]);
            w.put(value, count);
        }
    }
}

fn dynamic_header(lit_len: &[u8], dist_len: &[u8]) -> Header {
    let used = |l: &[u8], min: usize| {
        l.iter()
            .rposition(|&n| n > 0)
            .map_or(min, |i| (i + 1).max(min))
    };
    let (lit, dist) = (used(lit_len, 257), used(dist_len, 1));
    let all: Vec<u8> = lit_len[..lit]
        .iter()
        .chain(&dist_len[..dist])
        .copied()
        .collect();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < all.len() {
        let v = all[i];
        let run = all[i..].iter().take_while(|&&x| x == v).count();
        if v == 0 && run >= 3 {
            let r = run.min(138);
            if r >= 11 {
                runs.push((18, (r - 11) as u32, 7));
            } else {
                runs.push((17, (r - 3) as u32, 3));
            }
            i += r;
        } else if v != 0 && run >= 4 {
            runs.push((usize::from(v), 0, 0));
            let mut left = run - 1;
            while left >= 3 {
                let r = left.min(6);
                runs.push((16, (r - 3) as u32, 2));
                left -= r;
            }
            for _ in 0..left {
                runs.push((usize::from(v), 0, 0));
            }
            i += run;
        } else {
            runs.push((usize::from(v), 0, 0));
            i += 1;
        }
    }
    let mut freq = [0u32; 19];
    for &(s, _, _) in &runs {
        freq[s] += 1;
    }
    let cl_len = code_lengths(&freq, 7);
    let order = CODE_LENGTH_ORDER
        .iter()
        .rposition(|&s| cl_len[s] > 0)
        .map_or(4, |i| (i + 1).max(4));
    let bits = 14
        + 3 * order as u64
        + runs
            .iter()
            .map(|&(s, _, n)| u64::from(cl_len[s]) + u64::from(n))
            .sum::<u64>();
    Header {
        lit,
        dist,
        order,
        cl_len,
        runs,
        bits,
    }
}

/// Huffman code lengths of at most `max` bits for the symbols' counts. When the tree runs deeper, the counts are
/// halved (each kept above 0) and the tree built again, which flattens it.
fn code_lengths(counts: &[u32], max: u32) -> Vec<u8> {
    let mut c = counts.to_vec();
    // Two symbols at least, so the code is complete, which every inflater accepts; the extra one is never sent.
    let mut missing = 2usize.saturating_sub(c.iter().filter(|&&n| n > 0).count());
    for n in c.iter_mut() {
        if missing > 0 && *n == 0 {
            *n = 1;
            missing -= 1;
        }
    }
    loop {
        let lengths = huffman(&c);
        if lengths.iter().all(|&l| l <= max) {
            return lengths.iter().map(|&l| l as u8).collect();
        }
        for n in c.iter_mut().filter(|n| **n > 0) {
            *n = n.div_ceil(2);
        }
    }
}

/// Each symbol's depth in a Huffman tree, by the two-queue method: leaves sorted by count, inner nodes in the order
/// made. Ties go to the leaf, so the result is the same on every run.
fn huffman(counts: &[u32]) -> Vec<u32> {
    let mut leaves: Vec<usize> = (0..counts.len()).filter(|&s| counts[s] > 0).collect();
    leaves.sort_by_key(|&s| (counts[s], s));
    let n = leaves.len();
    let mut weight: Vec<u64> = leaves.iter().map(|&s| u64::from(counts[s])).collect();
    let mut parent = vec![0usize; 2 * n - 1];
    let (mut leaf, mut inner) = (0, n);
    for node in n..2 * n - 1 {
        let mut pick = || {
            let take_leaf = leaf < n && (inner >= node || weight[leaf] <= weight[inner]);
            if take_leaf {
                leaf += 1;
                leaf - 1
            } else {
                inner += 1;
                inner - 1
            }
        };
        let (a, b) = (pick(), pick());
        weight.push(weight[a] + weight[b]);
        parent[a] = node;
        parent[b] = node;
    }
    let mut depth = vec![0u32; 2 * n - 1];
    for k in (0..2 * n - 2).rev() {
        depth[k] = depth[parent[k]] + 1;
    }
    let mut out = vec![0; counts.len()];
    for (k, &s) in leaves.iter().enumerate() {
        out[s] = depth[k];
    }
    out
}

/// The canonical codes for the lengths (RFC 1951 section 3.2.2).
fn canonical(lengths: &[u8]) -> Vec<u32> {
    let mut count = [0u32; 16];
    for &l in lengths {
        count[usize::from(l)] += 1;
    }
    count[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0;
    for bits in 1..16 {
        code = (code + count[bits - 1]) << 1;
        next[bits] = code;
    }
    lengths
        .iter()
        .map(|&l| {
            if l == 0 {
                return 0;
            }
            let c = next[usize::from(l)];
            next[usize::from(l)] += 1;
            c
        })
        .collect()
}

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in bytes {
        c = CRC_TABLE[((c ^ u32::from(b)) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    // 5552 bytes is the most that can be summed before the 32-bit sums could overflow (zlib's NMAX).
    for part in bytes.chunks(5552) {
        for &x in part {
            a += u32::from(x);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
const STATES: [IconState; 3] = [IconState::Holding, IconState::Idle, IconState::Off];

#[cfg(test)]
fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let one = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    [one(a[0], b[0]), one(a[1], b[1]), one(a[2], b[2])]
}

/// Paints `img` (straight alpha) over the opaque `sheet`, enlarged `zoom` times by nearest neighbour.
#[cfg(test)]
fn paint(sheet: &mut Rgba, img: &Rgba, left: u32, top: u32, zoom: u32) {
    for y in 0..img.height * zoom {
        for x in 0..img.width * zoom {
            let s = (((y / zoom) * img.width + x / zoom) * 4) as usize;
            let d = (((top + y) * sheet.width + left + x) * 4) as usize;
            let a = f32::from(img.pixels[s + 3]) / 255.0;
            let under = [sheet.pixels[d], sheet.pixels[d + 1], sheet.pixels[d + 2]];
            let rgb = mix(
                under,
                [img.pixels[s], img.pixels[s + 1], img.pixels[s + 2]],
                a,
            );
            sheet.pixels[d..d + 3].copy_from_slice(&rgb);
        }
    }
}

#[cfg(test)]
fn blank(width: u32, height: u32, rgb: [u8; 3]) -> Rgba {
    Rgba {
        width,
        height,
        pixels: [rgb[0], rgb[1], rgb[2], 255].repeat((width * height) as usize),
    }
}

/// Rows: holding in one colour (the shape), holding in its colours, idle and off. Columns: 16, 20, 24, 32, 40 and 48
/// px at real size, then the same enlarged 8 times. `media/scripts/check-art.mjs` reads the glyphs at these places,
/// so a layout change goes there too.
#[cfg(test)]
fn sheet(back: [u8; 3], colours: GlyphColours) -> Rgba {
    const SIZES: [u32; 6] = [16, 20, 24, 32, 40, 48];
    const ZOOM: u32 = 8;
    const GAP: u32 = 16;
    const MARGIN: u32 = 32;
    let rows = [
        (IconState::Holding, GlyphColours::one(colours.ink)),
        (IconState::Holding, colours),
        (IconState::Idle, colours),
        (IconState::Off, colours),
    ];
    let row = 48 * ZOOM;
    let real: u32 = SIZES.iter().map(|s| s + GAP).sum();
    let zoomed: u32 = SIZES.iter().map(|s| s * ZOOM + GAP).sum();
    let width = MARGIN + real + MARGIN + zoomed - GAP + MARGIN;
    let height = MARGIN + rows.len() as u32 * (row + GAP) - GAP + MARGIN;
    let mut out = blank(width, height, back);
    let mut top = MARGIN;
    for (state, ink) in rows {
        let mut left = MARGIN;
        for size in SIZES {
            let glyph = colorize(&tray_mask(state, size), ink);
            paint(&mut out, &glyph, left, top + (row - size) / 2, 1);
            left += size + GAP;
        }
        left += MARGIN;
        for size in SIZES {
            let glyph = colorize(&tray_mask(state, size), ink);
            paint(&mut out, &glyph, left, top + (row - size * ZOOM) / 2, ZOOM);
            left += size * ZOOM + GAP;
        }
        top += row + GAP;
    }
    out
}

/// Rows: the app icon on light and on dark Start. Columns: every frame at real size, then the small frames, where
/// each pixel is drawn by hand or by rule, enlarged 4 times.
#[cfg(test)]
fn app_sheet() -> Rgba {
    const ZOOMED: [u32; 5] = [16, 20, 24, 32, 48];
    const ZOOM: u32 = 4;
    const GAP: u32 = 16;
    const MARGIN: u32 = 32;
    let row = 256 + 2 * MARGIN;
    let real: u32 = APP_ICON_SIZES.iter().map(|s| s + GAP).sum();
    let zoomed: u32 = ZOOMED.iter().map(|s| s * ZOOM + GAP).sum();
    let width = MARGIN + real + MARGIN + zoomed - GAP + MARGIN;
    let mut out = blank(width, 2 * row, [0; 3]);
    let mut top = 0;
    for back in [[0xf3; 3], [0x20; 3]] {
        paint(&mut out, &blank(width, row, back), 0, top, 1);
        let mut left = MARGIN;
        for size in APP_ICON_SIZES {
            paint(&mut out, &app_icon(size), left, top + (row - size) / 2, 1);
            left += size + GAP;
        }
        left += MARGIN;
        for size in ZOOMED {
            let icon = app_icon(size);
            paint(&mut out, &icon, left, top + (row - size * ZOOM) / 2, ZOOM);
            left += size * ZOOM + GAP;
        }
        top += row;
    }
    out
}

/// A number in an SVG: three decimals at most, no trailing zeros.
#[cfg(test)]
fn svg_num(v: f64) -> String {
    let text = format!("{v:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
fn svg_colour(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// The app icon at 256 px as SVG gradients, a blur and a plate mask (`defs`, ids `i-`), and the shapes that use them
/// (`body`): the layers of `draw_plated` in its order, from the same layout and constants, so the page, the README
/// and the social card draw the ICO's picture sharp at any size.
#[cfg(test)]
fn svg_icon_parts() -> (String, String) {
    let p = Plated::at(256, true);
    let n = |v: f32| svg_num(f64::from(v));
    let (m, w) = (f64::from(p.margin), f64::from(p.side));
    // `plate_colour`'s ramp: 0 at the plate's top left corner, 1 at its bottom right, along 120 degrees.
    let (dx, dy) = (3f64.sqrt() / 2.0, 0.5);
    let length = w * (dx + dy);
    let (reach, glow) = p.glow();
    let [x0, y0, x1, y1] = p.bar;
    let (fx, fy) = p.focus();
    let bloom_r = BLOOM_END * p.r;
    // `draw_plated` cuts the bloom at the plate's edge and an SVG gradient would not be cut, so it must end inside.
    let plate = p.plate(0.0);
    assert!(
        (0..64).all(|i| {
            let a = std::f32::consts::TAU * i as f32 / 64.0;
            plate.contains(p.cx + bloom_r * a.cos(), p.cy + bloom_r * a.sin())
        }),
        "the bloom reaches the plate's edge"
    );
    let (width, passes) = p.shadow_blur();
    let sigma = (f64::from(passes) * f64::from(width * width - 1) / 12.0).sqrt();
    let centre = format!("cx=\"{}\" cy=\"{}\"", n(p.cx), n(p.cy));
    let edge = svg_colour(LIGHT_EDGE);
    let faded = |id: &str, r: f32, at: &[(f32, f32)], unit: f32| {
        let mut out = format!(
            "<radialGradient id=\"i-{id}\" gradientUnits=\"userSpaceOnUse\" {centre} r=\"{}\">",
            n(r)
        );
        for (t, alpha) in at {
            out += &format!(
                "\n      <stop offset=\"{}\" stop-color=\"{edge}\" stop-opacity=\"{}\"/>",
                svg_num(f64::from(*t) / f64::from(unit)),
                n(*alpha)
            );
        }
        out + "\n    </radialGradient>"
    };
    let stop = |at: &str, rgb: [u8; 3]| {
        format!(
            "  <stop offset=\"{at}\" stop-color=\"{}\"/>",
            svg_colour(rgb)
        )
    };
    let defs = [
        format!(
            "<linearGradient id=\"i-ramp\" gradientUnits=\"userSpaceOnUse\" x1=\"{0}\" y1=\"{0}\" x2=\"{1}\" \
             y2=\"{2}\">",
            svg_num(m),
            svg_num(m + length * dx),
            svg_num(m + length * dy)
        ),
        stop("0", PLATE_LIGHT),
        stop("1", PLATE_DARK),
        "</linearGradient>".to_owned(),
        faded("glow", reach * p.side, &glow, 1.0),
        format!(
            "<linearGradient id=\"i-shade\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"{}\" x2=\"0\" y2=\"{}\">",
            svg_num(m + f64::from(SHADE_FROM) * w),
            svg_num(m + w)
        ),
        format!(
            "  <stop offset=\"0\" stop-color=\"{}\" stop-opacity=\"0\"/>",
            svg_colour(PLATE_DEEP)
        ),
        format!(
            "  <stop offset=\"1\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
            svg_colour(PLATE_DEEP),
            n(SHADE)
        ),
        "</linearGradient>".to_owned(),
        format!(
            "<linearGradient id=\"i-bar\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"0\">",
            n(x0),
            n(x1)
        ),
        stop("0", WHITE),
        stop("1", LIGHT_CORE),
        "</linearGradient>".to_owned(),
        faded("bloom", bloom_r, &BLOOM, BLOOM_END),
        format!(
            "<radialGradient id=\"i-light\" gradientUnits=\"userSpaceOnUse\" {centre} r=\"{}\" fx=\"{}\" fy=\"{}\">",
            n(p.r),
            n(fx),
            n(fy)
        ),
        stop(&n(LIT_RAMP[0]), LIGHT_CORE),
        stop(&n(LIT_RAMP[1]), LIGHT_EDGE),
        stop("1", LIGHT_RIM),
        "</radialGradient>".to_owned(),
        "<filter id=\"i-soft\" x=\"-10%\" y=\"-10%\" width=\"120%\" height=\"120%\">".to_owned(),
        format!("  <feGaussianBlur stdDeviation=\"{}\"/>", svg_num(sigma)),
        "</filter>".to_owned(),
        "<mask id=\"i-plate\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"256\" height=\"256\">".to_owned(),
        format!(
            "  <rect x=\"{0}\" y=\"{0}\" width=\"{1}\" height=\"{1}\" rx=\"{2}\" fill=\"#FFF\"/>",
            n(p.margin),
            n(p.side),
            n(p.corner)
        ),
        "</mask>".to_owned(),
    ]
    .join("\n    ");
    let body = [
        format!(
            "<rect x=\"{0}\" y=\"{1}\" width=\"{2}\" height=\"{2}\" rx=\"{3}\" fill=\"#000\" fill-opacity=\"{4}\" \
             filter=\"url(#i-soft)\"/>",
            n(p.margin),
            svg_num(m + f64::from(p.shadow_drop())),
            n(p.side),
            n(p.corner),
            n(SHADOW)
        ),
        "<g mask=\"url(#i-plate)\">".to_owned(),
        "  <rect width=\"256\" height=\"256\" fill=\"url(#i-ramp)\"/>".to_owned(),
        "  <rect width=\"256\" height=\"256\" fill=\"url(#i-glow)\"/>".to_owned(),
        "  <rect width=\"256\" height=\"256\" fill=\"url(#i-shade)\"/>".to_owned(),
        "</g>".to_owned(),
        format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"url(#i-bar)\"/>",
            n(x0),
            n(y0),
            n(x1 - x0),
            n(y1 - y0),
            n(p.bar_corner())
        ),
        format!("<circle {centre} r=\"{}\" fill=\"url(#i-bloom)\"/>", n(bloom_r)),
        format!("<circle {centre} r=\"{}\" fill=\"url(#i-light)\"/>", n(p.r)),
    ]
    .join("\n  ");
    (defs, body)
}

/// `site/wakeshot.svg`.
#[cfg(test)]
fn app_icon_svg() -> String {
    let (defs, body) = svg_icon_parts();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 256 256\" width=\"256\" height=\"256\">
  <!-- The Wakeshot app icon, the held light in Moss: the 256 px frame of assets/wakeshot.ico as shapes, so it stays
       sharp at any size. Written by crates/wakeshot-windows/src/tray/art.rs (write_art_files) from the ICO's layout
       and layers; change the art there, not here. -->
  <defs>
    {defs}
  </defs>
  {body}
</svg>
"
    )
}

/// "Wakeshot" in Instrument Sans SemiBold 1.000 as outlines, set beside the 256 px plate (margin 11, side 234): cap
/// height 0.46 of the plate's side and centred on it, the ink starting 0.26 of the side after the plate, tracking
/// -0.01em as on the page. `p6-brand-apply-web` shaped it with HarfBuzz and drew the font's outlines with fontTools
/// (its Log); the outlines change only with the font, those ratios or the plate, and are then made again that way.
#[cfg(test)]
const WORDMARK: &str = concat!(
    "M336.79 181.82 305.84 74.18H325.57L348.9 161.94L370.57 74.18H393L414.38 162.09",
    "L437.85 74.18H456.69L425.74 181.82H403.61L381.34 94.21L358.91 181.82ZM481.65 183.31",
    "Q470.14 183.31 463.34 177.56Q456.54 171.8 456.54 162.09Q456.54 151.62 464.16 145.49",
    "Q471.79 139.36 485.69 137.27L506.02 134.13V130.39Q506.02 118.28 492.42 118.28",
    "Q478.66 118.28 476.87 127.85H458.78Q459.97 117.24 468.87 110.66",
    "Q477.77 104.08 493.01 104.08Q508.56 104.08 516.71 111.18Q524.86 118.28 524.86 132.04",
    "V166.57Q524.86 173.9 526.95 181.82H508.11Q506.92 177.04 506.77 169.71",
    "Q499.74 183.31 481.65 183.31ZM475.37 159.99Q475.37 164.33 478.51 166.65",
    "Q481.65 168.96 487.18 168.96Q495.26 168.96 500.64 164.85Q506.02 160.74 506.02 155.06",
    "V146.24L488.83 148.78Q475.37 150.87 475.37 159.99ZM542.5 181.82V73.13H561.34V137.42",
    "L590.64 105.58H613.36L578.38 141.75L614.86 181.82H591.68L561.34 147.29V181.82Z",
    "M653.43 183.31Q641.32 183.31 632.27 178.23Q623.23 173.15 618.22 164.25",
    "Q613.21 155.36 613.21 143.55Q613.21 131.89 618.22 122.99Q623.23 114.1 632.2 109.09",
    "Q641.17 104.08 653.13 104.08Q664.49 104.08 672.86 108.79Q681.23 113.5 685.87 122.02",
    "Q690.5 130.54 690.5 141.9Q690.5 144.15 690.35 145.87Q690.2 147.58 689.91 149.38H632.35",
    "Q633.54 159.1 639 164.1Q644.46 169.11 653.58 169.11Q659.56 169.11 663.97 166.87",
    "Q668.38 164.63 670.62 159.99H688.41Q684.67 170.91 675.78 177.11",
    "Q666.88 183.31 653.43 183.31ZM652.83 118.28Q644.76 118.28 639.52 122.84",
    "Q634.29 127.4 632.8 136.22H671.37Q670.62 127.25 665.76 122.77Q660.9 118.28 652.83 118.28",
    "ZM732.66 183.31Q716.82 183.31 707.4 176.66Q697.98 170.01 697.08 158.2H714.27",
    "Q715.77 170.01 732.66 170.01Q747.31 170.01 747.31 161.04Q747.31 157.9 745.22 155.81",
    "Q743.13 153.71 737.3 152.52L725.04 149.68Q712.33 146.99 706.35 141.31",
    "Q700.37 135.62 700.37 126.8Q700.37 116.19 708.52 110.13Q716.67 104.08 731.02 104.08",
    "Q745.22 104.08 753.74 110.13Q762.26 116.19 763.31 126.8H746.12",
    "Q744.62 117.39 730.57 117.39Q717.56 117.39 717.56 125.31Q717.56 128.3 720.1 130.32",
    "Q722.65 132.34 728.63 133.83L741.78 136.82Q753.14 139.36 758.83 145.64",
    "Q764.51 151.92 764.51 160.44Q764.51 171.35 756.13 177.33Q747.76 183.31 732.66 183.31Z",
    "M775.87 181.82V74.18H794.71V118.73Q798.89 112 805.84 108.04Q812.79 104.08 822.06 104.08",
    "Q834.47 104.08 841.42 111.03Q848.38 117.98 848.38 129.79V181.82H829.39V133.68",
    "Q829.39 119.48 815.34 119.48Q806.22 119.48 800.46 125.38Q794.71 131.29 794.71 140.71",
    "V181.82ZM900.55 183.31Q888.44 183.31 879.25 178.23Q870.05 173.15 864.97 164.1",
    "Q859.89 155.06 859.89 143.4Q859.89 131.74 864.97 122.84Q870.05 113.95 879.17 109.01",
    "Q888.29 104.08 900.55 104.08Q912.81 104.08 921.93 109.01Q931.05 113.95 936.13 122.84",
    "Q941.22 131.74 941.22 143.4Q941.22 155.06 936.06 164.1Q930.9 173.15 921.78 178.23",
    "Q912.66 183.31 900.55 183.31ZM900.55 168.22Q909.52 168.22 915.5 161.41",
    "Q921.48 154.61 921.48 143.25Q921.48 132.04 915.58 125.61Q909.67 119.18 900.55 119.18",
    "Q891.43 119.18 885.45 125.61Q879.47 132.04 879.47 143.25Q879.47 154.61 885.45 161.41",
    "Q891.43 168.22 900.55 168.22ZM986.07 183.31Q972.01 183.31 965.36 176.74",
    "Q958.71 170.16 958.71 156.7V120.38H944.06V105.58H958.71V88.23L977.69 81.21V105.58H998.47",
    "V120.38H977.69V157.15Q977.69 168.07 990.1 168.07Q992.64 168.07 994.66 167.69",
    "Q996.68 167.32 998.47 166.72V181.37Q994.59 183.31 986.07 183.31Z",
);

/// Where the wordmark's ink ends on the right, in the same units.
#[cfg(test)]
const WORDMARK_RIGHT: f64 = 998.47;

/// `site/wakeshot-lockup-light.svg` and `-dark.svg`: the icon beside the wordmark, 80 px high, the word in the page's
/// text colour for that theme (`site/style.css`, `--text`).
#[cfg(test)]
fn lockup_svg(dark: bool) -> String {
    let p = Plated::at(256, true);
    assert_eq!(
        (p.margin, p.side),
        (11.0, 234.0),
        "the wordmark's outlines sit beside this plate: make them again"
    );
    let (defs, body) = svg_icon_parts();
    let (theme, ink) = if dark {
        ("dark", "#EEF2E8")
    } else {
        ("light", "#1A1D17")
    };
    // The shadow's blur reaches about three deviations past the plate: keep it inside the picture.
    let pad = 16.0;
    let (m, side) = (f64::from(p.margin), f64::from(p.side));
    let (left, top, height) = (m - pad, m - pad, side + 2.0 * pad + 2.0);
    let px_high = 80.0;
    let px_wide = ((WORDMARK_RIGHT + pad - left) * px_high / height).round();
    // The right edge widened to the picture's exact aspect, so it shows no letterbox at its own size.
    let wide = px_wide * height / px_high;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{px_wide}\" height=\"{px_high}\" \
         role=\"img\" aria-label=\"Wakeshot\">
  <!-- The Wakeshot lockup, {theme} backgrounds: the app icon as in site/wakeshot.svg and the wordmark in Instrument
       Sans SemiBold (site/fonts/OFL.txt), tracking -0.01em as on the page, the letters as outlines so any viewer
       draws them without the font. Written by crates/wakeshot-windows/src/tray/art.rs (write_art_files); change it
       there, not here. -->
  <defs>
    {defs}
  </defs>
  {body}
  <path fill=\"{ink}\" d=\"{WORDMARK}\"/>
</svg>
",
        svg_num(left),
        svg_num(top),
        svg_num(wide),
        svg_num(height)
    )
}

/// The SVG files the art writes into `site/`, by name.
#[cfg(test)]
fn site_svgs() -> [(&'static str, String); 3] {
    [
        ("wakeshot.svg", app_icon_svg()),
        ("wakeshot-lockup-light.svg", lockup_svg(false)),
        ("wakeshot-lockup-dark.svg", lockup_svg(true)),
    ]
}

/// The review sheets in `docs/research/img/ux/`, by file name: one list for the writer and its golden test.
#[cfg(test)]
fn review_sheets() -> [(&'static str, Rgba); 4] {
    [
        ("icon-sheet-dark.png", sheet(TASKBAR_DARK, ON_DARK)),
        ("icon-sheet-light.png", sheet(TASKBAR_LIGHT, ON_LIGHT)),
        // A sample high contrast theme: yellow text on black, every part in the text colour.
        (
            "icon-sheet-contrast.png",
            sheet([0, 0, 0], GlyphColours::one([0xff, 0xff, 0])),
        ),
        ("icon-app.png", app_sheet()),
    ]
}

/// Writes `assets/wakeshot.ico`, `assets/wakeshot-notice.png`, and in `site/` `favicon.ico`, the icon's SVG and the
/// two lockups, and the review sheets in `docs/research/img/ux/`; or all of them into `WAKESHOT_ART_DIR` when it is
/// set (a trial that leaves the repo alone). Run: `cargo test --locked -p wakeshot-windows tray::art::write_art_files
/// -- --ignored`.
#[cfg(test)]
#[test]
#[ignore = "writes the app icon, the notice icon, the page's icons and lockups, and the review sheets"]
fn write_art_files() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (assets, site, img) = match std::env::var_os("WAKESHOT_ART_DIR") {
        Some(dir) => {
            let dir = std::path::PathBuf::from(dir);
            (dir.clone(), dir.clone(), dir)
        }
        None => (
            root.join("assets"),
            root.join("site"),
            root.join("docs/research/img/ux"),
        ),
    };
    let write = |path: std::path::PathBuf, bytes: Vec<u8>| {
        std::fs::write(&path, bytes).expect("write");
        println!("wrote {}", path.display());
    };
    write(assets.join("wakeshot.ico"), app_icon_file());
    write(
        assets.join(crate::notify::NOTICE_ICON_FILE),
        crate::notify::notice_icon_png(),
    );
    write(site.join("favicon.ico"), favicon_file());
    for (name, svg) in site_svgs() {
        write(site.join(name), svg.into_bytes());
    }
    for (name, picture) in review_sheets() {
        write(img.join(name), png(&picture));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(alpha: &[u8]) -> usize {
        alpha.iter().filter(|&&a| a >= 128).count()
    }

    #[test]
    fn every_state_draws_at_every_size() {
        for state in STATES {
            for size in TRAY_SIZES {
                let mask = tray_mask(state, size);
                assert_eq!(mask.size, size);
                assert_eq!(mask.alpha.len(), (size * size) as usize);
                assert_eq!(mask.light.len(), (size * size) as usize);
                // A glyph, not an empty box or a filled one.
                let share = ink(&mask.alpha) as f32 / (size * size) as f32;
                assert!(
                    (0.08..0.6).contains(&share),
                    "{state:?} {size}: ink {share:.2}"
                );
            }
        }
    }

    #[test]
    fn only_the_holding_glyph_has_a_light_part() {
        for size in TRAY_SIZES {
            for state in STATES {
                let mask = tray_mask(state, size);
                assert!(mask.light.iter().zip(&mask.alpha).all(|(l, a)| l <= a));
                let lit = ink(&mask.light);
                if state == IconState::Holding {
                    // The filled light, not the line: about a third of the ink or more.
                    assert!(lit * 3 > ink(&mask.alpha), "{size}: light {lit}");
                } else {
                    assert_eq!(lit, 0, "{state:?} {size}");
                }
            }
        }
    }

    #[test]
    fn states_differ_in_shape() {
        // Shape, not colour: at least a quarter of the ink of two states sits in different pixels.
        for size in [16, 20, 24] {
            for (i, a) in STATES.iter().enumerate() {
                for b in &STATES[i + 1..] {
                    let (ma, mb) = (tray_mask(*a, size), tray_mask(*b, size));
                    let (mut differ, mut union) = (0, 0);
                    for (pa, pb) in ma.alpha.iter().zip(&mb.alpha) {
                        let (ia, ib) = (*pa >= 128, *pb >= 128);
                        union += usize::from(ia || ib);
                        differ += usize::from(ia != ib);
                    }
                    let share = differ as f32 / union as f32;
                    println!("{a:?} {b:?} {size}: {share:.2}");
                    assert!(share >= 0.25, "{a:?} {b:?} {size}: {share:.2}");
                }
            }
        }
    }

    /// Pixels between a quarter and three quarters covered, as a share of all pixels with ink.
    fn half_covered(mask: &Mask) -> f32 {
        let any = mask.alpha.iter().filter(|&&a| a > 0).count();
        let half = mask
            .alpha
            .iter()
            .filter(|&&a| (64..=191).contains(&a))
            .count();
        half as f32 / any as f32
    }

    /// The most grey the held light may leave at 16 to 32 px: its curves and the off slash at 2 px (measured at most
    /// 0.23, idle at 20 px; `docs/research.md`).
    const HELD_GREY: f32 = 0.25;

    #[test]
    fn strokes_are_crisp_at_small_sizes() {
        // Straight strokes sit on whole pixels, so only curves and diagonals leave grey pixels; one stroke off the
        // grid would leave 50% or more.
        for state in STATES {
            for size in [16, 20, 24, 32] {
                let share = half_covered(&tray_mask(state, size));
                println!("{state:?} {size}: {share:.2}");
                assert!(share <= HELD_GREY, "{state:?} {size}: {share:.2}");
            }
        }
    }

    #[test]
    fn the_line_is_solid() {
        // Every pixel of the line is fully covered, but for the round ends once the stroke is 2 px.
        for size in TRAY_SIZES {
            let h = Held::at(size);
            let mask = tray_mask(IconState::Idle, size);
            let cap = h.t.div_ceil(2) * u32::from(h.t >= 2);
            for y in h.line_y..h.line_y + h.t {
                for x in h.x0 + cap..h.x1 - cap {
                    assert_eq!(mask.alpha[(y * size + x) as usize], 255, "{size}: {x},{y}");
                }
            }
        }
    }

    #[test]
    fn the_ring_is_as_solid_as_the_line_at_1_px() {
        // At 16 and 20 px, the ring's pixels are whole or a quarter: no half-grey pixel that makes it look fainter.
        for size in [16, 20] {
            let mask = tray_mask(IconState::Idle, size);
            let grey = mask.alpha.iter().filter(|&&a| a > 64 && a < 255).count();
            assert_eq!(grey, 0, "{size}");
        }
    }

    /// The ink's centre from the top, in pixels: the alpha-weighted mean of pixel centres.
    fn ink_centre(alpha: &[u8], size: u32) -> f32 {
        let (mut sum, mut moment) = (0.0, 0.0);
        for (i, &a) in alpha.iter().enumerate() {
            let y = (i as u32 / size) as f32 + 0.5;
            sum += f32::from(a);
            moment += f32::from(a) * y;
        }
        moment / sum
    }

    #[test]
    fn the_light_sits_level_with_the_next_icon() {
        // The filled glyph's ink centre within 0.5 px of 0.455 of the box (`brand-review.md` T4); idle and off keep
        // the light where it is, so their centres are printed only.
        for size in TRAY_SIZES {
            let target = 0.455 * size as f32;
            let at = |state| ink_centre(&tray_mask(state, size).alpha, size) - target;
            let (holding, idle, off) = (
                at(IconState::Holding),
                at(IconState::Idle),
                at(IconState::Off),
            );
            println!("{size}: holding {holding:+.2} px, idle {idle:+.2}, off {off:+.2}");
            assert!(holding.abs() <= 0.5, "{size}: {holding:+.2}");
        }
    }

    /// The box round every pixel with ink.
    fn ink_box(alpha: &[u8], size: u32) -> (u32, u32, u32, u32) {
        let (mut left, mut top, mut right, mut bottom) = (size, size, 0, 0);
        for y in 0..size {
            for x in 0..size {
                if alpha[(y * size + x) as usize] > 0 {
                    (left, top) = (left.min(x), top.min(y));
                    (right, bottom) = (right.max(x + 1), bottom.max(y + 1));
                }
            }
        }
        (left, top, right, bottom)
    }

    #[test]
    fn glyph_stays_inside_its_box() {
        for state in STATES {
            for size in TRAY_SIZES {
                let mask = tray_mask(state, size);
                let pad = padding(size);
                let (left, top, right, bottom) = ink_box(&mask.alpha, size);
                assert!(
                    left >= pad && top >= pad && right <= size - pad && bottom <= size - pad,
                    "{state:?} {size}: ink {left},{top} to {right},{bottom}, pad {pad}"
                );
                // And it uses its box: no tiny glyph in a large slot.
                let live = size - 2 * pad;
                let span = (right - left).max(bottom - top);
                assert!(
                    span * 10 >= live * 8,
                    "{state:?} {size}: spans {span} of {live}"
                );
            }
        }
    }

    #[test]
    fn the_slash_stays_inside_the_idle_glyph() {
        // T7: off is no larger than idle; the slash runs inside the glyph's own box.
        for size in TRAY_SIZES {
            let idle = ink_box(&tray_mask(IconState::Idle, size).alpha, size);
            let off = ink_box(&tray_mask(IconState::Off, size).alpha, size);
            assert!(
                off.0 >= idle.0 && off.1 >= idle.1 && off.2 <= idle.2 && off.3 <= idle.3,
                "{size}: off {off:?}, idle {idle:?}"
            );
        }
    }

    #[test]
    fn the_slash_leaves_the_line_whole() {
        // The slash strikes the light only: the line and the rows round it are the same in off as in idle.
        for size in TRAY_SIZES {
            let h = Held::at(size);
            let rows = ((h.line_y - 1) * size) as usize..((h.line_y + h.t + 1) * size) as usize;
            let (idle, off) = (
                tray_mask(IconState::Idle, size),
                tray_mask(IconState::Off, size),
            );
            assert_eq!(idle.alpha[rows.clone()], off.alpha[rows], "{size}");
        }
    }

    /// Pixels at or above `min` with no neighbour (of eight) at or above it.
    fn loose(alpha: &[u8], size: u32, min: u8) -> Vec<(u32, u32)> {
        let at = |x: i64, y: i64| {
            if x < 0 || y < 0 || x >= i64::from(size) || y >= i64::from(size) {
                0
            } else {
                alpha[(y as u32 * size + x as u32) as usize]
            }
        };
        let mut out = Vec::new();
        for y in 0..i64::from(size) {
            for x in 0..i64::from(size) {
                if at(x, y) < min {
                    continue;
                }
                let alone = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                    .filter(|&d| d != (0, 0))
                    .all(|(dx, dy)| at(x + dx, y + dy) < min);
                if alone {
                    out.push((x as u32, y as u32));
                }
            }
        }
        out
    }

    #[test]
    fn no_loose_pixels_at_small_sizes() {
        // A4 and T1: a lone pixel at 100% to 150% reads as a speck.
        for size in [16, 20, 24] {
            for state in STATES {
                let mask = tray_mask(state, size);
                assert_eq!(loose(&mask.alpha, size, 32), [], "{state:?} {size}");
            }
            let p = Plated::at(size, size >= LIT_FROM);
            for (name, shape) in [("light", p.light_shape()), ("bar", p.bar_shape())] {
                let cover = raster(size, size, &shape);
                assert_eq!(loose(&cover, size, 32), [], "app icon {name} {size}");
            }
        }
    }

    fn linear(rgb: [f32; 3]) -> [f64; 3] {
        rgb.map(|c| {
            let v = f64::from(c) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        })
    }

    fn luminance(l: [f64; 3]) -> f64 {
        0.2126 * l[0] + 0.7152 * l[1] + 0.0722 * l[2]
    }

    fn ratio(la: f64, lb: f64) -> f64 {
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// WCAG 2 contrast.
    fn contrast(a: [f32; 3], b: [f32; 3]) -> f64 {
        ratio(luminance(linear(a)), luminance(linear(b)))
    }

    /// Protan, deutan and tritan at full strength, applied in linear sRGB (Machado, Oliveira and Fernandes 2009),
    /// the brand's colour-blind check since round 1.
    const MACHADO: [[[f64; 3]; 3]; 3] = [
        [
            [0.152286, 1.052583, -0.204868],
            [0.114503, 0.786281, 0.099216],
            [-0.003882, -0.048116, 1.051998],
        ],
        [
            [0.367322, 0.860646, -0.227968],
            [0.280085, 0.672501, 0.047413],
            [-0.011820, 0.042940, 0.968881],
        ],
        [
            [1.255528, -0.076749, -0.178779],
            [-0.078411, 0.930809, 0.147602],
            [0.004733, 0.691367, 0.303900],
        ],
    ];

    /// WCAG 2 contrast under the worst of the three simulations.
    fn contrast_colour_blind(a: [f32; 3], b: [f32; 3]) -> f64 {
        MACHADO
            .iter()
            .map(|m| {
                let seen =
                    |rgb: [f32; 3]| {
                        let l = linear(rgb);
                        luminance(m.map(|row| {
                            (row[0] * l[0] + row[1] * l[1] + row[2] * l[2]).clamp(0.0, 1.0)
                        }))
                    };
                ratio(seen(a), seen(b))
            })
            .fold(f64::MAX, f64::min)
    }

    /// The lowest contrast of some colour pairs, plain and under the worst colour-blind simulation.
    #[derive(Clone, Copy)]
    struct Lowest {
        plain: f64,
        blind: f64,
    }

    impl Lowest {
        const NONE: Lowest = Lowest {
            plain: f64::MAX,
            blind: f64::MAX,
        };

        fn of(a: [f32; 3], b: [f32; 3]) -> Self {
            let mut low = Lowest::NONE;
            low.add(a, b);
            low
        }

        fn add(&mut self, a: [f32; 3], b: [f32; 3]) {
            self.plain = self.plain.min(contrast(a, b));
            self.blind = self.blind.min(contrast_colour_blind(a, b));
        }

        fn at_least(self, min: f64) -> bool {
            self.plain >= min && self.blind >= min
        }
    }

    impl std::fmt::Display for Lowest {
        fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(out, "{:.2} (colour-blind {:.2})", self.plain, self.blind)
        }
    }

    #[test]
    fn glyph_contrast_on_taskbars() {
        // WCAG 2 non-text contrast (1.4.11), plain and colour-blind: every part of the glyph at least 3:1 on the
        // taskbar and on the hover plate, the holding light's Moss included.
        for (colours, backs) in [
            (ON_DARK, [TASKBAR_DARK, HOVER_DARK]),
            (ON_LIGHT, [TASKBAR_LIGHT, HOVER_LIGHT]),
        ] {
            for back in backs {
                for part in [colours.ink, colours.light] {
                    let low = Lowest::of(f(part), f(back));
                    println!("{part:02x?} on {back:02x?}: {low}");
                    assert!(low.at_least(3.0), "{part:02x?} on {back:02x?}: {low}");
                }
            }
        }
    }

    const START_LIGHT: [u8; 3] = [0xf3; 3];
    const START_DARK: [u8; 3] = [0x20; 3];

    fn colour_at(img: &Rgba, x: u32, y: u32) -> [f32; 3] {
        let i = ((y * img.width + x) * 4) as usize;
        [0, 1, 2].map(|k| f32::from(img.pixels[i + k]))
    }

    /// Plain plate: what the plate covers whole and the light and the bar leave alone.
    fn bare_plate(p: Plated) -> Vec<bool> {
        let size = p.size;
        let plate = raster(size, size, &p.plate(0.0));
        let light = raster(size, size, &p.light_shape());
        let bar = raster(size, size, &p.bar_shape());
        (0..plate.len())
            .map(|i| plate[i] == 255 && light[i] == 0 && bar[i] == 0)
            .collect()
    }

    /// A shape's edge as drawn: every pixel it covers whole, against the first pixel of plain plate one or two steps
    /// away up, down, left or right. Two steps, so a rounded end's half-covered pixel does not hide the pair.
    fn edge_contrast(icon: &Rgba, p: Plated, shape: &Shape) -> Lowest {
        let size = p.size;
        let mask = raster(size, size, shape);
        let bare = bare_plate(p);
        let mut low = Lowest::NONE;
        for y in 0..size {
            for x in 0..size {
                if mask[(y * size + x) as usize] < 255 {
                    continue;
                }
                for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                    for step in 1..=2 {
                        let inside =
                            |v: u32, d: i32| v.checked_add_signed(d * step).filter(|&v| v < size);
                        let (Some(qx), Some(qy)) = (inside(x, dx), inside(y, dy)) else {
                            break;
                        };
                        let q = (qy * size + qx) as usize;
                        if mask[q] == 255 {
                            break;
                        }
                        if bare[q] {
                            low.add(colour_at(icon, x, y), colour_at(icon, qx, qy));
                            break;
                        }
                    }
                }
            }
        }
        low
    }

    /// The light's core as drawn, at its centre, against every pixel of plain plate in the ring just past the
    /// bloom's end, where the plate shows with only its glow.
    fn core_past_the_bloom(icon: &Rgba, p: Plated) -> Lowest {
        let core = colour_at(icon, p.cx as u32, p.cy as u32);
        let end = BLOOM_END * p.r;
        let bare = bare_plate(p);
        let mut low = Lowest::NONE;
        for y in 0..p.size {
            for x in 0..p.size {
                let d = p.distance(x as f32 + 0.5, y as f32 + 0.5);
                if d >= end && d < end + 1.0 && bare[(y * p.size + x) as usize] {
                    low.add(core, colour_at(icon, x, y));
                }
            }
        }
        low
    }

    #[test]
    fn app_icon_contrast() {
        // The plate's two ends on light and dark Start.
        for plate in [PLATE_LIGHT, PLATE_DARK] {
            for start in [START_LIGHT, START_DARK] {
                let low = Lowest::of(f(plate), f(start));
                println!("plate {plate:02x?} on {start:02x?}: {low}");
                assert!(
                    low.at_least(3.0),
                    "plate {plate:02x?} on {start:02x?}: {low}"
                );
            }
        }
        // Every frame as `app_icon` draws it, plain and colour-blind. The bar's edge against the plate next to it.
        // The light: in a flat frame its edge against the plate next to it; in a lit frame its core against the plate
        // just past the bloom, since the bloom and the glow are the light's own soft edge, which a crisp rim cannot
        // beat (its edge as drawn is printed for the record, 1.4 to 2.0).
        for size in APP_ICON_SIZES {
            let icon = app_icon(size);
            let p = Plated::at(size, size >= LIT_FROM);
            let bar = edge_contrast(&icon, p, &p.bar_shape());
            let edge = edge_contrast(&icon, p, &p.light_shape());
            let light = if p.lit {
                let core = core_past_the_bloom(&icon, p);
                println!("{size}: bar {bar}; light's core past the bloom {core}; its edge {edge}");
                core
            } else {
                println!("{size}: bar {bar}; light's edge {edge}");
                edge
            };
            assert!(bar.at_least(3.0), "{size}: bar {bar}");
            assert!(light.at_least(3.0), "{size}: light {light}");
        }
    }

    /// Microsoft's half rule: the share of the icon's visible pixels (alpha at least half) that keep 3:1 against the
    /// background, as drawn over it; plain, and under all three colour-blind simulations.
    fn half_rule(icon: &Rgba, back: [u8; 3]) -> (f64, f64) {
        let (mut n, mut plain, mut blind) = (0, 0, 0);
        for p in icon.pixels.chunks(4) {
            if p[3] < 128 {
                continue;
            }
            let a = f32::from(p[3]) / 255.0;
            let rgb = [0, 1, 2].map(|k| f32::from(p[k]) * a + f32::from(back[k]) * (1.0 - a));
            n += 1;
            plain += usize::from(contrast(rgb, f(back)) >= 3.0);
            blind += usize::from(contrast_colour_blind(rgb, f(back)) >= 3.0);
        }
        (plain as f64 / n as f64, blind as f64 / n as f64)
    }

    #[test]
    fn half_the_icon_keeps_3_to_1_on_start() {
        // "Make sure at least half of your icon passes a 3.0:1 contrast ratio on light and dark theme", measured at
        // 48 px (Microsoft, "Design guidelines for Windows app icons"). Held under colour-blind simulation too.
        let icon = app_icon(48);
        for back in [START_LIGHT, START_DARK] {
            let (plain, blind) = half_rule(&icon, back);
            println!(
                "48 px on {back:02x?}: {:.0}% (colour-blind {:.0}%)",
                plain * 100.0,
                blind * 100.0
            );
            assert!(
                plain >= 0.5 && blind >= 0.5,
                "48 px on {back:02x?}: {plain:.2}, {blind:.2}"
            );
        }
    }

    #[test]
    fn colorize_keeps_alpha_and_sets_colour() {
        let mask = tray_mask(IconState::Idle, 16);
        let img = colorize(&mask, GlyphColours::one([1, 2, 3]));
        assert_eq!((img.width, img.height), (16, 16));
        for (p, a) in img.pixels.chunks(4).zip(&mask.alpha) {
            assert_eq!(p, [1, 2, 3, *a]);
        }
    }

    #[test]
    fn colorize_gives_the_light_its_own_colour() {
        let size = 24;
        let mask = tray_mask(IconState::Holding, size);
        let img = colorize(&mask, ON_DARK);
        let h = Held::at(size);
        let pixel = |x: u32, y: u32| {
            let i = ((y * size + x) * 4) as usize;
            [
                img.pixels[i],
                img.pixels[i + 1],
                img.pixels[i + 2],
                img.pixels[i + 3],
            ]
        };
        let [r, g, b] = HOLDING_ON_DARK;
        assert_eq!(
            pixel(size / 2, h.top + h.d / 2),
            [r, g, b, 255],
            "the light"
        );
        assert_eq!(
            pixel(size / 2, h.line_y),
            [0xff, 0xff, 0xff, 255],
            "the line"
        );
    }

    #[test]
    fn app_icon_draws_at_every_frame_size() {
        for size in APP_ICON_SIZES {
            let icon = app_icon(size);
            assert_eq!((icon.width, icon.height), (size, size));
            assert_eq!(icon.pixels.len(), (size * size * 4) as usize);
            let opaque = icon.pixels.chunks(4).filter(|p| p[3] == 255).count();
            // The plate's channels stay below 0xd0; the light and the bar are near white.
            let mark = icon
                .pixels
                .chunks(4)
                .filter(|p| p[3] == 255 && p[0] > 0xd0 && p[1] > 0xd0)
                .count();
            assert!(opaque * 5 > (size * size * 3) as usize, "{size}: plate");
            assert!(mark * 16 > opaque, "{size}: mark");
            // The corners are clear but for the shadow's faint edge: a rounded plate.
            assert!(icon.pixels[3] < 16, "{size}: corner {}", icon.pixels[3]);
        }
    }

    fn le16(b: &[u8], at: usize) -> u32 {
        u32::from(u16::from_le_bytes([b[at], b[at + 1]]))
    }

    fn le32(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    fn be32(b: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    /// Reads an ICO file back into frames, checking every header field this writer sets.
    fn parse_ico(bytes: &[u8]) -> Vec<Rgba> {
        assert_eq!(le16(bytes, 0), 0, "reserved");
        assert_eq!(le16(bytes, 2), 1, "type icon");
        let count = le16(bytes, 4) as usize;
        let mut next = 6 + 16 * count;
        let mut frames = Vec::new();
        for i in 0..count {
            let e = 6 + 16 * i;
            let side = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            let (w, h) = (side(bytes[e]), side(bytes[e + 1]));
            assert_eq!((bytes[e + 2], bytes[e + 3]), (0, 0), "palette, reserved");
            assert_eq!(
                (le16(bytes, e + 4), le16(bytes, e + 6)),
                (1, 32),
                "planes, bits"
            );
            let (len, offset) = (le32(bytes, e + 8) as usize, le32(bytes, e + 12) as usize);
            assert_eq!(offset, next, "frame {i} follows the one before");
            next = offset + len;
            assert!(next <= bytes.len(), "frame {i} inside the file");
            let d = &bytes[offset..next];
            if d.starts_with(b"\x89PNG") {
                assert_eq!(w, 256, "only the 256 frame is a PNG");
                let frame = parse_png(d);
                assert_eq!(d[25], 6, "the 256 frame is RGBA");
                assert_eq!((frame.width, frame.height), (w, h));
                frames.push(frame);
                continue;
            }
            assert!(w < 256, "the 256 frame is a PNG");
            assert_eq!(le32(d, 0), 40, "BITMAPINFOHEADER");
            assert_eq!((le32(d, 4), le32(d, 8)), (w, 2 * h), "DIB size");
            assert_eq!(
                (le16(d, 12), le16(d, 14), le32(d, 16)),
                (1, 32, 0),
                "planes, bits, BI_RGB"
            );
            let row_mask = w.div_ceil(32) * 4;
            assert_eq!(
                len as u32,
                40 + w * h * 4 + row_mask * h,
                "frame {i} length"
            );
            assert_eq!(le32(d, 20), w * h * 4 + row_mask * h, "biSizeImage");
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    // Bottom-up BGRA.
                    let s = (40 + ((h - 1 - y) * w + x) * 4) as usize;
                    let t = ((y * w + x) * 4) as usize;
                    pixels[t..t + 4].copy_from_slice(&[d[s + 2], d[s + 1], d[s], d[s + 3]]);
                    let m = (40 + w * h * 4 + (h - 1 - y) * row_mask + x / 8) as usize;
                    let transparent = d[m] & (0x80 >> (x % 8)) != 0;
                    assert_eq!(transparent, d[s + 3] == 0, "frame {i} AND mask at {x},{y}");
                }
            }
            frames.push(Rgba {
                width: w,
                height: h,
                pixels,
            });
        }
        assert_eq!(next, bytes.len(), "no bytes after the last frame");
        frames
    }

    /// Fully transparent pixels carry no colour in an ICO, so compare them as zero.
    fn cleared(img: &Rgba) -> Rgba {
        let mut out = img.clone();
        for p in out.pixels.chunks_mut(4) {
            if p[3] == 0 {
                p.copy_from_slice(&[0, 0, 0, 0]);
            }
        }
        out
    }

    fn pattern(width: u32, height: u32) -> Rgba {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let a = if (x + y) % 5 == 0 {
                    0
                } else {
                    (x * 7 + y * 3) as u8
                };
                pixels.extend([x as u8, y as u8, (x ^ y) as u8, a]);
            }
        }
        Rgba {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn ico_frames_parse_back() {
        let frames = vec![
            app_icon(16),
            pattern(33, 33),
            app_icon(48),
            pattern(256, 256),
        ];
        let file = ico(&frames);
        let back = parse_ico(&file);
        assert_eq!(back.len(), frames.len());
        for (a, b) in frames.iter().zip(&back) {
            assert_eq!((a.width, a.height), (b.width, b.height));
            // The PNG frame keeps every byte; the bitmaps clear the colour of transparent pixels.
            let want = if a.width == 256 {
                a.clone()
            } else {
                cleared(a)
            };
            assert_eq!(want, *b, "{} px", a.width);
        }
    }

    /// A slow bitwise CRC-32, apart from the writer's table, so a wrong table cannot pass.
    fn crc_check(bytes: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &b in bytes {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }

    fn adler_check(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &x in bytes {
            a = (a + u32::from(x)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    /// Reads deflate bits, least significant first.
    struct BitReader<'a> {
        data: &'a [u8],
        pos: usize,
    }

    impl BitReader<'_> {
        fn bit(&mut self) -> u32 {
            let byte = self.data[self.pos / 8];
            let bit = (byte >> (self.pos % 8)) & 1;
            self.pos += 1;
            u32::from(bit)
        }

        fn bits(&mut self, n: u32) -> u32 {
            (0..n).fold(0, |v, i| v | (self.bit() << i))
        }
    }

    /// A Huffman decoder built from code lengths, as zlib's `puff` does it: counts per length and the symbols in
    /// code order. Checks that the code is neither over-subscribed nor incomplete.
    struct Decoder {
        count: [u32; 16],
        symbols: Vec<usize>,
    }

    impl Decoder {
        fn new(lengths: &[u32]) -> Self {
            let mut count = [0u32; 16];
            for &l in lengths {
                count[l as usize] += 1;
            }
            let mut left: i64 = 1;
            for &c in &count[1..] {
                left = left * 2 - i64::from(c);
                assert!(left >= 0, "over-subscribed code");
            }
            assert_eq!(left, 0, "incomplete code {lengths:?}");
            let mut symbols = Vec::new();
            for len in 1..16 {
                for (s, &l) in lengths.iter().enumerate() {
                    if l == len {
                        symbols.push(s);
                    }
                }
            }
            count[0] = 0;
            Decoder { count, symbols }
        }

        fn decode(&self, r: &mut BitReader) -> usize {
            let (mut code, mut first, mut index) = (0u32, 0u32, 0u32);
            for len in 1..16 {
                code |= r.bit();
                let n = self.count[len];
                if code < first + n {
                    return self.symbols[(index + code - first) as usize];
                }
                index += n;
                first = (first + n) << 1;
                code <<= 1;
            }
            panic!("no code");
        }
    }

    fn fixed_decoders() -> (Decoder, Decoder) {
        let lit: Vec<u32> = (0..288)
            .map(|s| match s {
                0..=143 => 8,
                144..=255 => 9,
                256..=279 => 7,
                _ => 8,
            })
            .collect();
        (Decoder::new(&lit), Decoder::new(&[5; 32]))
    }

    fn dynamic_decoders(r: &mut BitReader) -> (Decoder, Decoder) {
        let lit = r.bits(5) as usize + 257;
        let dist = r.bits(5) as usize + 1;
        let order = r.bits(4) as usize + 4;
        assert!(lit <= 286 && dist <= 30, "HLIT {lit}, HDIST {dist}");
        let mut cl = [0u32; 19];
        for &s in &CODE_LENGTH_ORDER[..order] {
            cl[s] = r.bits(3);
        }
        let cl = Decoder::new(&cl);
        let mut lengths: Vec<u32> = Vec::new();
        while lengths.len() < lit + dist {
            let sym = cl.decode(r);
            let (value, times) = match sym {
                0..=15 => (sym as u32, 1),
                16 => (*lengths.last().expect("a length to copy"), 3 + r.bits(2)),
                17 => (0, 3 + r.bits(3)),
                _ => (0, 11 + r.bits(7)),
            };
            lengths.extend(std::iter::repeat_n(value, times as usize));
        }
        assert_eq!(lengths.len(), lit + dist, "lengths run past the codes");
        (Decoder::new(&lengths[..lit]), Decoder::new(&lengths[lit..]))
    }

    /// Inflates a zlib stream of stored, fixed and dynamic blocks, and checks its header and Adler-32.
    fn inflate(z: &[u8]) -> Vec<u8> {
        let (cmf, flg) = (u32::from(z[0]), u32::from(z[1]));
        assert_eq!(cmf & 0x0f, 8, "deflate");
        assert_eq!((cmf * 256 + flg) % 31, 0, "FCHECK");
        assert_eq!(flg & 0x20, 0, "no preset dictionary");
        let body = &z[2..z.len() - 4];
        let mut r = BitReader { data: body, pos: 0 };
        let mut out: Vec<u8> = Vec::new();
        loop {
            let last = r.bit();
            let (lit, dist) = match r.bits(2) {
                0 => {
                    r.pos = r.pos.div_ceil(8) * 8;
                    let at = r.pos / 8;
                    let len = le16(body, at) as usize;
                    assert_eq!(le16(body, at + 2) as usize, !len & 0xffff, "NLEN");
                    out.extend(&body[at + 4..at + 4 + len]);
                    r.pos = (at + 4 + len) * 8;
                    if last == 1 {
                        break;
                    }
                    continue;
                }
                1 => fixed_decoders(),
                2 => dynamic_decoders(&mut r),
                other => panic!("block type {other}"),
            };
            loop {
                let sym = lit.decode(&mut r);
                match sym {
                    0..=255 => out.push(sym as u8),
                    256 => break,
                    _ => {
                        let i = sym - 257;
                        assert!(i < 29, "length code {sym}");
                        let len = u32::from(LENGTH_BASE[i]) + r.bits(u32::from(LENGTH_EXTRA[i]));
                        let d = dist.decode(&mut r);
                        assert!(d < 30, "distance code {d}");
                        let back =
                            (u32::from(DIST_BASE[d]) + r.bits(u32::from(DIST_EXTRA[d]))) as usize;
                        assert!(back <= out.len() && back <= WINDOW, "distance {back}");
                        for _ in 0..len {
                            out.push(out[out.len() - back]);
                        }
                    }
                }
            }
            if last == 1 {
                break;
            }
        }
        assert_eq!(
            r.pos.div_ceil(8),
            body.len(),
            "no bytes after the last block"
        );
        assert_eq!(adler_check(&out), be32(z, z.len() - 4), "Adler-32");
        out
    }

    fn unfilter(raw: &[u8], stride: usize, height: usize, bpp: usize) -> Vec<u8> {
        assert_eq!(raw.len(), (stride + 1) * height, "scanline bytes");
        let mut out = vec![0u8; stride * height];
        for y in 0..height {
            let f = raw[y * (stride + 1)];
            for x in 0..stride {
                let v = raw[y * (stride + 1) + 1 + x];
                let a = if x >= bpp {
                    out[y * stride + x - bpp]
                } else {
                    0
                };
                let b = if y > 0 { out[(y - 1) * stride + x] } else { 0 };
                let c = if x >= bpp && y > 0 {
                    out[(y - 1) * stride + x - bpp]
                } else {
                    0
                };
                let predict = match f {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => ((u32::from(a) + u32::from(b)) / 2) as u8,
                    4 => {
                        let p = i32::from(a) + i32::from(b) - i32::from(c);
                        let (pa, pb, pc) = (
                            (p - i32::from(a)).abs(),
                            (p - i32::from(b)).abs(),
                            (p - i32::from(c)).abs(),
                        );
                        if pa <= pb && pa <= pc {
                            a
                        } else if pb <= pc {
                            b
                        } else {
                            c
                        }
                    }
                    _ => panic!("filter {f}"),
                };
                out[y * stride + x] = v.wrapping_add(predict);
            }
        }
        out
    }

    /// Checks a PNG file's structure and CRCs and returns its pixels.
    fn parse_png(file: &[u8]) -> Rgba {
        assert_eq!(&file[..8], b"\x89PNG\r\n\x1a\n", "signature");
        let mut at = 8;
        let mut kinds = Vec::new();
        let (mut width, mut height, mut kind_of_colour) = (0, 0, 0);
        let (mut idat, mut plte, mut trns) = (Vec::new(), Vec::new(), Vec::new());
        while at < file.len() {
            let len = be32(file, at) as usize;
            let kind = &file[at + 4..at + 8];
            let data = &file[at + 8..at + 8 + len];
            assert_eq!(
                be32(file, at + 8 + len),
                crc_check(&file[at + 4..at + 8 + len]),
                "CRC of {kind:?}"
            );
            match kind {
                b"IHDR" => {
                    (width, height, kind_of_colour) = (be32(data, 0), be32(data, 4), data[9]);
                    assert_eq!(data[8], 8, "8 bits");
                    assert!(
                        kind_of_colour == 3 || kind_of_colour == 6,
                        "palette or RGBA"
                    );
                    assert_eq!(&data[10..], [0, 0, 0], "no interlace");
                }
                b"PLTE" => plte.extend(data),
                b"tRNS" => trns.extend(data),
                b"IDAT" => idat.extend(data),
                _ => {}
            }
            kinds.push(String::from_utf8_lossy(kind).into_owned());
            at += 12 + len;
        }
        assert_eq!(at, file.len());
        assert_eq!(kinds.first().map(String::as_str), Some("IHDR"));
        assert_eq!(kinds.last().map(String::as_str), Some("IEND"));
        let (w, h) = (width as usize, height as usize);
        let pixels = if kind_of_colour == 3 {
            assert!(plte.len() % 3 == 0 && plte.len() <= 768 && trns.len() <= plte.len() / 3);
            let indices = unfilter(&inflate(&idat), w, h, 1);
            indices
                .iter()
                .flat_map(|&i| {
                    let i = usize::from(i);
                    let a = trns.get(i).copied().unwrap_or(255);
                    [plte[3 * i], plte[3 * i + 1], plte[3 * i + 2], a]
                })
                .collect()
        } else {
            unfilter(&inflate(&idat), w * 4, h, 4)
        };
        Rgba {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn png_is_valid() {
        // Uneven sizes, every alpha, long flat runs and repeated rows; RGBA past 256 colours, else a palette.
        let flat = Rgba {
            width: 700,
            height: 90,
            pixels: [9, 8, 7, 255].repeat(700 * 90),
        };
        let glyph = colorize(&tray_mask(IconState::Holding, 48), ON_DARK);
        let cases = [
            (pattern(37, 23), 6),
            (pattern(1, 1), 3),
            (flat, 3),
            (glyph, 3),
            (app_icon(256), 6),
            (app_icon(16), 3),
        ];
        for (img, colour_type) in cases {
            let file = png(&img);
            assert_eq!(
                file[25], colour_type,
                "{}x{} colour type",
                img.width, img.height
            );
            assert_eq!(parse_png(&file), img, "{}x{}", img.width, img.height);
        }
    }

    #[test]
    fn deflate_round_trips() {
        // Empty, one byte, no repeats, long runs, matches far back in the window and past it, and every byte value.
        let mut noise = Vec::new();
        let mut x = 12345u32;
        for _ in 0..70_000 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12345);
            noise.push((x >> 16) as u8);
        }
        let mut far = noise[..40_000].to_vec();
        far.extend_from_slice(&noise[..40_000]);
        let all: Vec<u8> = (0..=255).cycle().take(5000).collect();
        let cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![7],
            vec![1, 2],
            b"abcabcabcabcabcabcabcabcabcabcabcx".to_vec(),
            vec![0; 100_000],
            noise,
            far,
            all,
            scanlines(&app_icon(256).pixels, 256 * 4, 4),
        ];
        for data in cases {
            let z = zlib(&data);
            assert_eq!(inflate(&z), data, "{} bytes", data.len());
        }
    }

    #[test]
    fn png_compresses_flat_areas() {
        // The review sheets are mostly one colour; a stored PNG of one would be tens of megabytes.
        let flat = Rgba {
            width: 1000,
            height: 1000,
            pixels: [0x20, 0x20, 0x21, 255].repeat(1000 * 1000),
        };
        assert!(png(&flat).len() < 2_000, "{}", png(&flat).len());
    }

    #[test]
    fn the_256_frame_is_well_compressed() {
        // 256 KB of pixels; matches and each block's own codes take it to about an eighth (measured 32,229 bytes).
        let file = png_rgba(&app_icon(256));
        println!("256 frame: {} bytes", file.len());
        assert!(file.len() < 36_000, "{}", file.len());
    }

    #[test]
    fn crc_table_matches_the_bitwise_crc() {
        assert_eq!(crc_check(b"IEND"), 0xae42_6082);
        assert_eq!(crc_check(b"123456789"), 0xcbf4_3926);
    }

    fn repo(path: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }

    const REWRITE: &str =
        "run `cargo test --locked -p wakeshot-windows tray::art::write_art_files -- --ignored`";

    #[test]
    fn app_icon_file_matches_the_art() {
        let file = std::fs::read(repo("assets/wakeshot.ico")).expect("assets/wakeshot.ico");
        let frames = parse_ico(&file);
        let sizes: Vec<u32> = frames.iter().map(|f| f.width).collect();
        assert_eq!(sizes, APP_ICON_SIZES, "frame sizes");
        // Exact: the art uses no sin, cos or exp and the writers have no randomness, so x64 and ARM64 write the same
        // bytes.
        assert!(
            file == app_icon_file(),
            "assets/wakeshot.ico differs from the art: {REWRITE}"
        );
    }

    #[test]
    fn favicon_matches_the_art() {
        let file = std::fs::read(repo("site/favicon.ico")).expect("site/favicon.ico");
        let sizes: Vec<u32> = parse_ico(&file).iter().map(|f| f.width).collect();
        assert_eq!(sizes, FAVICON_SIZES, "frame sizes");
        assert!(
            file == favicon_file(),
            "site/favicon.ico differs from the art: {REWRITE}"
        );
    }

    #[test]
    fn site_svgs_match_the_art() {
        for (name, svg) in site_svgs() {
            let path = format!("site/{name}");
            let file = std::fs::read_to_string(repo(&path)).expect(&path);
            assert!(file == svg, "{path} differs from the art: {REWRITE}");
        }
    }

    #[test]
    fn review_sheets_match_the_art() {
        // `media/scripts/check-art.mjs` holds the media port to the glyph sheets, so without this a tray glyph change
        // made without the writer run passes every check while the media keep the old glyph.
        for (name, picture) in review_sheets() {
            let path = format!("docs/research/img/ux/{name}");
            let file = std::fs::read(repo(&path)).expect(&path);
            assert!(
                file == png(&picture),
                "{path} differs from the art: {REWRITE}"
            );
        }
    }

    #[test]
    fn svg_numbers_are_short_and_exact() {
        assert_eq!(svg_num(11.0), "11");
        assert_eq!(svg_num(43.875), "43.875");
        assert_eq!(svg_num(287.825_403), "287.825");
        assert_eq!(svg_num(0.0), "0");
        assert_eq!(svg_num(-0.000_1), "0");
        assert_eq!(svg_num(100.0), "100");
        assert_eq!(svg_num(f64::from(0.21_f32)), "0.21");
    }
}
