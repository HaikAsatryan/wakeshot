import React from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {Glyph} from '../art/Glyph.tsx';
import {css} from '../art/Glyph.tsx';
import {GLYPH_ON_DARK, GLYPH_ON_LIGHT} from '../art/sunrise.ts';
import {rise, set} from '../promo/scenes.tsx';
import {C} from '../theme.ts';

export type LoopProps = {theme: 'light' | 'dark'};

/** Art size of the glyph in the loop, in pixels: the art is drawn at this size, not scaled from a smaller one. */
const ART = 128;

/**
 * The README's silent loop: the tray glyph, in the colour the taskbar draws it, goes from idle to holding and back.
 * Frame 0 and the last frame are both the idle glyph, so the loop has no jump.
 */
export const IconLoop: React.FC<LoopProps> = ({theme}) => {
  const f = useCurrentFrame();
  const phase = f < 62 ? rise(f, 20, 26) : set(f, 82, 26);
  const ink = css(theme === 'dark' ? GLYPH_ON_DARK : GLYPH_ON_LIGHT);
  return (
    <AbsoluteFill style={{background: theme === 'dark' ? C.night : C.mist, alignItems: 'center', justifyContent: 'center'}}>
      <Glyph size={ART} phase={phase} ink={ink} glow={phase.fill * (theme === 'dark' ? 0.9 : 0.7)} id={`loop-${theme}`} />
    </AbsoluteFill>
  );
};
