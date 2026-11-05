// The Sunrise art, ported from `crates/wakeshot-windows/src/tray/art.rs` (functions `sunrise`, `horizon_and_sun`,
// `risen_sun`, `empty_sun`, `slashed`, `app_icon`, `raster`). The Rust file is the source of truth; this port
// exists because the video and the loop need the art as vectors at any size and in between two states.
// `scripts/check-art.mjs` rasters this port the way the Rust code does and compares it with the files the Rust code
// wrote, so a change to the art in Rust fails the check until this file follows.
//
// Plain types only (no enums, no parameter properties), so Node can run this file without a build step.

export type IconState = 'holding' | 'idle' | 'off';

export type Shape =
  | {k: 'disc'; cx: number; cy: number; r: number}
  | {k: 'rect'; x0: number; y0: number; x1: number; y1: number; r: [number, number, number, number]}
  | {k: 'line'; ax: number; ay: number; bx: number; by: number; w: number}
  | {k: 'half'; nx: number; ny: number; c: number}
  | {k: 'any'; of: Shape[]}
  | {k: 'all'; of: Shape[]}
  | {k: 'minus'; a: Shape; b: Shape};

export const GLYPH_ON_DARK: Rgb = [0xff, 0xff, 0xff];
export const GLYPH_ON_LIGHT: Rgb = [0x1a, 0x1a, 0x1a];
export const TASKBAR_DARK: Rgb = [0x20, 0x20, 0x21];
export const TASKBAR_LIGHT: Rgb = [0xd8, 0xda, 0xdc];
export const APP_LIGHT: Rgb = [0x79, 0x57, 0xf4];
export const APP_DARK: Rgb = [0x6b, 0x49, 0xf3];
export const SUN_GOLD: Rgb = [0xff, 0xcc, 0x4d];

export type Rgb = [number, number, number];

// Rust's `f32::round` goes half away from zero; every value here is positive, where `Math.round` does the same.
const round = Math.round;
const floor = Math.floor;

export function stroke(size: number): number {
  return Math.max(1, round(size / 16));
}

export type Grid = {x: number; y: number; k: number; t: number};

export function grid(x: number, y: number, size: number): Grid {
  return {x, y, k: size / 16, t: stroke(size)};
}

const ex = (g: Grid, u: number) => g.x + round(u * g.k);
const ey = (g: Grid, u: number) => g.y + round(u * g.k);
const len = (g: Grid, u: number) => u * g.k;
const centre = (g: Grid, v: number) => round(v - g.t / 2) + g.t / 2;

export type HorizonAndSun = {
  bar: {x0: number; y0: number; x1: number; y1: number; r: number};
  cx: number;
  r: number;
  base: number;
};

export function horizonAndSun(g: Grid, radius: number, horizon: number): HorizonAndSun {
  const t = g.t;
  const top = ey(g, horizon);
  const cx = centre(g, g.x + len(g, 8));
  const r = round(cx + len(g, radius)) - cx;
  return {
    bar: {x0: ex(g, 1), y0: top, x1: ex(g, 15), y1: top + t, r: floor(t / 2)},
    cx,
    r,
    base: top - t,
  };
}

const barShape = (b: HorizonAndSun['bar']): Shape => ({
  k: 'rect',
  x0: b.x0,
  y0: b.y0,
  x1: b.x1,
  y1: b.y1,
  r: [b.r, b.r, b.r, b.r],
});

/** The holding glyph's numbers: what the SVG and the raster both draw. */
export type Risen = HorizonAndSun & {
  gap: number;
  len: number;
  inner: number;
  /** Diagonal rays from `a` to `b` along 45 degrees from the sun's centre (strokes of width t, round caps). */
  a: number;
  b: number;
};

export function risen(g: Grid): Risen {
  const t = g.t;
  const hs = horizonAndSun(g, 4, 13);
  const gap = Math.max(round(len(g, 1.25)), t);
  const rayLen = Math.max(round(len(g, 3)), t);
  const inner = floor(hs.base - hs.r - gap);
  return {...hs, gap, len: rayLen, inner, a: hs.r + gap + t / 2, b: hs.r + gap + rayLen - t / 2};
}

