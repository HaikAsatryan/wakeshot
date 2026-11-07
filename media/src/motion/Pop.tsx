import React from 'react';
import {useCurrentFrame} from 'remotion';
import {POP, progress} from './ease.ts';

/** A pop lasts 10 frames; it is fully opaque after 4. */
export const POP_LEN = 10;
const FADE = 4;

/**
 * Pops its children in at `at`. Drawn things scale from 0.94 to 1 about `origin` (the thing they come from, such as
 * the tray icon) with a spring of about 4% overshoot. A capture (`rise`) is never scaled, so its pixels stay whole: it
 * rises 16 px instead, on whole pixels. `out` takes it away again over 6 frames.
 */
export const Pop: React.FC<{
  at: number;
  out?: number;
  origin?: string;
  rise?: boolean;
  style?: React.CSSProperties;
  children: React.ReactNode;
}> = ({at, out, origin = '50% 50%', rise, style, children}) => {
  const f = useCurrentFrame();
  if (f < at) return null;
  const leave = out === undefined ? 0 : progress(f, out, 6, (t) => t);
  if (leave >= 1) return null;
  const t = progress(f, at, POP_LEN, POP);
  const opacity = progress(f, at, FADE, (x) => x) * (1 - leave);
  const transform = rise ? `translateY(${Math.round(16 * (1 - Math.min(1, t)))}px)` : `scale(${0.94 + 0.06 * t})`;
  return <div style={{opacity, transform, transformOrigin: origin, ...style}}>{children}</div>;
};
