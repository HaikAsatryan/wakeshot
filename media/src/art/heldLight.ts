// The held light in Moss, ported from `crates/wakeshot-windows/src/tray/art.rs`: the tray glyph (`Held`,
// `tray_mask`, `struck`, `strike_pixels`, `colorize`), the app icon (`Plated`, `draw_plated`, `app_icon`,
// `app_icon_flat`) and the rasterizer (`Shape`, `raster`), plus `mix` and `paint` from its tests, which compose the
// review sheets. art.rs is the one source of the art. This port exists because the loop and the video draw the art at
// sizes art.rs never writes (the tray glyph at 128 px) and in between two states.
//
// art.rs computes in f32. JavaScript computes in f64, so every step here is rounded to f32 with `F` (Math.fround),
// one operation at a time and in the order Rust evaluates it. A double rounded once to f32 after a single +, -, *, /
// or sqrt of two f32 values is exactly the f32 result (a double has more than twice the bits plus two), so the port
// draws the same bytes; an expression written without `F` around each step would not. `scripts/check-art.mjs`
// compares the port with every file art.rs writes and allows no difference at all.
//
// Plain types only (no enums, no parameter properties), so Node runs this file without a build step.

export type Rgb = [number, number, number];
export type IconState = 'holding' | 'idle' | 'off';

/** The colours of a tray glyph: `ink` for the line and the outline, `light` for the filled light. */
export type GlyphColours = {ink: Rgb; light: Rgb};

/** A tray glyph: `alpha` is the whole glyph, `light` the part that is the filled light. Top-down rows. */
export type Mask = {size: number; alpha: Uint8Array; light: Uint8Array};

/** Straight (not premultiplied) RGBA, top-down rows. */
export type Rgba = {width: number; height: number; pixels: Uint8Array};

export const APP_ICON_SIZES = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];

export const GLYPH_ON_DARK: Rgb = [0xff, 0xff, 0xff];
export const GLYPH_ON_LIGHT: Rgb = [0x1a, 0x1a, 0x1a];
export const HOLDING_ON_DARK: Rgb = [0xb6, 0xe0, 0x5a];
export const HOLDING_ON_LIGHT: Rgb = [0x3b, 0x7a, 0x2a];
export const TASKBAR_DARK: Rgb = [0x20, 0x20, 0x21];
export const TASKBAR_LIGHT: Rgb = [0xd8, 0xda, 0xdc];

export const ON_DARK: GlyphColours = {ink: GLYPH_ON_DARK, light: HOLDING_ON_DARK};
export const ON_LIGHT: GlyphColours = {ink: GLYPH_ON_LIGHT, light: HOLDING_ON_LIGHT};

/** Every part in one colour, as high contrast asks (`GlyphColours::one`). */
export const one = (rgb: Rgb): GlyphColours => ({ink: rgb, light: rgb});

export const PLATE_LIGHT: Rgb = [0x62, 0x8c, 0x2b];
export const PLATE_DARK: Rgb = [0x3e, 0x7c, 0x3b];
export const PLATE_DEEP: Rgb = [0x2c, 0x5a, 0x26];
export const LIGHT_CORE: Rgb = [0xff, 0xfb, 0xe8];
export const LIGHT_EDGE: Rgb = [0xff, 0xea, 0x9c];
export const LIGHT_RIM: Rgb = [0xf7, 0xdc, 0x7f];
export const WHITE: Rgb = [0xff, 0xff, 0xff];

/** From this size up the app icon is lit (glow, bloom, shade, shadow); below it the frames are flat. */
export const LIT_FROM = 30;

const F = Math.fround;

/** Rust's `f32::round`: half away from zero (`Math.round` goes half up, which differs below zero). */
const round = (x: number) => (x < 0 ? -Math.round(-x) : Math.round(x));
/** Rust's `as u32` and `as u8` from f32: toward zero, saturating, NaN to 0. */
const toU32 = (x: number) => (x > 0 ? Math.trunc(x) : 0);
const toU8 = (x: number) => (x > 0 ? Math.min(255, Math.trunc(x)) : 0);
const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));
/** Rust's u32 division. */
const div = (a: number, b: number) => Math.floor(a / b);