export function risenShapes(g: Grid): {horizon: Shape; sun: Shape} {
  const t = g.t;
  const s = risen(g);
  const {cx, r, base} = s;
  const parts: Shape[] = [
    {k: 'all', of: [{k: 'disc', cx, cy: base, r}, {k: 'half', nx: 0, ny: 1, c: base}]},
    {k: 'line', ax: cx, ay: s.inner - t / 2, bx: cx, by: s.inner - s.len + t / 2, w: t},
  ];
  if (t < 1.5) {
    const col = Math.trunc(cx - 0.5);
    const row = Math.trunc(base);
    const reach = r + s.gap + 0.5;
    let first = 0;
    for (let j = 0; j < row; j++) {
      const dx = 1 + j;
      const dy = 0.5 + j;
      if (dx * dx + dy * dy >= reach * reach) {
        first = j;
        break;
      }
    }
    const steps = Math.max(round(s.len / Math.SQRT2), 1);
    const dot = (x: number, y: number): Shape => ({k: 'rect', x0: x, y0: y, x1: x + 1, y1: y + 1, r: [0, 0, 0, 0]});
    for (let j = first; j < first + steps; j++) {
      if (j + 1 > col || j + 1 > row) break;
      parts.push(dot(col - 1 - j, row - 1 - j));
      parts.push(dot(col + 1 + j, row - 1 - j));
    }
  } else {
    const d = Math.SQRT1_2;
    for (const side of [-1, 1]) {
      parts.push({k: 'line', ax: cx + side * d * s.a, ay: base - d * s.a, bx: cx + side * d * s.b, by: base - d * s.b, w: t});
    }
  }
  return {horizon: barShape(s.bar), sun: {k: 'any', of: parts}};
}

export function idle(g: Grid): HorizonAndSun {
  return horizonAndSun(g, 5, 12);
}

function emptySun(g: Grid, off: boolean): Shape {
  const t = g.t;
  const s = idle(g);
  const ring: Shape = {
    k: 'all',
    of: [
      {k: 'minus', a: {k: 'disc', cx: s.cx, cy: s.base, r: s.r}, b: {k: 'disc', cx: s.cx, cy: s.base, r: s.r - t}},
      {k: 'half', nx: 0, ny: 1, c: s.base},
    ],
  };
  let horizon = barShape(s.bar);
  if (off) {
    horizon = {k: 'all', of: [horizon, {k: 'half', nx: 1, ny: -1, c: g.x - g.y}]};
  }
  return {k: 'any', of: [horizon, ring]};
}

/** The off bar, from `slashed`. */
export function offBar(g: Grid, w: number) {
  const t = g.t;
  return {ax: ex(g, 1) + t / 2, ay: ey(g, 1) + t / 2, bx: ex(g, 15) - t / 2, by: ey(g, 15) - t / 2, w};
}

export function glyphShape(state: IconState, g: Grid): Shape {
  if (state === 'holding') {
    const {horizon, sun} = risenShapes(g);
    return {k: 'any', of: [horizon, sun]};
  }
  if (state === 'idle') return emptySun(g, false);
  const bar = (w: number): Shape => ({k: 'line', ...offBar(g, w)});
  return {k: 'any', of: [{k: 'minus', a: emptySun(g, true), b: bar(3 * g.t)}, bar(g.t)]};
}

