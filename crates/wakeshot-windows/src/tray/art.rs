//! Icon art as code: tray glyphs, the app icon, ICO and PNG writers.
//!
//! Owner: `p5-icon-art`, then `p5-haik-check`.
//!
//! No Windows API here, so the file could move to the core. Shapes use plain arithmetic and `sqrt`, never `sin` or
//! `cos`: those may differ in the last bit between x64 and ARM64, and the golden test compares exactly.
//!
//! The tray icon draws `tray_mask` at the size the taskbar wants and colours it with `colorize` (`GLYPH_ON_DARK`,
//! `GLYPH_ON_LIGHT` or the high contrast text colour). Design notes and measured colours: `docs/research.md`, "Tray
//! UX: icon art"; the brand (the Sunrise, the colours, why): `docs/brand.md`.

use wakeshot_core::view::IconState;

/// Alpha only, `size` by `size`, top-down rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub size: u32,
    pub alpha: Vec<u8>,
}

/// Straight (not premultiplied) RGBA, top-down rows, 4 bytes per pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// The tray glyph designs. Haik picked the Sunrise (2026-11-04, `docs/brand.md`); the others stay drawable until
/// `p5-haik-check`, which removes them. Off is always the idle glyph struck through, as Windows marks its own off
/// states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Concept {
    /// Holding: a risen half sun with rays over the horizon. Idle: an empty half sun on the horizon.
    Sunrise,
    /// Holding: a full cup with steam. Idle: an empty cup.
    Cup,
    /// Holding: an open eye. Idle: a closed eye with lashes.
    Eye,
    /// Holding: a sun. Idle: a moon.
    Sky,
}

impl Concept {
    pub const ALL: [Concept; 4] = [Concept::Sunrise, Concept::Cup, Concept::Eye, Concept::Sky];
}

/// The concept the product draws.
pub const PICK: Concept = Concept::Sunrise;

/// The sizes Windows asks for in the tray, 100% to 400% scale.
pub const TRAY_SIZES: [u32; 10] = [16, 20, 24, 28, 32, 36, 40, 48, 56, 64];

/// The frames of `assets/wakeshot.ico` (track note 3.2: no 256 frame).
pub const APP_ICON_SIZES: [u32; 7] = [16, 20, 24, 32, 40, 48, 64];

// Measured in this laptop's screenshots (research "Tray UX: icon art"). The taskbar is translucent, so its colour
// moves a few steps with the wallpaper.

/// Windows' own tray glyph colour on a dark taskbar.
pub const GLYPH_ON_DARK: [u8; 3] = [0xff, 0xff, 0xff];
/// Windows' own glyph colour on a light background (its glyphs there match black at alpha 0xe4; solid 1a1a1a looks the
/// same and does not need a second alpha).
pub const GLYPH_ON_LIGHT: [u8; 3] = [0x1a, 0x1a, 0x1a];
/// The dark taskbar behind the tray.
pub const TASKBAR_DARK: [u8; 3] = [0x20, 0x20, 0x21];
/// The light taskbar behind the tray.
pub const TASKBAR_LIGHT: [u8; 3] = [0xd8, 0xda, 0xdc];
/// The plate under a tray icon the mouse rests on, dark taskbar.
pub const HOVER_DARK: [u8; 3] = [0x2d, 0x2d, 0x2e];
/// The plate under a tray icon the mouse rests on, light taskbar.
pub const HOVER_LIGHT: [u8; 3] = [0xec, 0xed, 0xee];

/// The app icon's plate (the brand's night violet) runs from this colour at the top left to `APP_DARK` at the bottom
/// right. Both keep 3:1 with light and dark backgrounds and with the gold sun on them (`docs/brand.md`).
const APP_LIGHT: [u8; 3] = [0x79, 0x57, 0xf4];
const APP_DARK: [u8; 3] = [0x6b, 0x49, 0xf3];
/// The sun in the app icon (the brand's sun gold); the horizon under it is white.
const SUN_GOLD: [u8; 3] = [0xff, 0xcc, 0x4d];

/// The empty border a tray glyph keeps on each side, in pixels: 1 px at 100% scale, 4 px at 400%.
pub fn padding(size: u32) -> u32 {
    (size / 16).max(1)
}

/// The picked concept's tray glyph for `state`.
pub fn tray_mask(state: IconState, size: u32) -> Mask {
    concept_mask(PICK, state, size)
}

/// Any concept's tray glyph, for the review sheets.
pub fn concept_mask(concept: Concept, state: IconState, size: u32) -> Mask {
    let shape = glyph(concept, state, Grid::new(0, 0, size));
    Mask {
        size,
        alpha: raster(size, size, &shape),
    }
}

/// The glyph in one colour; the caller picks it from the theme.
pub fn colorize(mask: &Mask, rgb: [u8; 3]) -> Rgba {
    let pixels = mask
        .alpha
        .iter()
        .flat_map(|&a| [rgb[0], rgb[1], rgb[2], a])
        .collect();
    Rgba {
        width: mask.size,
        height: mask.size,
        pixels,
    }
}

/// The picked concept's app icon.
pub fn app_icon(size: u32) -> Rgba {
    concept_app_icon(PICK, size)
}