const FRAC_PI_4 = F(Math.PI / 4);

// ---- Shapes and the rasterizer -------------------------------------------------------------------------------

/** One shape, in pixels of the canvas, y down. Rect corner radii: top left, top right, bottom right, bottom left. */
export type Shape =
  | {k: 'disc'; cx: number; cy: number; r: number}
  | {k: 'rect'; x0: number; y0: number; x1: number; y1: number; r: [number, number, number, number]}
  | {k: 'line'; ax: number; ay: number; bx: number; by: number; w: number}
  | {k: 'any'; of: Shape[]}
  | {k: 'minus'; a: Shape; b: Shape};

export const minus = (a: Shape, b: Shape): Shape => ({k: 'minus', a, b});

export function contains(s: Shape, x: number, y: number): boolean {
  switch (s.k) {
    case 'disc': {
      const dx = F(x - s.cx);
      const dy = F(y - s.cy);
      return F(F(dx * dx) + F(dy * dy)) <= F(s.r * s.r);
    }
    case 'rect': {
      if (x < s.x0 || x > s.x1 || y < s.y0 || y > s.y1) return false;
      const left = x < F(F(s.x0 + s.x1) / 2);
      const top = y < F(F(s.y0 + s.y1) / 2);
      const r = top ? (left ? s.r[0] : s.r[1]) : left ? s.r[3] : s.r[2];
      const cx = left ? F(s.x0 + r) : F(s.x1 - r);
      const cy = top ? F(s.y0 + r) : F(s.y1 - r);
      const inCorner = (left ? x < cx : x > cx) && (top ? y < cy : y > cy);
      const dx = F(x - cx);
      const dy = F(y - cy);
      return !inCorner || F(F(dx * dx) + F(dy * dy)) <= F(r * r);
    }
    case 'line': {
      const vx = F(s.bx - s.ax);
      const vy = F(s.by - s.ay);
      const len2 = F(F(vx * vx) + F(vy * vy));
      const t = len2 > 0 ? clamp(F(F(F(F(x - s.ax) * vx) + F(F(y - s.ay) * vy)) / len2), 0, 1) : 0;
      const dx = F(x - F(s.ax + F(t * vx)));
      const dy = F(y - F(s.ay + F(t * vy)));
      return F(F(dx * dx) + F(dy * dy)) <= F(F(s.w * s.w) / 4);
    }
    case 'any':
      return s.of.some((p) => contains(p, x, y));
    case 'minus':
      return contains(s.a, x, y) && !contains(s.b, x, y);
  }
}

const SAMPLES = 8;

/** Coverage per pixel, 0 to 255, from 8 by 8 samples per pixel (`raster`). The sample points are exact in f32. */
export function raster(width: number, height: number, shape: Shape): Uint8Array {
  const n = SAMPLES * SAMPLES;
  const out = new Uint8Array(width * height);
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      let hits = 0;
      for (let sy = 0; sy < SAMPLES; sy++) {
        for (let sx = 0; sx < SAMPLES; sx++) {
          if (contains(shape, px + (sx + 0.5) / SAMPLES, py + (sy + 0.5) / SAMPLES)) hits++;
        }
      }
      out[py * width + px] = div(hits * 255 + div(n, 2), n);
    }
  }
  return out;
}

// ---- The tray glyph ------------------------------------------------------------------------------------------

/** Stroke width in whole pixels, about 1 px per 16 px of icon (`stroke`). */
export function stroke(size: number): number {
  return toU32(Math.max(round(F(size / 16)), 1));
}

/** The held light at one tray size, in whole pixels (`Held`). */
export type Held = {t: number; d: number; top: number; lineY: number; x0: number; x1: number};

