import React from 'react';
import {DARK, MOSS} from '../theme.ts';
import {GLIDE, progress} from './ease.ts';

export type Point = {x: number; y: number};
/** The cursor arrives at (x, y) at frame `at`, after a glide of `len` frames from where it was. */
export type CursorMove = {at: number; x: number; y: number; len: number};

const ARROW = 'M1.5 1.5 L1.5 20.5 L6.2 16.2 L9.4 23.6 L12.6 22.2 L9.5 15 L15.8 15 Z';

/** How far the glide bows off the straight line, as a share of its length: a hand never moves dead straight. */
const BOW = 0.12;

/** Where the cursor's tip is at frame `f`. */
export function cursorAt(moves: CursorMove[], f: number): Point {
  let p: Point = {x: moves[0].x, y: moves[0].y};
  for (const m of moves) {
    const start = m.at - m.len;
    if (f < start) break;
    const t = progress(f, start, m.len, GLIDE);
    // A quadratic curve through a control point set off the line's middle, to the right of the direction of travel.
    const dx = m.x - p.x;
    const dy = m.y - p.y;
    const c = {x: p.x + dx / 2 - dy * BOW, y: p.y + dy / 2 + dx * BOW};
    const u = 1 - t;
    p = {x: u * u * p.x + 2 * u * t * c.x + t * t * m.x, y: u * u * p.y + 2 * u * t * c.y + t * t * m.y};
  }
  return p;
}

/**
 * The drawn arrow, its tip at `tip` (screen pixels). On a click (frames in `clicks`) it dips to 90% and back over 6
 * frames and a ring spreads from the tip over 8.
 */
export const Cursor: React.FC<{tip: Point; f: number; clicks: number[]; opacity?: number; size?: number}> = ({
  tip,
  f,
  clicks,
  opacity = 1,
  size = 1,
}) => {
  const last = clicks.filter((c) => f >= c).pop();
  const since = last === undefined ? Infinity : f - last;
  const dip = since < 6 ? 1 - 0.1 * Math.sin((since / 6) * Math.PI) : 1;
  const ring = since < 8 ? since / 8 : null;
  return (
    <div style={{position: 'absolute', left: tip.x, top: tip.y, opacity}}>
      {ring !== null ? (
        <div
          style={{
            position: 'absolute',
            left: -28 * ring * size,
            top: -28 * ring * size,
            width: 56 * ring * size,
            height: 56 * ring * size,
            borderRadius: '50%',
            border: `${2 * size}px solid ${MOSS.lightEdge}`,
            opacity: 1 - ring,
          }}
        />
      ) : null}
      <svg
        width={36 * size}
        height={52 * size}
        viewBox="0 0 18 26"
        style={{position: 'absolute', left: -1.5 * size, top: -1.5 * size, transform: `scale(${dip})`, transformOrigin: '1.5px 1.5px', overflow: 'visible'}}
      >
        {/* A plain offset copy for the shadow, not a CSS drop-shadow filter: filtered layers rendered a level apart
            from one run to the next, and the clean rebuild must give the same bytes. */}
        <path d={ARROW} transform="translate(0.6 1.2)" fill="#000" opacity={0.35} />
        <path d={ARROW} fill={DARK.text} stroke={DARK.terminal} strokeWidth={1.3} strokeLinejoin="round" />
      </svg>
    </div>
  );
};
