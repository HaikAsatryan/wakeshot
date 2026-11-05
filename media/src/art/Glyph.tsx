import React from 'react';
import {MOSS} from '../theme.ts';
import {appLayout, idle, risen, type Grid, type Rgb} from './sunrise.ts';

export const hex = (rgb: Rgb) => `#${rgb.map((v) => v.toString(16).padStart(2, '0')).join('')}`;

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

// sRGB to OKLab and back (Bjorn Ottosson's matrices). Fades between two colours go through OKLab, so the colours in
// between keep their lightness and hue on a straight path instead of dipping into grey or brown as an sRGB mix does.
const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const fromLinear = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

function oklab(hex: string): [number, number, number] {
  const [r, g, b] = [1, 3, 5].map((i) => toLinear(parseInt(hex.slice(i, i + 2), 16) / 255));
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

/** The colour `t` of the way from `a` to `b` (both `#RRGGBB`), mixed in OKLab, as `#RRGGBB`. */
export function mixColour(a: string, b: string, t: number): string {
  if (t <= 0) return a;
  if (t >= 1) return b;
  const p = oklab(a);
  const q = oklab(b);
  const [L, A, B] = [0, 1, 2].map((i) => p[i] + (q[i] - p[i]) * t);
  const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const rgb = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  return `#${rgb.map((c) => Math.round(Math.min(1, Math.max(0, fromLinear(c))) * 255).toString(16).padStart(2, '0')).join('')}`;
}

/**
 * The held light's numbers at `size` pixels. Provisional: the 32 px drawing of the round 2 design (`marks2.py`,
 * `HELD[32]`) scaled to any size, until `p6-brand-apply-art` lands and this becomes the checked port of `art.rs`.
 */
export function heldLayout(size: number) {
  const u = size / 32;
  const t = 2 * u;
  const r = 8 * u;
  const top = 4 * u;
  return {cx: size / 2, cy: top + r, r, t, line: {x0: 4 * u, x1: 28 * u, y0: top + 2 * r + 4 * u, h: t}};
}

/**
 * The tray glyph, the held light, at `size` pixels of art. `on` 0 is idle (the light as an outline), 1 is holding
 * (the light filled in its holding colour, as the tray draws it); in between the light fades on in place. `page` is
 * the colour behind the glyph: the unlit inside starts as it.
 */
export const HeldLight: React.FC<{size: number; scale?: number; on: number; ink: string; light: string; page: string}> = ({
  size,
  scale = 1,
  on,
  ink,
  light,
  page,
}) => {
  const L = heldLayout(size);
  // The outline takes the light's colour in the first third of the fade and the inside brightens over all of it, so
  // the in-between frames read as one dim light, not as a pale rim round a dark button. Dimming runs the same path
  // backwards: the inside darkens first, the outline turns back to the glyph colour last.
  const rim = Math.min(1, on / 0.35);
  const outer = L.r;
  const inner = L.r - L.t;
  const circle = (r: number) =>
    `M ${L.cx - r} ${L.cy} A ${r} ${r} 0 1 0 ${L.cx + r} ${L.cy} A ${r} ${r} 0 1 0 ${L.cx - r} ${L.cy} Z`;
  const ring = `${circle(outer)} ${circle(inner)}`;
  return (
    <svg width={size * scale} height={size * scale} viewBox={`0 0 ${size} ${size}`}>
      {/*
        The inside reaches under the ring's middle, so its soft edge is covered by the ring and never meets the
        background: lit, inside and ring are one colour with no seam between them.
      */}
      {on > 0 ? <circle cx={L.cx} cy={L.cy} r={L.r - L.t / 2} fill={mixColour(page, light, on)} /> : null}
      <path d={ring} fill={mixColour(ink, light, rim)} fillRule="evenodd" />
      <rect x={L.line.x0} y={L.line.y0} width={L.line.x1 - L.line.x0} height={L.line.h} rx={L.line.h / 2} fill={ink} />
    </svg>
  );
};

/**
 * Where the Sunrise is between its two states, for the app icon in the video's title until the art port of
 * `p6-brand-apply-art` replaces it. 0 everywhere is idle and 1 everywhere holding.
 */
export type Phase = {
  /** Horizon and sun move from the idle place and size to the holding ones. */
  shift: number;
  /** The empty sun fills. */
  fill: number;
  /** The three rays grow out. */
  rays: number;
};

export const IDLE: Phase = {shift: 0, fill: 0, rays: 0};
export const HOLDING: Phase = {shift: 1, fill: 1, rays: 1};

type PartsProps = {
  g: Grid;
  phase: Phase;
  ink: string;
  /** The sun and rays; the horizon keeps `ink`. */
  sun?: string;
};

/** The glyph's shapes on the given grid, for an `<svg>` whose units are pixels of the art. */
const SunriseParts: React.FC<PartsProps> = ({g, phase, ink, sun = ink}) => {
  const t = g.t;
  const i = idle(g);
  const h = risen(g);
  const s = phase.shift;
  const cx = i.cx;
  const r = lerp(i.r, h.r, s);
  const base = lerp(i.base, h.base, s);
  const bar = {x0: i.bar.x0, x1: i.bar.x1, y0: lerp(i.bar.y0, h.bar.y0, s), r: i.bar.r};
  // The hole closes as the sun fills; at 1 the half ring is the holding half disc.
  const ri = (r - t) * (1 - phase.fill);
  // A hole thinner than half a stroke is closed outright, so no speck is left in the nearly filled sun.
  const sunPath =
    ri >= t / 2
      ? `M ${cx - r} ${base} A ${r} ${r} 0 0 1 ${cx + r} ${base} L ${cx + ri} ${base} A ${ri} ${ri} 0 0 0 ${cx - ri} ${base} Z`
      : `M ${cx - r} ${base} A ${r} ${r} 0 0 1 ${cx + r} ${base} Z`;
  const d = Math.SQRT1_2;
  const grow = phase.rays;
  const rays: [number, number, number, number][] = [
    [cx, h.inner - t / 2, cx, h.inner - h.len + t / 2],
    [cx - d * h.a, h.base - d * h.a, cx - d * h.b, h.base - d * h.b],
    [cx + d * h.a, h.base - d * h.a, cx + d * h.b, h.base - d * h.b],
  ];
  return (
    <g>
      <path d={sunPath} fill={sun} fillRule="evenodd" />
      {grow > 0
        ? rays.map(([ax, ay, bx, by], k) => {
            // Each ray starts a little after the one before, the upright one first.
            const own = Math.max(0, Math.min(1, grow * 1.6 - k * 0.3));
            if (own <= 0) return null;
            return (
              <line
                key={k}
                x1={ax}
                y1={ay}
                x2={lerp(ax, bx, own)}
                y2={lerp(ay, by, own)}
                stroke={sun}
                strokeWidth={t}
                strokeLinecap="round"
                opacity={Math.min(1, own * 4)}
              />
            );
          })
        : null}
      <rect x={bar.x0} y={bar.y0} width={bar.x1 - bar.x0} height={t} rx={bar.r} fill={ink} />
    </g>
  );
};

/**
 * The app icon at `size` pixels: the Moss plate with the old mark until the art port of `p6-brand-apply-art` brings
 * the held light. At `HOLDING` the light is the brand's light edge colour; at `IDLE` it is white.
 */
export const AppIcon: React.FC<{size: number; scale?: number; phase: Phase; id: string}> = ({
  size,
  scale = 1,
  phase,
  id,
}) => {
  const L = appLayout(size);
  const u = {x: Math.sqrt(3) / 2, y: 0.5};
  return (
    <svg width={size * scale} height={size * scale} viewBox={`0 0 ${size} ${size}`}>
      <defs>
        <linearGradient
          id={`${id}-plate`}
          gradientUnits="userSpaceOnUse"
          x1={L.centre - L.half * u.x}
          y1={L.centre - L.half * u.y}
          x2={L.centre + L.half * u.x}
          y2={L.centre + L.half * u.y}
        >
          <stop offset="0" stopColor={MOSS.plateLight} />
          <stop offset="1" stopColor={MOSS.plateDark} />
        </linearGradient>
      </defs>
      <rect
        x={L.margin}
        y={L.margin}
        width={size - 2 * L.margin}
        height={size - 2 * L.margin}
        rx={L.corner}
        fill={`url(#${id}-plate)`}
      />
      <SunriseParts g={L.grid} phase={phase} ink="#FFFFFF" sun={mixColour('#FFFFFF', MOSS.lightEdge, phase.fill)} />
    </svg>
  );
};
