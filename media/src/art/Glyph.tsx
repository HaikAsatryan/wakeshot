import React from 'react';
import {appLayout, grid, idle, risen, type Grid, type Rgb} from './sunrise.ts';

export const css = (rgb: Rgb) => `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`;

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/**
 * Where the Sunrise is between its two tray states. 0 everywhere is the idle glyph and 1 everywhere the holding glyph,
 * both exactly as art.rs draws them at this size; values in between are the motion the brand describes (the sun
 * rises when work starts and sets when it ends), which the tray itself does not show.
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
  /** A soft light behind the risen sun, 0 to 1. Not in the tray: the brand's holding highlight for the page and video. */
  glow?: number;
  glowColour?: string;
  id: string;
};

/** The glyph's shapes on the given grid, for an `<svg>` whose units are pixels of the art. */
export const SunriseParts: React.FC<PartsProps> = ({g, phase, ink, sun = ink, glow = 0, glowColour = '#FFCC4D', id}) => {
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
      {glow > 0 ? (
        <>
          <defs>
            <radialGradient id={`${id}-glow`} cx={cx} cy={base - r * 0.3} r={r * 3} gradientUnits="userSpaceOnUse">
              <stop offset="0" stopColor={glowColour} stopOpacity={0.5 * glow} />
              <stop offset="0.45" stopColor={glowColour} stopOpacity={0.18 * glow} />
              <stop offset="1" stopColor={glowColour} stopOpacity={0} />
            </radialGradient>
          </defs>
          <circle cx={cx} cy={base - r * 0.3} r={r * 3} fill={`url(#${id}-glow)`} />
        </>
      ) : null}
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

/** The tray glyph at `size` pixels of art, drawn `scale` times larger. */
export const Glyph: React.FC<{
  size: number;
  scale?: number;
  phase: Phase;
  ink: string;
  sun?: string;
  glow?: number;
  id: string;
}> = ({size, scale = 1, phase, ink, sun, glow, id}) => (
  <svg width={size * scale} height={size * scale} viewBox={`0 0 ${size} ${size}`} style={{overflow: 'visible'}}>
    <SunriseParts g={grid(0, 0, size)} phase={phase} ink={ink} sun={sun} glow={glow} id={id} />
  </svg>
);

/**
 * The app icon at `size` pixels: the violet plate with the Sunrise. At `HOLDING` it is the app icon art.rs draws; at
 * `IDLE` the sun is an empty white half ring, a state that exists only in the video's opening shot.
 */
export const AppIcon: React.FC<{size: number; scale?: number; phase: Phase; id: string}> = ({
  size,
  scale = 1,
  phase,
  id,
}) => {
  const L = appLayout(size);
  const u = {x: Math.sqrt(3) / 2, y: 0.5};
  const sun = mixHex('#FFFFFF', '#FFCC4D', phase.fill);
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
          <stop offset="0" stopColor="#7957F4" />
          <stop offset="1" stopColor="#6B49F3" />
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
      <SunriseParts g={L.grid} phase={phase} ink="#FFFFFF" sun={sun} id={id} />
    </svg>
  );
};

function mixHex(a: string, b: string, t: number): string {
  const p = (s: string, i: number) => parseInt(s.slice(1 + i * 2, 3 + i * 2), 16);
  const c = [0, 1, 2].map((i) => Math.round(p(a, i) + (p(b, i) - p(a, i)) * t));
  return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
}
