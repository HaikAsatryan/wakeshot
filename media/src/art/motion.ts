// The art in between two states, which the app never draws but the loop and the video show: the light fading on.
// Both ends of every fade are the port's own pixels (heldLight.ts), byte for byte, so the first and last frames of a
// fade are exactly what Wakeshot draws; only the frames in between are made here.
//
// Plain types only, so Node runs this file too (`scripts/check-video.mjs` compares the loop's frames with the ends).

import {
  LIGHT_CORE,
  LIT_FROM,
  SHADOW,
  WHITE,
  appIcon,
  barShape,
  blank,
  bytes,
  clear,
  colorize,
  cover,
  f,
  held,
  heldDisc,
  lightShape,
  minus,
  mixf,
  over,
  paint,
  plateColour,
  plateShape,
  plated,
  raster,
  shadow,
  trayMask,
  type GlyphColours,
  type Rgb,
  type Rgba,
  type Shape,
} from './heldLight.ts';

export const hex = (rgb: Rgb) => `#${rgb.map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase()}`;

export const rgbOf = (hex: string): Rgb => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) as Rgb;

// sRGB to OKLab and back (Bjorn Ottosson's matrices). Fades go through OKLab, so the colours in between keep their
// lightness and hue on a straight path instead of dipping into grey or brown as an sRGB mix does.
const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const fromLinear = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

function oklab(rgb: Rgb): [number, number, number] {
  const [r, g, b] = rgb.map((v) => toLinear(v / 255));
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function fromOklab(L: number, A: number, B: number): Rgb {
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const linear = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  return linear.map((c) => Math.round(Math.min(1, Math.max(0, fromLinear(c))) * 255)) as Rgb;
}

/** The colour `t` of the way from `a` to `b`, mixed in OKLab; exactly `a` at 0 and `b` at 1. */
export function mixOklab(a: Rgb, b: Rgb, t: number): Rgb {
  if (t <= 0) return a;
  if (t >= 1) return b;
  const p = oklab(a);
  const q = oklab(b);
  return fromOklab(p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, p[2] + (q[2] - p[2]) * t);
}

/** How far the part of a fade that leads has gone at `on`: all the way by 35% of the fade. */
export const leadAt = (on: number) => Math.min(1, Math.max(0, on / 0.35));

/**
 * One fade: two pictures of the same size and, per pixel, how much of it leads (0 to 255): that share changes at
 * `leadAt`, the rest at `on`. The part that leads is the part whose colour must jump furthest for the frames in
 * between to read as one light coming on, not as two things.
 */
export type Fade = {width: number; height: number; from: Uint8Array; to: Uint8Array; lead: Uint8Array};

/**
 * The picture `on` of the way through a fade (0 is `from`, 1 is `to`, byte for byte). Each pixel that differs
 * between the ends is mixed in OKLab, its leading share at `leadAt(on)` and the rest at `on`. Dimming runs the same
 * path back, so the leading part changes last.
 */
export function fadeAt(fade: Fade, on: number): Uint8ClampedArray<ArrayBuffer> {
  const n = fade.width * fade.height;
  const out = new Uint8ClampedArray(n * 4);
  if (on <= 0) out.set(fade.from);
  else if (on >= 1) out.set(fade.to);
  else {
    const lead = leadAt(on);
    for (let i = 0; i < n; i++) {
      const k = i * 4;
      const a: Rgb = [fade.from[k], fade.from[k + 1], fade.from[k + 2]];
      const b: Rgb = [fade.to[k], fade.to[k + 1], fade.to[k + 2]];
      const [fa, fb] = [fade.from[k + 3], fade.to[k + 3]];
      if (a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && fa === fb) {
        out.set([a[0], a[1], a[2], fa], k);
        continue;
      }
      const share = fade.lead[i] / 255;
      const rgb = mixOklab(a, b, share * lead + (1 - share) * on);
      out.set([rgb[0], rgb[1], rgb[2], Math.round(fa + (fb - fa) * on)], k);
    }
  }
  return out;
}

/** An image over an opaque colour, as art.rs composes its review sheets. */
function onPage(img: Rgba, page: Rgb): Uint8Array {
  const out = blank(img.width, img.height, page);
  paint(out, img, 0, 0, 1);
  return out.pixels;
}

/**
 * The tray glyph at `size` px going idle to holding, on the opaque `page`: the ends are `tray_mask` idle and holding
 * in `colours`, composed as the review sheets are. The outline leads: it turns from the glyph colour to the light's
 * while the inside brightens over the whole fade, so the frames in between read as one dim light, not as a white
 * ring round a dark button. Sizes with a hand-drawn ring (16 and 20 px) would lead with the geometric ring's place;
 * the loop draws 128 px, which has none.
 */
export function trayFade(size: number, colours: GlyphColours, page: Rgb): Fade {
  const h = held(size);
  return {
    width: size,
    height: size,
    from: onPage(colorize(trayMask('idle', size), colours), page),
    to: onPage(colorize(trayMask('holding', size), colours), page),
    lead: raster(size, size, minus(heldDisc(h, size, 0), heldDisc(h, size, h.t))),
  };
}

/**
 * Where the tray glyph's `size` px square goes in a `width` x `height` frame: its ink box (the light's top to the
 * line's bottom; the glyph is centred across already) in the middle, on whole pixels.
 */
export function trayPlace(width: number, height: number, size: number): {left: number; top: number} {
  const h = held(size);
  return {left: Math.round((width - size) / 2), top: Math.round(height / 2 - (h.top + h.lineY + h.t) / 2)};
}

/**
 * The app icon with its light off, the start of the video's title: the plate, its shade, shadow and the bar as
 * `app_icon` draws them, no glow and no bloom, and the light as a white outline as heavy as the bar, the way the tray
 * shows an idle light. Not drawn by art.rs: the app never shows its icon unlit.
 */
export function appIconUnlit(size: number): Rgba {
  const p = plated(size, size >= LIT_FROM);
  const plate = raster(size, size, plateShape(p, 0));
  const bar = raster(size, size, barShape(p));
  const ring = raster(size, size, unlitRing(size));
  const shade = p.lit ? shadow(p) : new Uint8Array(size * size);
  const [barX0, , barX1] = p.bar;
  const pixels = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = y * size + x;
      const [fx, fy] = [x + 0.5, y + 0.5];
      const px = clear();
      over(px, [0, 0, 0], Math.fround(SHADOW * cover(shade, i)));
      if (plate[i]) over(px, plateColour(p, fx, fy, false), cover(plate, i));
      if (bar[i]) {
        const along = Math.min(1, Math.max(0, Math.fround(Math.fround(fx - barX0) / Math.fround(barX1 - barX0))));
        over(px, mixf(f(WHITE), f(LIGHT_CORE), along), cover(bar, i));
      }
      if (ring[i]) over(px, f(WHITE), cover(ring, i));
      pixels.set(bytes(px), i * 4);
    }
  }
  return {width: size, height: size, pixels};
}