export function held(size: number): Held {
  const t = stroke(size);
  let d = div(size, 2);
  if ((d + size) % 2 === 1) d -= 1;
  const gap = div(size + 4, 8);
  const width = size - 2 * div(size, 8);
  const disc = F(F(FRAC_PI_4 * d) * d);
  const line = width * t;
  const below = F(F(F(F(disc * d) / 2) + F(line * F(F(d + gap) + F(t / 2)))) / F(disc + line));
  const top = toU32(Math.max(round(F(F(F(0.455) * size) - below)), 0));
  const x0 = div(size - width, 2);
  return {t, d, top, lineY: top + d + gap, x0, x1: x0 + width};
}

export function heldLine(h: Held): Shape {
  const r = h.t >= 2 ? F(h.t / 2) : 0;
  return {k: 'rect', x0: h.x0, y0: h.lineY, x1: h.x1, y1: h.lineY + h.t, r: [r, r, r, r]};
}

export function heldDisc(h: Held, size: number, inset: number): Shape {
  const r = F(h.d / 2);
  return {k: 'disc', cx: F(size / 2), cy: F(h.top + r), r: F(r - inset)};
}

const RING_8 = ['.sXXXXs.', 'sX....Xs', 'X......X', 'X......X', 'X......X', 'X......X', 'sX....Xs', '.sXXXXs.'];
const RING_10 = [
  '..sXXXXs..',
  '.sX....Xs.',
  'sX......Xs',
  'X........X',
  'X........X',
  'X........X',
  'X........X',
  'sX......Xs',
  '.sX....Xs.',
  '..sXXXXs..',
];

/** The 1 px outlines art.rs draws by hand (`hand_ring`). */
function handRing(h: Held): string[] | null {
  if (h.t === 1 && h.d === 8) return RING_8;
  if (h.t === 1 && h.d === 10) return RING_10;
  return null;
}

/** The outline: the line and the light's ring (`tray_mask`'s `outline`). */
export function heldOutline(h: Held, size: number): Shape {
  return {k: 'any', of: [heldLine(h), minus(heldDisc(h, size, 0), heldDisc(h, size, h.t))]};
}

/** The tray glyph for `state` (`tray_mask`). */
export function trayMask(state: IconState, size: number): Mask {
  const h = held(size);
  const n = size * size;
  const outline = heldOutline(h, size);
  let alpha: Uint8Array;
  let light: Uint8Array = new Uint8Array(n);
  if (state === 'holding') {
    alpha = raster(size, size, heldLine(h));
    light = raster(size, size, heldDisc(h, size, 0));
  } else if (state === 'off' && h.t >= 2) {
    alpha = raster(size, size, struck(h, size, outline));
  } else {
    alpha = raster(size, size, outline);
  }
  if (state !== 'holding') {
    const rows = handRing(h);
    if (rows) {
      const left = div(size - h.d, 2);
      rows.forEach((row, j) => {
        for (let i = 0; i < row.length; i++) {
          const c = row[i];
          alpha[(h.top + j) * size + left + i] = c === 'X' ? 255 : c === 's' ? 64 : 0;
        }
      });
    }
    if (state === 'off' && h.t === 1) strikePixels(alpha, h, size);
  }
  for (let i = 0; i < n; i++) alpha[i] = Math.min(255, alpha[i] + light[i]);
  return {size, alpha, light};
}

/**
 * The square the off slash crosses (`strike_box`): left, top, right, bottom. From the light's top left corner through
 * its centre, down to a stroke above the line, so the line stays whole.
 */
function strikeBox(h: Held, size: number): [number, number, number, number] {
  const left = div(size - h.d, 2);
  const bottom = h.lineY - h.t;
  return [left, h.top, left + (bottom - h.top), bottom];
}

/** Off at 2 px strokes and up: the outline struck through with a stroke-wide gap each side (`struck`). */
function struck(h: Held, size: number, base: Shape): Shape {
  const t = h.t;
  const [x0, y0, x1, y1] = strikeBox(h, size);
  const half = F(t / 2);
  const slash = (w: number): Shape => ({
    k: 'line',
    ax: F(x0 + half),
    ay: F(y0 + half),
    bx: F(x1 - half),
    by: F(y1 - half),
    w,
  });
  return {k: 'any', of: [minus(base, slash(F(3 * t))), slash(t)]};
}

