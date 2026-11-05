import React from 'react';
import {MOSS} from '../theme.ts';
import type {FocusKey} from '../timeline.ts';
import {SETTLE, progress} from './ease.ts';

export type Rect = {x: number; y: number; w: number; h: number};

/** The focus comes in over 8 frames, glides to a new target over 12, and goes over 6 (`promo-video.md`, "Focus"). */
const IN = 8;
const GLIDE = 12;
const OUT = 6;
/** The ring sits this far outside its target. */
const GAP = 6;

const lerp = (a: Rect, b: Rect, t: number): Rect => ({
  x: a.x + (b.x - a.x) * t,
  y: a.y + (b.y - a.y) * t,
  w: a.w + (b.w - a.w) * t,
  h: a.h + (b.h - a.h) * t,
});

/**
 * Where the focus is at frame `f` and how strong (0 none, 1 full). `resolve` gives a target's rect on screen at this
 * frame, so a ring stays on its target while the camera moves.
 */
export function focusAt(
  keys: FocusKey[],
  f: number,
  resolve: (target: string) => Rect | null,
): {rect: Rect | null; amount: number; dim: boolean} {
  let rect: Rect | null = null;
  let amount = 0;
  let dim = true;
  let last: string | null = null;
  for (const k of keys) {
    if (f < k.at) break;
    if (k.target === null) {
      amount = k.cut ? 0 : amount * (1 - progress(f, k.at, OUT, (t) => t));
      if (last) rect = resolve(last);
      if (amount === 0) last = null;
      continue;
    }
    const to = resolve(k.target);
    if (!to) continue;
    if (last && rect && amount > 0) {
      const from = resolve(last) ?? rect;
      rect = lerp(from, to, progress(f, k.at, GLIDE, SETTLE));
      amount = 1;
    } else {
      rect = to;
      amount = progress(f, k.at, IN, SETTLE);
    }
    last = k.target;
    dim = k.dim ?? true;
  }
  return {rect, amount, dim};
}

/** A rounded rectangle as an SVG path, clockwise. */
const rounded = (r: Rect, radius: number) => {
  const k = Math.min(radius, r.w / 2, r.h / 2);
  return (
    `M${r.x + k} ${r.y}H${r.x + r.w - k}A${k} ${k} 0 0 1 ${r.x + r.w} ${r.y + k}V${r.y + r.h - k}` +
    `A${k} ${k} 0 0 1 ${r.x + r.w - k} ${r.y + r.h}H${r.x + k}A${k} ${k} 0 0 1 ${r.x} ${r.y + r.h - k}V${r.y + k}` +
    `A${k} ${k} 0 0 1 ${r.x + k} ${r.y}Z`
  );
};

/**
 * Everything but the target drops to 35% brightness and a 4 px blur (unless `dim` is false), and a 2 px ring in the
 * light's edge colour with a soft glow wraps the target 6 px outside it. The real picture inside the ring stays
 * untouched.
 */
export const Focus: React.FC<{rect: Rect | null; amount: number; dim?: boolean; radius?: number; width: number; height: number}> = ({
  rect,
  amount,
  dim = true,
  radius = 10,
  width,
  height,
}) => {
  if (!rect || amount <= 0.001) return null;
  const hole = `M0 0H${width}V${height}H0Z${rounded(rect, radius)}`;
  const ring = {x: rect.x - GAP, y: rect.y - GAP, w: rect.w + 2 * GAP, h: rect.h + 2 * GAP};
  return (
    <>
      {dim ? (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            clipPath: `path(evenodd, '${hole}')`,
            backdropFilter: `blur(${(4 * amount).toFixed(2)}px) brightness(${(1 - 0.65 * amount).toFixed(3)})`,
          }}
        />
      ) : null}
      <div
        style={{
          position: 'absolute',
          left: ring.x,
          top: ring.y,
          width: ring.w,
          height: ring.h,
          boxSizing: 'border-box',
          border: `2px solid ${MOSS.lightEdge}`,
          borderRadius: radius + GAP,
          boxShadow: `0 0 24px ${MOSS.lightEdge}66, inset 0 0 12px ${MOSS.lightEdge}22`,
          opacity: amount,
        }}
      />
    </>
  );
};