/** The inside of the unlit light's outline: a disc one bar height smaller than the light. */
function unlitInside(size: number): Shape {
  const p = plated(size, true);
  return {k: 'disc', cx: p.cx, cy: p.cy, r: p.r - (p.bar[3] - p.bar[1])};
}

/**
 * How much each pixel of the app icon leads as its light comes on: all of the outline's inside, then less and less
 * across the outline to none at the light's edge and beyond. A step at the inside's edge left a seam there in the
 * frames in between: the lit middle's warm edge against an outline still nearly white, and a dark hairline where
 * the unlit edge pixels held some plate.
 */
function insideLeads(size: number): Uint8Array {
  const p = plated(size, true);
  const inner = p.r - (p.bar[3] - p.bar[1]);
  const out = new Uint8Array(size * size);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const d = Math.hypot(x + 0.5 - p.cx, y + 0.5 - p.cy);
      out[y * size + x] = Math.round(255 * Math.min(1, Math.max(0, (p.r - d) / (p.r - inner))));
    }
  }
  return out;
}

/** The unlit light's outline, as heavy as the bar. */
function unlitRing(size: number): Shape {
  return minus(lightShape(plated(size, true)), unlitInside(size));
}

/**
 * The app icon at `size` px, its light off to on; the end is `lit`, `app_icon(size)` unless the caller has it already.
 * The inside of the light leads: it fills with light almost at once, as a lamp does when switched on, while the
 * outline warms from white to the light's warm edge and the glow and bloom spread over the whole fade. (With the
 * outline leading, as in the tray glyph, the frames in between showed a warm ring round a grey-green middle.)
 */
export function appIconFade(size: number, lit: Rgba = appIcon(size)): Fade {
  return {
    width: size,
    height: size,
    from: appIconUnlit(size).pixels,
    to: lit.pixels,
    lead: insideLeads(size),
  };
}