/// Any concept's app icon, for the review sheet: the holding glyph on a violet rounded square, so it reads on light
/// and dark backgrounds alike. The Sunrise's sun is gold over a white horizon; the other concepts are white.
pub fn concept_app_icon(concept: Concept, size: u32) -> Rgba {
    let centre = size as f32 / 2.0;
    // A margin of size / 24 (2 px at 48), so the plate does not look larger than other apps' icons. A design call:
    // Microsoft's pages give no margin number.
    let margin = ((size + 12) / 24) as f32;
    let half = centre - margin;
    let corner = (size as f32 * 0.1875).max(1.0);
    let plate = raster(
        size,
        size,
        &Shape::Rect {
            x0: margin,
            y0: margin,
            x1: size as f32 - margin,
            y1: size as f32 - margin,
            r: [corner; 4],
        },
    );
    // About 5/7 of the icon, with an even border so the mark sits on whole pixels.
    let mut mark_size = (size as f32 * 0.72).round() as u32;
    if (size - mark_size) % 2 == 1 {
        mark_size -= 1;
    }
    let at = (size - mark_size) / 2;
    let g = Grid::new(at, at, mark_size);
    let layers: Vec<(Vec<u8>, [u8; 3])> = match concept {
        Concept::Sunrise => {
            let (horizon, sun) = risen_sun(g);
            vec![
                (raster(size, size, &sun), SUN_GOLD),
                (raster(size, size, &horizon), [0xff; 3]),
            ]
        }
        _ => vec![(
            raster(size, size, &glyph(concept, IconState::Holding, g)),
            [0xff; 3],
        )],
    };
    // The light falls from the top left (Microsoft's default gradient angle, 120 degrees).
    let (dx, dy) = (3f32.sqrt() / 2.0, 0.5);
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let i = (y * size + x) as usize;
            if plate[i] == 0 {
                pixels.extend([0, 0, 0, 0]);
                continue;
            }
            let along = ((x as f32 + 0.5 - centre) * dx + (y as f32 + 0.5 - centre) * dy) / half;
            let mut rgb = mix(APP_LIGHT, APP_DARK, ((along + 1.0) / 2.0).clamp(0.0, 1.0));
            for (mark, colour) in &layers {
                rgb = mix(rgb, *colour, f32::from(mark[i]) / 255.0);
            }
            pixels.extend([rgb[0], rgb[1], rgb[2], plate[i]]);
        }
    }
    Rgba {
        width: size,
        height: size,
        pixels,
    }
}