export function contains(s: Shape, x: number, y: number): boolean {
  switch (s.k) {
    case 'disc': {
      const dx = x - s.cx;
      const dy = y - s.cy;
      return dx * dx + dy * dy <= s.r * s.r;
    }
    case 'rect': {
      if (x < s.x0 || x > s.x1 || y < s.y0 || y > s.y1) return false;
      const left = x < (s.x0 + s.x1) / 2;
      const top = y < (s.y0 + s.y1) / 2;
      const r = top ? (left ? s.r[0] : s.r[1]) : left ? s.r[3] : s.r[2];
      const cx = left ? s.x0 + r : s.x1 - r;
      const cy = top ? s.y0 + r : s.y1 - r;
      const inCorner = (left ? x < cx : x > cx) && (top ? y < cy : y > cy);
      const dx = x - cx;
      const dy = y - cy;
      return !inCorner || dx * dx + dy * dy <= r * r;
    }
    case 'line': {
      const vx = s.bx - s.ax;
      const vy = s.by - s.ay;
      const len2 = vx * vx + vy * vy;
      const t = len2 > 0 ? Math.min(1, Math.max(0, ((x - s.ax) * vx + (y - s.ay) * vy) / len2)) : 0;
      const dx = x - (s.ax + t * vx);
      const dy = y - (s.ay + t * vy);
      return dx * dx + dy * dy <= (s.w * s.w) / 4;
    }
    case 'half':
      return s.nx * x + s.ny * y <= s.c;
    case 'any':
      return s.of.some((p) => contains(p, x, y));
    case 'all':
      return s.of.every((p) => contains(p, x, y));
    case 'minus':
      return contains(s.a, x, y) && !contains(s.b, x, y);
  }
}

const SAMPLES = 8;

/** Coverage per pixel, 0 to 255, exactly as `raster` in art.rs samples it. */
export function raster(width: number, height: number, shape: Shape): Uint8Array {
  const n = SAMPLES * SAMPLES;
  const step = 1 / SAMPLES;
  const out = new Uint8Array(width * height);
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      let hits = 0;
      for (let sy = 0; sy < SAMPLES; sy++) {
        for (let sx = 0; sx < SAMPLES; sx++) {
          if (contains(shape, px + (sx + 0.5) * step, py + (sy + 0.5) * step)) hits++;
        }
      }
      out[py * width + px] = floor((hits * 255 + floor(n / 2)) / n);
    }
  }
  return out;
}

export function mix(a: Rgb, b: Rgb, t: number): Rgb {
  const one = (p: number, q: number) => round(p + (q - p) * t);
  return [one(a[0], b[0]), one(a[1], b[1]), one(a[2], b[2])];
}

/** The app icon's layout numbers at `size`, from `app_icon`. */
export function appLayout(size: number) {
  const c = size / 2;
  const margin = floor((size + 12) / 24);
  const half = c - margin;
  const corner = Math.max(size * 0.1875, 1);
  let mark = round(size * 0.72);
  if ((size - mark) % 2 === 1) mark -= 1;
  const at = (size - mark) / 2;
  return {centre: c, margin, half, corner, mark, at, grid: grid(at, at, mark)};
}

/** The app icon as straight RGBA, as `app_icon(size)` draws it. */
export function appIconRgba(size: number): Uint8Array {
  const L = appLayout(size);
  const plate = raster(size, size, {
    k: 'rect',
    x0: L.margin,
    y0: L.margin,
    x1: size - L.margin,
    y1: size - L.margin,
    r: [L.corner, L.corner, L.corner, L.corner],
  });
  const {horizon, sun} = risenShapes(L.grid);
  const layers: [Uint8Array, Rgb][] = [
    [raster(size, size, sun), SUN_GOLD],
    [raster(size, size, horizon), [0xff, 0xff, 0xff]],
  ];
  const dx = Math.sqrt(3) / 2;
  const dy = 0.5;
  const out = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = y * size + x;
      if (plate[i] === 0) continue;
      const along = ((x + 0.5 - L.centre) * dx + (y + 0.5 - L.centre) * dy) / L.half;
      let rgb = mix(APP_LIGHT, APP_DARK, Math.min(1, Math.max(0, (along + 1) / 2)));
      for (const [m, colour] of layers) rgb = mix(rgb, colour, m[i] / 255);
      out.set([rgb[0], rgb[1], rgb[2], plate[i]], i * 4);
    }
  }
  return out;
}