/** Off at 1 px strokes: whole pixels down the diagonal, every neighbour cleared (`strike_pixels`). */
function strikePixels(alpha: Uint8Array, h: Held, size: number) {
  const [x0, y0, , y1] = strikeBox(h, size);
  const steps = y1 - y0;
  const set = (x: number, y: number, v: number) => {
    if (x >= 0 && x < size && y >= 0 && y < size) alpha[y * size + x] = v;
  };
  for (let i = 0; i < steps; i++) {
    const [x, y] = [x0 + i, y0 + i];
    set(x + 1, y, 0);
    set(x, y + 1, 0);
    set(x - 1, y, 0);
    set(x, y - 1, 0);
    set(x + 1, y - 1, 0);
    set(x - 1, y + 1, 0);
  }
  for (let i = 0; i < steps; i++) set(x0 + i, y0 + i, 255);
}

/** The glyph in its colours (`colorize`). */
export function colorize(mask: Mask, colours: GlyphColours): Rgba {
  const n = mask.size * mask.size;
  const pixels = new Uint8Array(n * 4);
  for (let i = 0; i < n; i++) {
    const a = mask.alpha[i];
    const l = mask.light[i];
    let rgb: Rgb;
    if (l === 0) rgb = colours.ink;
    else if (l >= a) rgb = colours.light;
    else {
      const mixOne = (ink: number, light: number) => div(l * light + (a - l) * ink + div(a, 2), a);
      rgb = [0, 1, 2].map((c) => mixOne(colours.ink[c], colours.light[c])) as Rgb;
    }
    pixels.set([rgb[0], rgb[1], rgb[2], a], i * 4);
  }
  return {width: mask.size, height: mask.size, pixels};
}

// ---- The app icon --------------------------------------------------------------------------------------------

/** The app icon's layout at one size, in pixels (`Plated`). */
export type Plated = {
  size: number;
  lit: boolean;
  margin: number;
  side: number;
  corner: number;
  cx: number;
  cy: number;
  r: number;
  /** Left, top, right, bottom. */
  bar: [number, number, number, number];
};

export function plated(size: number, lit: boolean): Plated {
  const s = size;
  const margin = div(size + 12, 24);
  const side = F(s - F(2 * margin));
  const r = round(F(F(s * 3) / 16));
  const barH = Math.max(round(F(s / 16)), 1);
  const gap = Math.max(round(F(s * F(0.082))), F(barH + 1));
  const top = round(F(F(F(s - F(F(F(2 * r) + gap) + barH)) / 2) - F(F(0.014) * s)));
  const barW = F(2 * round(F(s * F(0.2735))));
  const barY = F(F(top + F(2 * r)) + gap);
  const barX = F(F(s - barW) / 2);
  return {
    size,
    lit,
    margin,
    side,
    corner: F(F(side * 3) / 16),
    cx: F(s / 2),
    cy: F(top + r),
    r,
    bar: [barX, barY, F(barX + barW), F(barY + barH)],
  };
}

export function plateShape(p: Plated, dy: number): Shape {
  const far = F(p.size - p.margin);
  const c = p.corner;
  return {k: 'rect', x0: p.margin, y0: F(p.margin + dy), x1: far, y1: F(far + dy), r: [c, c, c, c]};
}

export function barShape(p: Plated): Shape {
  const [x0, y0, x1, y1] = p.bar;
  const h = F(y1 - y0);
  const r = h >= 2 ? F(h / 2) : 0;
  return {k: 'rect', x0, y0, x1, y1, r: [r, r, r, r]};
}

export function lightShape(p: Plated): Shape {
  return {k: 'disc', cx: p.cx, cy: p.cy, r: p.r};
}

export type Rgbf = [number, number, number];

