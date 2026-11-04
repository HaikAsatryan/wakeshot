import React from 'react';
import {Composition} from 'remotion';
import './fonts.ts';
import {IconLoop} from './loop/IconLoop.tsx';
import {Promo} from './promo/Promo.tsx';
import {LOOP, VIDEO} from './timeline.ts';

export const Root: React.FC = () => (
  <>
    <Composition
      id="Promo"
      component={Promo}
      durationInFrames={VIDEO.frames}
      fps={VIDEO.fps}
      width={VIDEO.width}
      height={VIDEO.height}
      defaultProps={{draft: true}}
    />
    <Composition
      id="IconLoopLight"
      component={IconLoop}
      durationInFrames={LOOP.frames}
      fps={LOOP.fps}
      width={LOOP.width}
      height={LOOP.height}
      defaultProps={{theme: 'light'}}
    />
    <Composition
      id="IconLoopDark"
      component={IconLoop}
      durationInFrames={LOOP.frames}
      fps={LOOP.fps}
      width={LOOP.width}
      height={LOOP.height}
      defaultProps={{theme: 'dark'}}
    />
  </>
);