/// The bytes of `assets/wakeshot.ico`.
pub fn app_icon_file() -> Vec<u8> {
    let frames: Vec<Rgba> = APP_ICON_SIZES.iter().map(|&s| app_icon(s)).collect();
    ico(&frames)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let one = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
    [one(a[0], b[0]), one(a[1], b[1]), one(a[2], b[2])]
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
    /// The points with `nx * x + ny * y <= c`; cuts a ring into an arc.
    Half {
        nx: f32,
        ny: f32,
        c: f32,
    },
    Any(Vec<Shape>),
    All(Vec<Shape>),
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
            Shape::Half { nx, ny, c } => nx * x + ny * y <= *c,
            Shape::Any(all) => all.iter().any(|s| s.contains(x, y)),
            Shape::All(all) => all.iter().all(|s| s.contains(x, y)),
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

/// Stroke width in whole pixels, about 1 px per 16 px of icon, so a straight stroke covers whole pixels.
fn stroke(size: u32) -> f32 {
    (size as f32 / 16.0).round().max(1.0)
}

/// A 16-unit design grid laid over a box of whole pixels. Edges snap to pixel edges, so a 1 px stroke at 16 px stays
/// one sharp pixel instead of two grey ones.
#[derive(Clone, Copy)]
struct Grid {
    x: f32,
    y: f32,
    k: f32,
    t: f32,
}

impl Grid {
    fn new(x: u32, y: u32, size: u32) -> Self {
        Self {
            x: x as f32,
            y: y as f32,
            k: size as f32 / 16.0,
            t: stroke(size),
        }
    }

    fn ex(self, u: f32) -> f32 {
        self.x + (u * self.k).round()
    }

    fn ey(self, u: f32) -> f32 {
        self.y + (u * self.k).round()
    }

    fn len(self, u: f32) -> f32 {
        u * self.k
    }

    /// Moves a stroke's centre line so both of its edges land on pixel edges.
    fn centre(self, v: f32) -> f32 {
        (v - self.t / 2.0).round() + self.t / 2.0
    }
}

fn glyph(concept: Concept, state: IconState, g: Grid) -> Shape {
    match concept {
        Concept::Sunrise => sunrise(state, g),
        Concept::Cup => cup(state, g),
        Concept::Eye => eye(state, g),
        Concept::Sky => sky(state, g),
    }
}

/// A rounded rect's outline, `t` wide, inside its edges.
fn frame(x0: f32, y0: f32, x1: f32, y1: f32, r: [f32; 4], t: f32) -> Shape {
    let outer = Shape::Rect { x0, y0, x1, y1, r };
    let inner = Shape::Rect {
        x0: x0 + t,
        y0: y0 + t,
        x1: x1 - t,
        y1: y1 - t,
        r: r.map(|v| (v - t).max(0.0)),
    };
    minus(outer, inner)
}

/// The off state: the idle glyph struck through from top left to bottom right, with a stroke-wide gap on each side
/// of the bar, as Windows draws its own off glyphs.
fn slashed(g: Grid, base: Shape) -> Shape {
    let t = g.t;
    let bar = |w: f32| Shape::Line {
        ax: g.ex(1.0) + t / 2.0,
        ay: g.ey(1.0) + t / 2.0,
        bx: g.ex(15.0) - t / 2.0,
        by: g.ey(15.0) - t / 2.0,
        w,
    };
    Shape::Any(vec![minus(base, bar(3.0 * t)), bar(t)])
}

/// A sun over the horizon: risen with rays while holding, an empty half sun when idle, struck through when off.
fn sunrise(state: IconState, g: Grid) -> Shape {
    match state {
        IconState::Holding => {
            let (horizon, sun) = risen_sun(g);
            Shape::Any(vec![horizon, sun])
        }
        IconState::Idle => empty_sun(g, false),
        IconState::Off => slashed(g, empty_sun(g, true)),
    }
}

/// The horizon bar with its top at `horizon` units, and a sun centre and radius above it: the centre sits on a pixel
/// centre for 1 px strokes (so the upright ray is one sharp column), and both sides of the sun on pixel edges.
/// Returns the bar, the centre's x, the radius and the sun's flat base, one stroke above the bar.
fn horizon_and_sun(g: Grid, radius: f32, horizon: f32) -> (Shape, f32, f32, f32) {
    let t = g.t;
    let top = g.ey(horizon);
    let bar = Shape::Rect {
        x0: g.ex(1.0),
        y0: top,
        x1: g.ex(15.0),
        y1: top + t,
        r: [(t / 2.0).floor(); 4],
    };
    let cx = g.centre(g.x + g.len(8.0));
    let r = (cx + g.len(radius)).round() - cx;
    (bar, cx, r, top - t)
}

/// The holding glyph's two layers, so the app icon can colour them apart: the horizon, and the half sun with three
/// rays (upright and at 45 degrees), each a gap away from the sun.
fn risen_sun(g: Grid) -> (Shape, Shape) {
    let t = g.t;
    let (horizon, cx, r, base) = horizon_and_sun(g, 4.0, 13.0);
    let gap = g.len(1.25).round().max(t);
    let len = g.len(3.0).round().max(t);
    let half_disc = Shape::All(vec![
        Shape::Disc { cx, cy: base, r },
        Shape::Half {
            nx: 0.0,
            ny: 1.0,
            c: base,
        },
    ]);
    // Both ends of the upright ray on pixel edges, so it stays sharp.
    let inner = (base - r - gap).floor();
    let mut parts = vec![
        half_disc,
        Shape::Line {
            ax: cx,
            ay: inner - t / 2.0,
            bx: cx,
            by: inner - len + t / 2.0,
            w: t,
        },
    ];
    if t < 1.5 {
        // A 1 px line at 45 degrees turns into grey pairs; whole pixels stepping along the diagonal stay sharp. The
        // first pixel is the one whose centre clears the sun by the gap.
        let (col, row) = ((cx - 0.5) as u32, base as u32);
        let reach = r + gap + 0.5;
        let first = (0..row)
            .find(|&j| {
                let (dx, dy) = (1.0 + j as f32, 0.5 + j as f32);
                dx * dx + dy * dy >= reach * reach
            })
            .unwrap_or(0);
        let steps = (len / std::f32::consts::SQRT_2).round().max(1.0) as u32;
        let dot = |x: u32, y: u32| Shape::Rect {
            x0: x as f32,
            y0: y as f32,
            x1: x as f32 + 1.0,
            y1: y as f32 + 1.0,
            r: [0.0; 4],
        };
        for j in first..first + steps {
            if j + 1 > col || j + 1 > row {
                break;
            }
            parts.push(dot(col - 1 - j, row - 1 - j));
            parts.push(dot(col + 1 + j, row - 1 - j));
        }
    } else {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let (a, b) = (r + gap + t / 2.0, r + gap + len - t / 2.0);
        for side in [-1.0, 1.0] {
            parts.push(Shape::Line {
                ax: cx + side * d * a,
                ay: base - d * a,
                bx: cx + side * d * b,
                by: base - d * b,
                w: t,
            });
        }
    }
    (horizon, Shape::Any(parts))
}

/// The idle glyph: a larger, empty half sun on a horizon one unit higher than holding's, so the glyph sits in the
/// middle of its slot. `off` cuts the horizon at the off bar, so no stub is left past the bar's gap.
fn empty_sun(g: Grid, off: bool) -> Shape {
    let t = g.t;
    let (horizon, cx, r, base) = horizon_and_sun(g, 5.0, 12.0);
    let ring = Shape::All(vec![
        minus(
            Shape::Disc { cx, cy: base, r },
            Shape::Disc {
                cx,
                cy: base,
                r: r - t,
            },
        ),
        Shape::Half {
            nx: 0.0,
            ny: 1.0,
            c: base,
        },
    ]);
    let horizon = if off {
        // The off bar runs along x - g.x = y - g.y (`slashed`); keep the horizon on its lower left side.
        Shape::All(vec![
            horizon,
            Shape::Half {
                nx: 1.0,
                ny: -1.0,
                c: g.x - g.y,
            },
        ])
    } else {
        horizon
    };
    Shape::Any(vec![horizon, ring])
}

/// A cup with a handle: filled with steam while holding, empty when idle, struck through when off.
fn cup(state: IconState, g: Grid) -> Shape {
    let t = g.t;
    let (x0, y0, x1, y1) = (g.ex(1.0), g.ey(6.0), g.ex(12.0), g.ey(15.0));
    let rb = g.len(3.5).round().max(t);
    let round_bottom = [0.0, 0.0, rb, rb];
    // The handle's left side lies on the body's right wall. Its corners stay tight: a wider round turns its few pixels
    // grey at 16 and 20 px.
    let rh = g.len(1.5).floor().max(t);
    let handle = frame(
        x1 - t,
        g.ey(7.5),
        g.ex(15.0),
        g.ey(12.5),
        [0.0, rh, rh, 0.0],
        t,
    );
    let outline = || Shape::Any(vec![frame(x0, y0, x1, y1, round_bottom, t), handle.clone()]);
    match state {
        IconState::Holding => {
            let body = Shape::Rect {
                x0,
                y0,
                x1,
                y1,
                r: round_bottom,
            };
            Shape::Any(vec![body, handle.clone(), steam(g, x0, x1, y0)])
        }
        IconState::Idle => outline(),
        IconState::Off => slashed(g, outline()),
    }
}

/// Upright wisps over the cup, from the top of the box to a small gap above the rim: three when they fit with a
/// stroke-wide gap between them, else two.
fn steam(g: Grid, x0: f32, x1: f32, rim: f32) -> Shape {
    let t = g.t;
    let gap = g.len(1.5).round().max(t);
    let (top, bottom) = (g.ey(1.0) + t / 2.0, rim - gap - t / 2.0);
    let width = x1 - x0;
    let mid = (x0 + x1) / 2.0;
    let three = (width / 4.0).round();
    let (n, pitch) = if three >= 2.0 * t {
        (3, three)
    } else {
        (2, (width / 3.0).round())
    };
    let wisps = (0..n)
        .map(|i| {
            let x = g.centre(mid + (i as f32 - (n - 1) as f32 / 2.0) * pitch);
            Shape::Line {
                ax: x,
                ay: top,
                bx: x,
                by: bottom.max(top),
                w: t,
            }
        })
        .collect();
    Shape::Any(wisps)
}

/// An eye: open with a pupil while holding, closed (asleep) when idle, closed and struck through when off.
fn eye(state: IconState, g: Grid) -> Shape {
    let t = g.t;
    let (left, right) = (g.ex(1.0), g.ex(15.0));
    let mid = (left + right) / 2.0;
    let half_width = (right - left) / 2.0;
    match state {
        IconState::Holding => {
            let (top, bottom) = (g.ey(3.0), g.ey(13.0));
            let sag = (bottom - top) / 2.0;
            // The two lids are arcs of one radius through both corners.
            let r = (half_width * half_width + sag * sag) / (2.0 * sag);
            let lens = |inset: f32| {
                Shape::All(vec![
                    Shape::Disc {
                        cx: mid,
                        cy: top + r,
                        r: r - inset,
                    },
                    Shape::Disc {
                        cx: mid,
                        cy: bottom - r,
                        r: r - inset,
                    },
                ])
            };
            let pupil = Shape::Disc {
                cx: mid,
                cy: (top + bottom) / 2.0,
                r: sag - 2.0 * t,
            };
            Shape::Any(vec![minus(lens(0.0), lens(t)), pupil])
        }
        IconState::Idle => closed_eye(g),
        IconState::Off => slashed(g, closed_eye(g)),
    }
}

/// The lower lid as an arc with three lashes.
fn closed_eye(g: Grid) -> Shape {
    let t = g.t;
    let (left, right) = (g.ex(1.0), g.ex(15.0));
    let mid = (left + right) / 2.0;
    let half_width = (right - left) / 2.0 - t / 2.0;
    let (corner, low) = (g.ey(6.5) + t / 2.0, g.ey(11.0) - t / 2.0);
    let sag = low - corner;
    // The stroke's centre line: a circle through both corners and the lowest point.
    let r = (half_width * half_width + sag * sag) / (2.0 * sag);
    let cy = low - r;
    let arc = Shape::All(vec![
        minus(
            Shape::Disc {
                cx: mid,
                cy,
                r: r + t / 2.0,
            },
            Shape::Disc {
                cx: mid,
                cy,
                r: r - t / 2.0,
            },
        ),
        Shape::Half {
            nx: 0.0,
            ny: -1.0,
            c: -corner,
        },
    ]);
    let cap = |x: f32| Shape::Disc {
        cx: x,
        cy: corner,
        r: t / 2.0,
    };
    // The band runs past its corners at an angle; it ends at the corners.
    let arc = Shape::All(vec![
        arc,
        Shape::Half {
            nx: -1.0,
            ny: 0.0,
            c: -left,
        },
        Shape::Half {
            nx: 1.0,
            ny: 0.0,
            c: right,
        },
    ]);
    let lash = |x: f32, to_x: f32| {
        let dx = x - mid;
        Shape::Line {
            ax: x,
            ay: cy + (r * r - dx * dx).sqrt(),
            bx: to_x,
            by: g.ey(14.5) - t / 2.0,
            w: t,
        }
    };
    let centre = g.centre(mid);
    Shape::Any(vec![
        arc,
        cap(mid - half_width),
        cap(mid + half_width),
        lash(centre, centre),
        lash(mid - g.len(4.0), mid - g.len(5.5)),
        lash(mid + g.len(4.0), mid + g.len(5.5)),
    ])
}

/// A sun while holding, a moon (the PC sleeps as usual) when idle, the moon struck through when off.
fn sky(state: IconState, g: Grid) -> Shape {
    let t = g.t;
    match state {
        IconState::Holding => {
            // Centred on a pixel centre for odd strokes, so the upright and level rays stay sharp.
            let c = g.centre(g.x + g.len(8.0));
            let cy = g.centre(g.y + g.len(8.0));
            let r = (c + g.len(3.0)).round() - c;
            let (from, to) = (r + t, g.ex(15.0) - c);
            let d = std::f32::consts::FRAC_1_SQRT_2;
            let mut parts = vec![Shape::Disc { cx: c, cy, r }];
            for (ux, uy) in [
                (1.0, 0.0),
                (d, d),
                (0.0, 1.0),
                (-d, d),
                (-1.0, 0.0),
                (-d, -d),
                (0.0, -1.0),
                (d, -d),
            ] {
                let (a, b) = (from + t / 2.0, to - t / 2.0);
                parts.push(Shape::Line {
                    ax: c + ux * a,
                    ay: cy + uy * a,
                    bx: c + ux * b,
                    by: cy + uy * b,
                    w: t,
                });
            }
            Shape::Any(parts)
        }
        IconState::Idle => moon(g),
        IconState::Off => slashed(g, moon(g)),
    }
}

fn moon(g: Grid) -> Shape {
    let (c, r) = (g.x + g.len(8.0), g.len(7.0));
    minus(
        Shape::Disc {
            cx: c,
            cy: g.y + g.len(8.0),
            r,
        },
        Shape::Disc {
            cx: c + g.len(3.5),
            cy: g.y + g.len(4.5),
            r: g.len(5.5),
        },
    )
}

/// An ICO file of 32-bit frames, each with an AND mask, in the order given. Each frame is square, 1 to 256 px.
pub fn ico(frames: &[Rgba]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = frames.iter().map(dib).collect();
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

/// A PNG file, 8 bits per channel: a palette when the image has at most 256 colours (icon sheets have about 65), else
/// RGBA. Deflate with fixed codes and runs only: flat areas shrink a hundredfold, and no crate is needed.
pub fn png(img: &Rgba) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::with_capacity(13);
    header.extend(img.width.to_be_bytes());
    header.extend(img.height.to_be_bytes());
    match palette(img) {
        Some((colours, indices)) => {
            header.extend([8, 3, 0, 0, 0]);
            chunk(&mut out, b"IHDR", &header);
            let rgb: Vec<u8> = colours.iter().flat_map(|c| [c[0], c[1], c[2]]).collect();
            chunk(&mut out, b"PLTE", &rgb);
            if colours.iter().any(|c| c[3] < 255) {
                let alpha: Vec<u8> = colours.iter().map(|c| c[3]).collect();
                chunk(&mut out, b"tRNS", &alpha);
            }
            chunk(
                &mut out,
                b"IDAT",
                &zlib(&scanlines(&indices, img.width as usize, 1)),
            );
        }
        None => {
            header.extend([8, 6, 0, 0, 0]);
            chunk(&mut out, b"IHDR", &header);
            chunk(
                &mut out,
                b"IDAT",
                &zlib(&scanlines(&img.pixels, img.width as usize * 4, 4)),
            );
        }
    }
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
    // Deflate, 32 KB window, no dictionary; 0x7801 is a multiple of 31 as the header check needs.
    let mut out = vec![0x78, 0x01];
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
    fn code(&mut self, code: u32, len: u32) {
        self.put(code.reverse_bits() >> (32 - len), len);
    }

    /// The fixed literal and length codes of RFC 1951 section 3.2.6.
    fn symbol(&mut self, sym: u32) {
        match sym {
            0..=143 => self.code(0x30 + sym, 8),
            144..=255 => self.code(0x190 + sym - 144, 9),
            256..=279 => self.code(sym - 256, 7),
            _ => self.code(0xc0 + sym - 280, 8),
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// One fixed-code block. Its only matches repeat the byte before (distance 1), which after the row filters covers
/// the flat areas.
fn deflate(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter::default();
    w.put(1, 1);
    w.put(1, 2);
    let mut i = 0;
    while i < data.len() {
        let run = match i {
            0 => 0,
            _ => data[i..]
                .iter()
                .take(258)
                .take_while(|&&b| b == data[i - 1])
                .count(),
        };
        if run >= 3 {
            let k = LENGTH_BASE
                .iter()
                .rposition(|&b| usize::from(b) <= run)
                .unwrap_or(0);
            w.symbol(257 + k as u32);
            w.put(
                (run - usize::from(LENGTH_BASE[k])) as u32,
                u32::from(LENGTH_EXTRA[k]),
            );
            w.code(0, 5);
            i += run;
        } else {
            w.symbol(u32::from(data[i]));
            i += 1;
        }
    }
    w.symbol(256);
    w.finish()
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

/// Rows: each concept's holding, idle and off; a bar at the left marks the pick. Columns: 16, 20, 24, 32, 40 and 48 px
/// at real size, then the same enlarged 8 times.
#[cfg(test)]
fn sheet(back: [u8; 3], ink: [u8; 3]) -> Rgba {
    const SIZES: [u32; 6] = [16, 20, 24, 32, 40, 48];
    const ZOOM: u32 = 8;
    const GAP: u32 = 16;
    const MARGIN: u32 = 32;
    let row = 48 * ZOOM;
    let real: u32 = SIZES.iter().map(|s| s + GAP).sum();
    let zoomed: u32 = SIZES.iter().map(|s| s * ZOOM + GAP).sum();
    let width = MARGIN + real + MARGIN + zoomed - GAP + MARGIN;
    let height = Concept::ALL.len() as u32 * (3 * (row + GAP) + MARGIN) + MARGIN;
    let mut out = blank(width, height, back);
    let mut top = MARGIN;
    for concept in Concept::ALL {
        if concept == PICK {
            let bar = blank(6, 3 * row + 2 * GAP, mix(back, ink, 0.5));
            paint(&mut out, &bar, 10, top, 1);
        }
        for state in STATES {
            let mut left = MARGIN;
            for size in SIZES {
                let glyph = colorize(&concept_mask(concept, state, size), ink);
                paint(&mut out, &glyph, left, top + (row - size) / 2, 1);
                left += size + GAP;
            }
            left += MARGIN;
            for size in SIZES {
                let glyph = colorize(&concept_mask(concept, state, size), ink);
                paint(&mut out, &glyph, left, top + (row - size * ZOOM) / 2, ZOOM);
                left += size * ZOOM + GAP;
            }
            top += row + GAP;
        }
        top += MARGIN;
    }
    out
}

/// Rows: each concept's app icon on a light and on a dark background. Columns: every frame size at real size, then
/// enlarged 4 times.
#[cfg(test)]
fn app_sheet() -> Rgba {
    const ZOOM: u32 = 4;
    const GAP: u32 = 16;
    const MARGIN: u32 = 32;
    let row = 64 * ZOOM + 2 * GAP;
    let real: u32 = APP_ICON_SIZES.iter().map(|s| s + GAP).sum();
    let zoomed: u32 = APP_ICON_SIZES.iter().map(|s| s * ZOOM + GAP).sum();
    let width = MARGIN + real + zoomed + MARGIN;
    let mut out = blank(width, 2 * Concept::ALL.len() as u32 * row, [0; 3]);
    let mut top = 0;
    for concept in Concept::ALL {
        for back in [[0xf3; 3], [0x20; 3]] {
            paint(&mut out, &blank(width, row, back), 0, top, 1);
            let mut left = MARGIN;
            for size in APP_ICON_SIZES {
                paint(
                    &mut out,
                    &concept_app_icon(concept, size),
                    left,
                    top + (row - size) / 2,
                    1,
                );
                left += size + GAP;
            }
            for size in APP_ICON_SIZES {
                let icon = concept_app_icon(concept, size);
                paint(&mut out, &icon, left, top + (row - size * ZOOM) / 2, ZOOM);
                left += size * ZOOM + GAP;
            }
            top += row;
        }
    }
    out
}

/// Writes `assets/wakeshot.ico` and the review sheets in `docs/research/img/ux/`, or all of them into
/// `WAKESHOT_ART_DIR` when it is set (a trial that leaves the repo alone). Run: `cargo test --locked -p
/// wakeshot-windows tray::art::write_art_files -- --ignored`.
#[cfg(test)]
#[test]
#[ignore = "writes the app icon and the review sheets"]
fn write_art_files() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (icon_dir, img_dir) = match std::env::var_os("WAKESHOT_ART_DIR") {
        Some(dir) => (dir.clone().into(), dir.into()),
        None => (root.join("assets"), root.join("docs/research/img/ux")),
    };
    let write = |path: std::path::PathBuf, bytes: Vec<u8>| {
        std::fs::write(&path, bytes).expect("write");
        println!("wrote {}", path.display());
    };
    write(icon_dir.join("wakeshot.ico"), app_icon_file());
    for (name, back, ink) in [
        ("dark", TASKBAR_DARK, GLYPH_ON_DARK),
        ("light", TASKBAR_LIGHT, GLYPH_ON_LIGHT),
        // A sample high contrast pair: yellow on black.
        ("contrast", [0, 0, 0], [0xff, 0xff, 0]),
    ] {
        write(
            img_dir.join(format!("icon-sheet-{name}.png")),
            png(&sheet(back, ink)),
        );
    }
    write(img_dir.join("icon-app.png"), png(&app_sheet()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(mask: &Mask) -> usize {
        mask.alpha.iter().filter(|&&a| a >= 128).count()
    }

    #[test]
    fn every_state_draws_at_every_size() {
        for concept in Concept::ALL {
            for state in STATES {
                for size in TRAY_SIZES {
                    let mask = concept_mask(concept, state, size);
                    assert_eq!(mask.size, size);
                    assert_eq!(mask.alpha.len(), (size * size) as usize);
                    // A glyph, not an empty box or a filled one.
                    let share = ink(&mask) as f32 / (size * size) as f32;
                    assert!(
                        (0.08..0.6).contains(&share),
                        "{concept:?} {state:?} {size}: ink {share:.2}"
                    );
                }
            }
        }
    }

    #[test]
    fn states_differ_in_shape() {
        // Shape, not colour: at least a quarter of the ink of two states sits in different pixels.
        for concept in Concept::ALL {
            for size in [16, 24] {
                for (i, a) in STATES.iter().enumerate() {
                    for b in &STATES[i + 1..] {
                        let (ma, mb) = (
                            concept_mask(concept, *a, size),
                            concept_mask(concept, *b, size),
                        );
                        let (mut differ, mut union) = (0, 0);
                        for (pa, pb) in ma.alpha.iter().zip(&mb.alpha) {
                            let (ia, ib) = (*pa >= 128, *pb >= 128);
                            union += usize::from(ia || ib);
                            differ += usize::from(ia != ib);
                        }
                        let share = differ as f32 / union as f32;
                        assert!(share >= 0.25, "{concept:?} {a:?} {b:?} {size}: {share:.2}");
                    }
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

    /// The most grey the sunrise may leave at 16 to 32 px (measured: see the test).
    const SUNRISE_GREY: f32 = 0.35;

    #[test]
    fn strokes_are_crisp_at_small_sizes() {
        for concept in Concept::ALL {
            // Straight strokes sit on whole pixels, so only curves and diagonals leave grey pixels: the cup has 13% at
            // most (its round bottom and the off bar). One stroke off the grid would leave 50% or more. The sunrise's
            // empty sun is a 1 px curve at 16 and 20 px (up to SUNRISE_GREY there); its rays are whole pixels. Eye and
            // Sky are mostly curves (up to 42%) and stay only until `p5-haik-check`.
            let bound = match concept {
                Concept::Sunrise => SUNRISE_GREY,
                Concept::Cup => 0.15,
                Concept::Eye | Concept::Sky => 0.45,
            };
            for state in STATES {
                for size in [16, 20, 24, 32] {
                    let share = half_covered(&concept_mask(concept, state, size));
                    assert!(share <= bound, "{concept:?} {state:?} {size}: {share:.2}");
                }
            }
        }
    }

    fn luminance(rgb: [u8; 3]) -> f64 {
        let channel = |c: u8| {
            let v = f64::from(c) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2])
    }

    fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn glyph_contrast_on_taskbars() {
        // WCAG 2 non-text contrast: at least 3:1 on the taskbar and on the hover plate.
        for (glyph, back) in [
            (GLYPH_ON_DARK, TASKBAR_DARK),
            (GLYPH_ON_DARK, HOVER_DARK),
            (GLYPH_ON_LIGHT, TASKBAR_LIGHT),
            (GLYPH_ON_LIGHT, HOVER_LIGHT),
        ] {
            let ratio = contrast(glyph, back);
            assert!(ratio >= 3.0, "{glyph:?} on {back:?}: {ratio:.2}");
        }
        // The app icon's gold sun and white horizon on both ends of its plate, and the plate on light and dark
        // backgrounds.
        for c in [APP_LIGHT, APP_DARK] {
            assert!(contrast([0xff; 3], c) >= 3.0, "white on {c:?}");
            assert!(contrast(SUN_GOLD, c) >= 3.0, "gold on {c:?}");
            assert!(contrast([0xf3; 3], c) >= 3.0, "{c:?} on light");
            assert!(contrast([0x20; 3], c) >= 3.0, "{c:?} on dark");
        }
    }

    #[test]
    fn glyph_stays_inside_its_box() {
        for concept in Concept::ALL {
            for state in STATES {
                for size in TRAY_SIZES {
                    let mask = concept_mask(concept, state, size);
                    let pad = padding(size);
                    let (mut left, mut top, mut right, mut bottom) = (size, size, 0, 0);
                    for y in 0..size {
                        for x in 0..size {
                            if mask.alpha[(y * size + x) as usize] == 0 {
                                continue;
                            }
                            let inside =
                                (pad..size - pad).contains(&x) && (pad..size - pad).contains(&y);
                            assert!(
                                inside,
                                "{concept:?} {state:?} {size}: ink at {x},{y}, pad {pad}"
                            );
                            (left, top) = (left.min(x), top.min(y));
                            (right, bottom) = (right.max(x + 1), bottom.max(y + 1));
                        }
                    }
                    // And it uses its box: no tiny glyph in a large slot.
                    let live = size - 2 * pad;
                    let span = (right - left).max(bottom - top);
                    assert!(
                        span * 10 >= live * 8,
                        "{concept:?} {state:?} {size}: spans {span} of {live}"
                    );
                }
            }
        }
    }

    #[test]
    fn colorize_keeps_alpha_and_sets_colour() {
        let mask = tray_mask(IconState::Holding, 16);
        let img = colorize(&mask, [1, 2, 3]);
        assert_eq!((img.width, img.height), (16, 16));
        for (p, a) in img.pixels.chunks(4).zip(&mask.alpha) {
            assert_eq!(p, [1, 2, 3, *a]);
        }
    }

    #[test]
    fn app_icon_draws_at_every_frame_size() {
        for concept in Concept::ALL {
            for size in APP_ICON_SIZES {
                let icon = concept_app_icon(concept, size);
                assert_eq!((icon.width, icon.height), (size, size));
                assert_eq!(icon.pixels.len(), (size * size * 4) as usize);
                let opaque = icon.pixels.chunks(4).filter(|p| p[3] == 255).count();
                // The plate's green stays at 0x57 or below; the gold and white mark lifts it.
                let mark = icon
                    .pixels
                    .chunks(4)
                    .filter(|p| p[3] == 255 && p[1] > 0xa0)
                    .count();
                assert!(
                    opaque * 5 > (size * size * 2) as usize,
                    "{concept:?} {size}: plate"
                );
                assert!(mark * 12 > opaque, "{concept:?} {size}: mark");
            }
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
            assert_eq!(cleared(a), *b, "{} px", a.width);
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

        /// A Huffman code, read most significant bit first.
        fn code(&mut self, n: u32) -> u32 {
            (0..n).fold(0, |v, _| (v << 1) | self.bit())
        }
    }

    const LENGTH_BASE: [u32; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u32; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u32; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u32; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    /// The fixed Huffman literal and length codes of RFC 1951 section 3.2.6.
    fn fixed_symbol(r: &mut BitReader) -> u32 {
        let mut code = r.code(7);
        if code <= 0b001_0111 {
            return 256 + code;
        }
        code = (code << 1) | r.bit();
        if (0x30..=0xbf).contains(&code) {
            return code - 0x30;
        }
        if (0xc0..=0xc7).contains(&code) {
            return 280 + code - 0xc0;
        }
        code = (code << 1) | r.bit();
        assert!((0x190..=0x1ff).contains(&code), "bad fixed code {code:#x}");
        144 + code - 0x190
    }

    /// Inflates a zlib stream of stored and fixed Huffman blocks, the two kinds a small writer emits, and checks its
    /// header and Adler-32.
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
            match r.bits(2) {
                0 => {
                    r.pos = r.pos.div_ceil(8) * 8;
                    let at = r.pos / 8;
                    let len = le16(body, at) as usize;
                    assert_eq!(le16(body, at + 2) as usize, !len & 0xffff, "NLEN");
                    out.extend(&body[at + 4..at + 4 + len]);
                    r.pos = (at + 4 + len) * 8;
                }
                1 => loop {
                    let sym = fixed_symbol(&mut r);
                    match sym {
                        0..=255 => out.push(sym as u8),
                        256 => break,
                        _ => {
                            let i = (sym - 257) as usize;
                            let len = LENGTH_BASE[i] + r.bits(LENGTH_EXTRA[i]);
                            let d = r.code(5) as usize;
                            let dist = (DIST_BASE[d] + r.bits(DIST_EXTRA[d])) as usize;
                            assert!(dist <= out.len(), "distance before the start");
                            for _ in 0..len {
                                out.push(out[out.len() - dist]);
                            }
                        }
                    }
                },
                other => panic!("block type {other}"),
            }
            if last == 1 {
                break;
            }
        }
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
        let glyph = colorize(&tray_mask(IconState::Holding, 48), [0xff, 0xff, 0]);
        let cases = [
            (pattern(37, 23), 6),
            (pattern(1, 1), 3),
            (flat, 3),
            (glyph, 3),
            // The violet plate's gradient is narrow, so the app icon fits a palette.
            (app_icon(64), 3),
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
    fn png_compresses_flat_areas() {
        // The review sheets are mostly one colour; a stored PNG of one would be tens of megabytes.
        let flat = Rgba {
            width: 1000,
            height: 1000,
            pixels: [0x20, 0x20, 0x21, 255].repeat(1000 * 1000),
        };
        assert!(png(&flat).len() < 60_000);
    }

    #[test]
    fn crc_table_matches_the_bitwise_crc() {
        assert_eq!(crc_check(b"IEND"), 0xae42_6082);
        assert_eq!(crc_check(b"123456789"), 0xcbf4_3926);
    }

    fn icon_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/wakeshot.ico")
    }

    #[test]
    fn app_icon_file_matches_the_art() {
        let file = std::fs::read(icon_path()).expect("assets/wakeshot.ico");
        let frames = parse_ico(&file);
        let sizes: Vec<u32> = frames.iter().map(|f| f.width).collect();
        assert_eq!(sizes, APP_ICON_SIZES, "frame sizes");
        for frame in &frames {
            // Exact: the shapes use no sin or cos, so x64 and ARM64 draw the same bytes.
            assert!(
                *frame == cleared(&app_icon(frame.width)),
                "the {} px frame differs from the art: run `cargo test --locked -p wakeshot-windows \
                 tray::art::write_art_files -- --ignored`",
                frame.width
            );
        }
    }
}