export const f = (rgb: Rgb): Rgbf => [rgb[0], rgb[1], rgb[2]];

export function mixf(a: Rgbf, b: Rgbf, t: number): Rgbf {
  return [F(a[0] + F(F(b[0] - a[0]) * t)), F(a[1] + F(F(b[1] - a[1]) * t)), F(a[2] + F(F(b[2] - a[2]) * t))];
}

/** A value along straight segments between `[t, value]` stops, flat past both ends (`stops`). */
function stops(points: [number, number][], t: number): number {
  const [first, last] = [points[0], points[points.length - 1]];
  if (t <= first[0]) return first[1];
  for (let i = 0; i + 1 < points.length; i++) {
    const [[t0, v0], [t1, v1]] = [points[i], points[i + 1]];
    if (t <= t1) return F(v0 + F(F(F(v1 - v0) * F(t - t0)) / F(t1 - t0)));
  }
  return last[1];
}

function distance(p: Plated, x: number, y: number): number {
  const dx = F(x - p.cx);
  const dy = F(y - p.cy);
  return F(Math.sqrt(F(F(dx * dx) + F(dy * dy))));
}

const SQRT3_HALF = F(F(Math.sqrt(3)) / 2);

/**
 * The plate's colour at a point, without the bloom: the ramp, then when lit the glow and the shade (`plate_colour`).
 * art.rs always adds the glow to a lit plate; `glow` false is the media's own, for the title's unlit icon
 * (`motion.ts`, `appIconUnlit`).
 */
export function plateColour(p: Plated, x: number, y: number, glow: boolean): Rgbf {
  const [m, w] = [p.margin, p.side];
  const [dx, dy] = [SQRT3_HALF, 0.5];
  const along = F(F(F(F(x - m) * dx) + F(F(y - m) * dy)) / F(w * F(dx + dy)));
  let rgb = mixf(f(PLATE_LIGHT), f(PLATE_DARK), clamp(along, 0, 1));
  if (p.lit) {
    const [reach, strength] = p.size >= 64 ? [F(0.62), F(0.6)] : [F(0.5), F(0.42)];
    if (glow) {
      const t = F(distance(p, x, y) / F(reach * w));
      const alpha = stops(
        [
          [0, strength],
          [F(0.45), F(F(0.35) * strength)],
          [1, 0],
        ],
        t,
      );
      rgb = mixf(rgb, f(LIGHT_EDGE), alpha);
    }
    const down = F(F(y - m) / w);
    const shade = clamp(F(F(F(0.35) * F(down - F(0.55))) / F(0.45)), 0, F(0.35));
    rgb = mixf(rgb, f(PLATE_DEEP), shade);
  }
  return rgb;
}

/** The light's colour at a point: a radial ramp from a spot up and left of its centre (`light_colour`). */
export function lightColour(p: Plated, x: number, y: number): Rgbf {
  const r = p.r;
  const [fx, fy] = [F(p.cx - F(F(0.22) * r)), F(p.cy - F(F(0.28) * r))];
  const [vx, vy] = [F(x - fx), F(y - fy)];
  const [ex, ey] = [F(p.cx - fx), F(p.cy - fy)];
  const a = F(F(F(ex * ex) + F(ey * ey)) - F(r * r));
  const b = F(F(vx * ex) + F(vy * ey));
  const c = F(F(vx * vx) + F(vy * vy));
  const t = clamp(F(F(b - F(Math.sqrt(F(F(b * b) - F(a * c))))) / a), 0, 1);
  if (p.lit) {
    const rgb = mixf(f(LIGHT_CORE), f(LIGHT_EDGE), clamp(F(F(t - 0.5) / F(0.36)), 0, 1));
    return mixf(rgb, f(LIGHT_RIM), clamp(F(F(t - F(0.86)) / F(0.14)), 0, 1));
  }
  return mixf(f(LIGHT_CORE), f(LIGHT_EDGE), clamp(F(F(t - F(0.6)) / F(0.4)), 0, 1));
}

