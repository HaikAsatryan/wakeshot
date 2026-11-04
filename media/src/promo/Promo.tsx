import React from 'react';
import {AbsoluteFill, Sequence, staticFile} from 'remotion';
import {Audio} from '@remotion/media';
import {C} from '../theme.ts';
import {SCENES} from '../timeline.ts';
import {DraftBanner, DraftContext} from './parts.tsx';
import {KeepAwake, LidClosed, Outro, SettingsBack, Speed, Title, WorkEnds, WorkStarts} from './scenes.tsx';

export type PromoProps = {draft: boolean};

const ORDER: [keyof typeof SCENES, React.FC][] = [
  ['title', Title],
  ['start', WorkStarts],
  ['lid', LidClosed],
  ['end', WorkEnds],
  ['settings', SettingsBack],
  ['keepAwake', KeepAwake],
  ['speed', Speed],
  ['outro', Outro],
];

/** The 30 second video. The soundtrack is one file that `scripts/make-sounds.mjs` mixes on the same timeline. */
export const Promo: React.FC<PromoProps> = ({draft}) => (
  <DraftContext.Provider value={draft}>
    <AbsoluteFill style={{background: `radial-gradient(ellipse at 50% 40%, #221C3D 0%, ${C.night} 70%)`}}>
      {ORDER.map(([key, Scene]) => (
        <Sequence key={key} from={SCENES[key].from} durationInFrames={SCENES[key].len} name={key}>
          <Scene />
        </Sequence>
      ))}
      <Audio src={staticFile('sound/soundtrack.wav')} />
      {draft ? <DraftBanner /> : null}
    </AbsoluteFill>
  </DraftContext.Provider>
);
