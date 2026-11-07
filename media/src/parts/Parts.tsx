// Each motion part on its own small composition with a test world, so a part is checked by itself before the scenes
// use it (`docs/research/promo-video.md`, "Remotion parts (in `media/src/`)"). Not part of any output.
import React from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {SANS} from '../fonts.ts';
import {World, toScreen, velocityAt, viewAt, type Key, type View} from '../motion/Camera.tsx';
import {Counter} from '../motion/Counter.tsx';
import {Cursor, cursorAt} from '../motion/Cursor.tsx';
import {Focus, focusAt, type Rect} from '../motion/Focus.tsx';
import {Pop} from '../motion/Pop.tsx';
import {Stage} from '../motion/Stage.tsx';
import {Words} from '../motion/Words.tsx';
import {LaptopTilt} from '../promo/LaptopTilt.tsx';
import {SettingsCard} from '../promo/SettingsCard.tsx';
import {DARK} from '../theme.ts';
import type {CaptionBlock, FocusKey} from '../timeline.ts';

const SIZE = {width: 1920, height: 1080};
export const PART_FRAMES = 90;

/** Three numbered boxes in a test world: two near (a slide apart) and one far (a whip below). */
const PLACES: Record<'a' | 'b' | 'c', View> = {a: {x: 0, y: 0, zoom: 1}, b: {x: 1200, y: 0, zoom: 1}, c: {x: 1200, y: 1600, zoom: 1}};
const KEYS: Key<'a' | 'b' | 'c'>[] = [
  {at: 0, to: 'a', kind: 'cut', len: 0},
  {at: 15, to: 'b', kind: 'move', len: 15},
  {at: 45, to: 'c', kind: 'whip', len: 12},
];

const Box: React.FC<{label: string; x: number; y: number}> = ({label, x, y}) => (
  <div
    style={{
      position: 'absolute',
      left: x - 300,
      top: y - 200,
      width: 600,
      height: 400,
      background: DARK.panel,
      border: `2px solid ${DARK.panelLine}`,
      borderRadius: 24,
      color: DARK.text,
      fontFamily: SANS,
      fontSize: 160,
      fontWeight: 600,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
  >
    {label}
  </div>
);

export const PartCamera: React.FC = () => {
  const f = useCurrentFrame();
  const view = viewAt(KEYS, PLACES, f);
  return (
    <AbsoluteFill>
      <Stage focus={{x: 960, y: 540}} width={1920} height={1080} />
      <World view={view} velocity={velocityAt(KEYS, PLACES, f)} size={SIZE}>
        {Object.entries(PLACES).map(([k, p]) => (
          <Box key={k} label={k.toUpperCase()} x={p.x} y={p.y} />
        ))}
      </World>
    </AbsoluteFill>
  );
};

const FOCUS: FocusKey[] = [
  {at: 10, target: 'one'},
  {at: 40, target: 'two'},
  {at: 70, target: null},
];

export const PartFocus: React.FC = () => {
  const f = useCurrentFrame();
  const view = {x: 0, y: 0, zoom: 1};
  const rects: Record<string, Rect> = {one: {x: -600, y: -150, w: 500, h: 300}, two: {x: 150, y: -100, w: 400, h: 200}};
  const resolve = (t: string) => {
    const r = rects[t];
    const p = toScreen(view, SIZE, r.x, r.y);
    return {...r, x: p.x, y: p.y};
  };
  const focus = focusAt(FOCUS, f, resolve);
  return (
    <AbsoluteFill>
      <Stage focus={{x: 960, y: 540}} width={1920} height={1080} />
      <World view={view} velocity={{vx: 0, vy: 0}} size={SIZE}>
        <Box label="1" x={-350} y={0} />
        <Box label="2" x={350} y={0} />
      </World>
      <Focus rect={focus.rect} amount={focus.amount} width={1920} height={1080} />
    </AbsoluteFill>
  );
};

const BLOCK: CaptionBlock = {
  id: 'part',
  lines: [
    {text: 'Close the *lid*.', at: 0, style: 'caption'},
    {text: 'The work goes *on*, locked.', at: 30, style: 'caption'},
  ],
  until: 75,
  leave: 'fade',
};
const BIG: CaptionBlock = {id: 'big', lines: [{text: 'Light and *fast*.', at: 15, style: 'big'}], until: 90, leave: 'stay'};

export const PartWords: React.FC = () => (
  <AbsoluteFill>
    <Stage focus={{x: 960, y: 540}} width={1920} height={1080} />
    <Words block={BIG} baselines={[400]} />
    <Words block={BLOCK} baselines={[856, 928]} />
  </AbsoluteFill>
);

export const PartPieces: React.FC = () => {
  const f = useCurrentFrame();
  const tip = cursorAt(
    [
      {at: 0, x: 1500, y: 900, len: 0},
      {at: 20, x: 600, y: 500, len: 15},
      {at: 50, x: 1100, y: 420, len: 12},
    ],
    f,
  );
  return (
    <AbsoluteFill>
      <Stage focus={{x: 960, y: 540}} width={1920} height={1080} />
      <div style={{position: 'absolute', left: 200, top: 200}}>
        <Pop at={20}>
          <div style={{width: 300, height: 200, background: DARK.panel, borderRadius: 20, border: `2px solid ${DARK.panelLine}`}} />
        </Pop>
      </div>
      <div style={{position: 'absolute', left: 1000, top: 600, fontFamily: SANS, fontSize: 150, fontWeight: 600, color: DARK.accent}}>
        <Counter at={30} len={12} value={12.4} digits={1} />
      </div>
      <Cursor tip={tip} f={f} clicks={[22, 52]} />
    </AbsoluteFill>
  );
};

export const PartLaptop: React.FC = () => {
  const f = useCurrentFrame();
  const shut = Math.min(1, Math.max(0, (f - 15) / 15)) ** 3;
  return (
    <AbsoluteFill>
      <Stage focus={{x: 960, y: 540}} width={1920} height={1080} />
      <div style={{position: 'absolute', left: 580, top: 240}}>
        <LaptopTilt f={f} shut={shut} yaw={-30 + (6 * f) / 90} edge={f > 30 ? 1 : 0} led={1} streamFrom={0} />
      </div>
      <div style={{position: 'absolute', left: 1000, top: 700}}>
        <SettingsCard f={f} rolls={[45, 60]} />
      </div>
    </AbsoluteFill>
  );
};