/** Where the bloom ends, in light radii (`BLOOM_END`). */
const BLOOM_END = F(1.6);

/** The bloom's alpha by distance from the light's centre, in light radii (`BLOOM`). */
const BLOOM: [number, number][] = [
  [0, F(0.8)],
  [1, F(0.44)],
  [F(1.28), F(0.144)],
  [BLOOM_END, 0],
];

/** The light's soft bloom round its edge, as an alpha (`bloom`). */
export function bloom(p: Plated, x: number, y: number): number {
  return stops(BLOOM, F(distance(p, x, y) / p.r));
}

/** One box blur pass across, then one down, `width` pixels wide; outside the image counts as clear (`box_blur`). */
function boxBlur(alpha: Uint8Array, size: number, width: number) {
  const [n, half] = [size, div(width, 2)];
  const line = new Uint8Array(n);
  for (const across of [true, false]) {
    for (let k = 0; k < n; k++) {
      const at = (j: number) => (across ? k * n + j : j * n + k);
      for (let j = 0; j < n; j++) {
        const [lo, hi] = [Math.max(0, j - half), Math.min(j + half, n - 1)];
        let sum = 0;
        for (let q = lo; q <= hi; q++) sum += alpha[at(q)];
        line[j] = div(sum + div(width, 2), width);
      }
      for (let j = 0; j < n; j++) alpha[at(j)] = line[j];
    }
  }
}

/** The drop shadow's alpha under the plate (`shadow`). */
export function shadow(p: Plated): Uint8Array {
  const s = p.size;
  const alpha = raster(p.size, p.size, plateShape(p, F(s / 96)));
  const sigma = F(s / 64);
  let width: number;
  let passes: number;
  if (sigma < F(1.2)) {
    width = 3;
    passes = Math.max(toU32(round(F(F(F(1.5) * sigma) * sigma))), 1);
  } else {
    width = toU32(round(F(F(Math.sqrt(F(F(F(4 * sigma) * sigma) + 1))) / 2))) * 2 + 1;
    passes = 3;
  }
  for (let i = 0; i < passes; i++) boxBlur(alpha, p.size, width);
  return alpha;
}

/** How dark the drop shadow is at its darkest (`SHADOW`). */
export const SHADOW = F(0.3);

/** A pixel being painted: straight colour, 0 to 255, and alpha, 0 to 1 (`Paint`). */
export type Paint = {rgb: Rgbf; a: number};

export const clear = (): Paint => ({rgb: [0, 0, 0], a: 0});

/** Lays `rgb` at alpha `a` over the pixel, as a browser does in sRGB (`Paint::over`). */
export function over(px: Paint, rgb: Rgbf, a: number) {
  if (a <= 0) return;
  const under = F(px.a * F(1 - a));
  const out = F(a + under);
  for (let c = 0; c < 3; c++) px.rgb[c] = F(F(F(rgb[c] * a) + F(px.rgb[c] * under)) / out);
  px.a = out;
}

/** `Paint::bytes`. */
export function bytes(px: Paint): [number, number, number, number] {
  const a = toU8(round(F(px.a * 255)));
  if (a === 0) return [0, 0, 0, 0];
  const c = (v: number) => toU8(clamp(round(v), 0, 255));
  return [c(px.rgb[0]), c(px.rgb[1]), c(px.rgb[2]), a];
}

/** The coverage of one raster as an f32 alpha (`cover`). */
export const cover = (m: Uint8Array, i: number) => F(m[i] / 255);

/** The app icon (`app_icon`): lit from `LIT_FROM` up, flat below. */
export function appIcon(size: number): Rgba {
  return drawPlated(plated(size, size >= LIT_FROM));
}

/** The app icon drawn flat at any size (`app_icon_flat`). */
export function appIconFlat(size: number): Rgba {
  return drawPlated(plated(size, false));
}

