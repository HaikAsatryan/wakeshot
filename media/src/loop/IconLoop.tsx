import React from 'react';
import {AbsoluteFill, Easing, interpolate, useCurrentFrame} from 'remotion';
import {HeldLight, hex} from '../art/Glyph.tsx';
import {GLYPH_ON_DARK, GLYPH_ON_LIGHT} from '../art/sunrise.ts';
import {GITHUB, MOSS} from '../theme.ts';
import {LOOP} from '../timeline.ts';

export type LoopProps = {theme: 'light' | 'dark'};

/** Art size of the glyph in the loop, in pixels: the art is drawn at this size, not scaled from a smaller one. */
const ART = 128;

/**
 * The light comes on quickly and settles, as a lamp does, and dims more slowly; the rest of the 5 s is the two
 * states held long enough to read: idle 1.6 s in all (both ends), holding about 2.2 s.
 */
export const LOOP_BEATS = {
  on: {from: 25, len: 11, easing: Easing.out(Easing.cubic)},
  off: {from: 90, len: 20, easing: Easing.inOut(Easing.cubic)},
};

/** How lit the light is at frame `f`: 0 idle, 1 holding. */
export function lit(f: number): number {
  const {on, off} = LOOP_BEATS;
  const opts = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;
  if (f < off.from) return interpolate(f, [on.from, on.from + on.len], [0, 1], {...opts, easing: on.easing});
  return interpolate(f, [off.from, off.from + off.len], [1, 0], {...opts, easing: off.easing});
}

/**
 * The README's silent loop: the tray glyph goes idle, holding, idle. The light fades on and off in place, in the
 * colours the tray draws (only the light takes the brand colour), on GitHub's own page colour so the GIF has no box.
 * The first and last frames are both idle, so the loop has no jump.
 */
export const IconLoop: React.FC<LoopProps> = ({theme}) => {
  const f = useCurrentFrame();
  if (LOOP_BEATS.off.from + LOOP_BEATS.off.len >= LOOP.frames) throw new Error('the loop must end idle');
  const page = GITHUB[theme];
  const ink = hex(theme === 'dark' ? GLYPH_ON_DARK : GLYPH_ON_LIGHT);
  return (
    <AbsoluteFill style={{background: page, alignItems: 'center', justifyContent: 'center'}}>
      <HeldLight
        size={ART}
        on={lit(f)}
        ink={ink}
        light={theme === 'dark' ? MOSS.holdingOnDark : MOSS.holdingOnLight}
        page={page}
      />
    </AbsoluteFill>
  );
};
