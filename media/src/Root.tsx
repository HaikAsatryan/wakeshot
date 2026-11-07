import React from 'react';
import {Composition, Folder} from 'remotion';
import './fonts.ts';
import {IconLoop} from './loop/IconLoop.tsx';
import {PART_FRAMES, PartCamera, PartFocus, PartLaptop, PartPieces, PartWords} from './parts/Parts.tsx';
import {Promo} from './promo/Promo.tsx';
import {LOOP, VIDEO} from './timeline.ts';

const part = {durationInFrames: PART_FRAMES, fps: VIDEO.fps, width: VIDEO.width, height: VIDEO.height};

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
    <Folder name="Parts">
      <Composition id="PartCamera" component={PartCamera} {...part} />
      <Composition id="PartFocus" component={PartFocus} {...part} />
      <Composition id="PartWords" component={PartWords} {...part} />
      <Composition id="PartPieces" component={PartPieces} {...part} />
      <Composition id="PartLaptop" component={PartLaptop} {...part} />
    </Folder>
  </>
);