export function drawPlated(p: Plated): Rgba {
  const size = p.size;
  const plate = raster(size, size, plateShape(p, 0));
  const bar = raster(size, size, barShape(p));
  const light = raster(size, size, lightShape(p));
  const shade = p.lit ? shadow(p) : new Uint8Array(size * size);
  const [barX0, , barX1] = p.bar;
  const pixels = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = y * size + x;
      const [fx, fy] = [x + 0.5, y + 0.5];
      const px = clear();
      // Each colour is worked out only where its layer covers the pixel: `over` skips a zero alpha, so the bytes are
      // the same as art.rs, which works them out everywhere.
      over(px, [0, 0, 0], F(SHADOW * cover(shade, i)));
      if (plate[i]) over(px, plateColour(p, fx, fy, true), cover(plate, i));
      if (bar[i]) {
        const along = clamp(F(F(fx - barX0) / F(barX1 - barX0)), 0, 1);
        over(px, mixf(f(WHITE), f(LIGHT_CORE), along), cover(bar, i));
      }
      if (p.lit && plate[i]) over(px, f(LIGHT_EDGE), F(bloom(p, fx, fy) * cover(plate, i)));
      if (light[i]) over(px, lightColour(p, fx, fy), cover(light, i));
      pixels.set(bytes(px), i * 4);
    }
  }
  return {width: size, height: size, pixels};
}

// ---- The review sheets (art.rs tests) ------------------------------------------------------------------------

/** `mix` from art.rs's tests: `a` to `b` by `t` in f32, rounded. */
export function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const m = (p: number, q: number) => toU8(round(F(p + F(F(q - p) * t))));
  return [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])];
}

/** `paint` from art.rs's tests: `img` over the opaque `sheet` at `left`, `top`, enlarged `zoom` times. */
export function paint(sheet: Rgba, img: Rgba, left: number, top: number, zoom: number) {
  for (let y = 0; y < img.height * zoom; y++) {
    for (let x = 0; x < img.width * zoom; x++) {
      const s = (div(y, zoom) * img.width + div(x, zoom)) * 4;
      const d = ((top + y) * sheet.width + left + x) * 4;
      const a = F(img.pixels[s + 3] / 255);
      const under: Rgb = [sheet.pixels[d], sheet.pixels[d + 1], sheet.pixels[d + 2]];
      sheet.pixels.set(mix(under, [img.pixels[s], img.pixels[s + 1], img.pixels[s + 2]], a), d);
    }
  }
}

/** An opaque image of one colour (`blank`). */
export function blank(width: number, height: number, rgb: Rgb): Rgba {
  const pixels = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) pixels.set([rgb[0], rgb[1], rgb[2], 255], i * 4);
  return {width, height, pixels};
}

/** The review sheet as art.rs's `sheet` draws it: rows holding in one colour, holding, idle, off. */
export function sheet(back: Rgb, colours: GlyphColours): Rgba {
  const SIZES = [16, 20, 24, 32, 40, 48];
  const [ZOOM, GAP, MARGIN] = [8, 16, 32];
  const rows: [IconState, GlyphColours][] = [
    ['holding', one(colours.ink)],
    ['holding', colours],
    ['idle', colours],
    ['off', colours],
  ];
  const row = 48 * ZOOM;
  const real = SIZES.reduce((sum, s) => sum + s + GAP, 0);
  const zoomed = SIZES.reduce((sum, s) => sum + s * ZOOM + GAP, 0);
  const width = MARGIN + real + MARGIN + zoomed - GAP + MARGIN;
  const height = MARGIN + rows.length * (row + GAP) - GAP + MARGIN;
  const out = blank(width, height, back);
  let top = MARGIN;
  for (const [state, ink] of rows) {
    let left = MARGIN;
    for (const size of SIZES) {
      paint(out, colorize(trayMask(state, size), ink), left, top + div(row - size, 2), 1);
      left += size + GAP;
    }
    left += MARGIN;
    for (const size of SIZES) {
      paint(out, colorize(trayMask(state, size), ink), left, top + div(row - size * ZOOM, 2), ZOOM);
      left += size * ZOOM + GAP;
    }
    top += row + GAP;
  }
  return out;
}
