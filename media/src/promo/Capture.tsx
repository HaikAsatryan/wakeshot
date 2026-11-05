import React from 'react';
import {Img, staticFile} from 'remotion';
import shotsData from '../../data/shots.json';
import {TrayGlyph} from '../art/Glyph.tsx';
import {ON_DARK, TASKBAR_DARK} from '../art/heldLight.ts';
import {SANS} from '../fonts.ts';
import {DARK} from '../theme.ts';
import {DraftContext} from './draft.ts';

export type Box = {x: number; y: number; w: number; h: number};
type ShotInfo = {what: string; file: string | null; standIn: boolean; from: string};
type SetInfo = {
  what: string;
  theme: 'dark';
  scale: number;
  w: number;
  h: number;
  taskbar: number;
  boxes: Record<string, Box>;
  rows: (Box & {name: string})[];
  shots: Record<string, ShotInfo>;
};
export type SetName = 'tray' | 'menu';
export const SETS = shotsData.sets as Record<SetName, SetInfo>;

/** A named place inside a set, in capture pixels: a box or a menu row. */
export function boxOf(set: SetName, name: string): Box {
  const s = SETS[set];
  const b = s.boxes[name] ?? s.rows.find((r) => r.name === name);
  if (!b) throw new Error(`no box or row ${name} in set ${set} of data/shots.json`);
  return {x: b.x, y: b.y, w: b.w, h: b.h};
}

/** Whether a shot shows the icon holding: the sketches need it; a real capture shows it by itself. */
const HOLDING = new Set(['S2', 'S3', 'S5d']);
/** The tray set's shots that show a tooltip, and its box. */
const TIPS: Record<string, string> = {S3: 'tip', S4: 'tipIdle'};

/**
 * One shot of a set at a whole-number `zoom`, its top left at the parent's origin. A real capture is drawn with hard
 * pixels, so it is shown as it was taken. Until it exists, the draft draws a plain sketch of the set (gray bars where
 * the words go: the video never redraws a UI word) and tags it; the final render throws instead. `clip` shows only
 * one box of the shot, for the parts that pop over the shot before it.
 */
export const Capture: React.FC<{set: SetName; shot: string; zoom: number; clip?: string; style?: React.CSSProperties}> = ({
  set,
  shot,
  zoom,
  clip,
  style,
}) => {
  const draft = React.useContext(DraftContext);
  const s = SETS[set];
  const info = s.shots[shot];
  if (!info) throw new Error(`no shot ${shot} in set ${set} of data/shots.json`);
  if (!Number.isInteger(zoom)) throw new Error(`a capture is shown at whole-number zoom only; ${set} ${shot} asked for ${zoom}`);
  if (!draft && (!info.file || info.standIn)) throw new Error(`shot ${shot} has no real capture; the final video needs it`);
  const c = clip ? boxOf(set, clip) : null;
  const clipPath = c ? `inset(${c.y * zoom}px ${(s.w - c.x - c.w) * zoom}px ${(s.h - c.y - c.h) * zoom}px ${c.x * zoom}px)` : undefined;
  return (
    <div style={{position: 'absolute', left: 0, top: 0, width: s.w * zoom, height: s.h * zoom, clipPath, ...style}}>
      {info.file ? (
        <Img src={staticFile(info.file)} style={{width: s.w * zoom, height: s.h * zoom, imageRendering: 'pixelated', display: 'block'}} />
      ) : (
        <Sketch set={set} shot={shot} zoom={zoom} />
      )}
    </div>
  );
};

/** The stand-in tag for a set, outside the zoomed picture so it reads the same at any zoom. */
export const StandInTag: React.FC<{set: SetName; shots: string[]; style?: React.CSSProperties}> = ({set, shots, style}) => {
  const draft = React.useContext(DraftContext);
  const s = SETS[set];
  if (!draft || shots.every((id) => s.shots[id].file && !s.shots[id].standIn)) return null;
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        top: -40,
        background: DARK.draft,
        color: DARK.onDraft,
        fontFamily: SANS,
        fontWeight: 600,
        fontSize: 20,
        padding: '3px 10px',
        borderRadius: 4,
        whiteSpace: 'nowrap',
        ...style,
      }}
    >
      {shots.join(', ')} stand-in sketch: no capture yet
    </div>
  );
};

// The sketch's colours: a dark Windows taskbar and menu, in plain greys. Not a copy of any real picture.
const SK = {bar: '#202021', barLine: '#2E2E30', menu: '#2B2B2D', menuLine: '#3F3F42', hover: '#3A3A3D', ink: '#8A8A8E', faint: '#5A5A5E'};

const Rect: React.FC<{b: Box; z: number; style: React.CSSProperties}> = ({b, z, style}) => (
  <div style={{position: 'absolute', left: b.x * z, top: b.y * z, width: b.w * z, height: b.h * z, ...style}} />
);

/** A gray bar where a word or line of a real capture would be. */
const Bar: React.FC<{x: number; y: number; w: number; z: number; h?: number; colour?: string}> = ({x, y, w, z, h = 9, colour = SK.ink}) => (
  <Rect b={{x, y, w, h}} z={z} style={{background: colour, borderRadius: (h / 2) * z}} />
);

const Sketch: React.FC<{set: SetName; shot: string; zoom: number}> = ({set, shot, zoom: z}) => {
  const s = SETS[set];
  const glyph = boxOf(set, 'glyph');
  const holding = HOLDING.has(shot);
  const top = s.taskbar;
  // Other tray icons and the clock as plain shapes, sized from the set's own glyph so the sketch fits its scale.
  const g = glyph.w;
  const u = s.scale / 100;
  const iconsY = glyph.y + 2;
  const parts: React.ReactNode[] = [
    <Rect key="bar" b={{x: 0, y: top, w: s.w, h: s.h - top}} z={z} style={{background: SK.bar, borderTop: `${z}px solid ${SK.barLine}`}} />,
    <Rect key="i1" b={{x: glyph.x - 2 * g, y: iconsY, w: g - 4, h: g - 4}} z={z} style={{background: SK.faint, borderRadius: 4 * z}} />,
    <Rect key="i2" b={{x: glyph.x - 3.5 * g, y: iconsY, w: g - 4, h: g - 4}} z={z} style={{background: SK.faint, borderRadius: 4 * z}} />,
    <Bar key="c1" x={s.w - 64 * u} y={top + 14 * u} w={44 * u} h={6 * u} z={z} />,
    <Bar key="c2" x={s.w - 70 * u} y={top + 28 * u} w={56 * u} h={6 * u} z={z} />,
  ];
  const menuOpen = ['S5a', 'S5b', 'S5c'].includes(shot);
  if (set === 'menu' && menuOpen) {
    const m = boxOf('menu', 'menu');
    parts.push(<Rect key="menu" b={m} z={z} style={{background: SK.menu, border: `${z}px solid ${SK.menuLine}`, borderRadius: 8 * z, boxSizing: 'border-box'}} />);
    const hover = shot === 'S5b' || shot === 'S5c' ? 'keepAwake' : null;
    for (const r of s.rows.filter((row) => row.x >= m.x)) {
      if (r.name === hover) parts.push(<Rect key={`h-${r.name}`} b={r} z={z} style={{background: SK.hover, borderRadius: 4 * z}} />);
      const lines = r.name === 'status' ? 2 : 1;
      for (let i = 0; i < lines; i++) {
        const w = r.name === 'status' ? (i === 0 ? 230 : 160) : 90 + ((r.name.length * 37) % 80);
        parts.push(<Bar key={`b-${r.name}-${i}`} x={r.x + 44} y={r.y + (r.h - 9 * lines - 10 * (lines - 1)) / 2 + i * 19} w={w} z={z} />);
      }
      parts.push(<Rect key={`d-${r.name}`} b={{x: r.x + 14, y: r.y + r.h / 2 - 8, w: 16, h: 16}} z={z} style={{border: `${1.5 * z}px solid ${SK.faint}`, borderRadius: 4 * z, boxSizing: 'border-box'}} />);
    }
    // Lines between the groups of rows.
    for (const y of [240, 340, 440]) parts.push(<Rect key={`sep-${y}`} b={{x: m.x + 12, y, w: m.w - 24, h: 1}} z={z} style={{background: SK.menuLine}} />);
    if (hover) {
      const sub = boxOf('menu', 'sub');
      parts.push(<Rect key="sub" b={sub} z={z} style={{background: SK.menu, border: `${z}px solid ${SK.menuLine}`, borderRadius: 8 * z, boxSizing: 'border-box'}} />);
      for (const r of s.rows.filter((row) => row.x < m.x)) {
        if (shot === 'S5c' && r.name === 'oneHour') parts.push(<Rect key={`h-${r.name}`} b={r} z={z} style={{background: SK.hover, borderRadius: 4 * z}} />);
        parts.push(<Bar key={`b-${r.name}`} x={r.x + 36} y={r.y + r.h / 2 - 4.5} w={70 + ((r.name.length * 29) % 70)} z={z} />);
      }
    }
  }
  if (set === 'tray' && TIPS[shot]) {
    const tip = boxOf('tray', TIPS[shot]);
    parts.push(<Rect key="tip" b={tip} z={z} style={{background: SK.menu, border: `${z}px solid ${SK.menuLine}`, borderRadius: 8, boxSizing: 'border-box'}} />);
    parts.push(<Bar key="t1" x={tip.x + 18} y={tip.y + tip.h * 0.3} w={tip.w * 0.3} h={12} z={z} />);
    parts.push(<Bar key="t2" x={tip.x + 18} y={tip.y + tip.h * 0.62} w={tip.w - 36} h={12} z={z} />);
  }
  return (
    <div style={{position: 'absolute', inset: 0}}>
      {parts}
      <TrayGlyph
        size={glyph.w}
        on={holding ? 1 : 0}
        colours={ON_DARK}
        page={TASKBAR_DARK}
        style={{position: 'absolute', left: glyph.x * z, top: glyph.y * z, width: glyph.w * z, height: glyph.h * z, imageRendering: 'pixelated'}}
      />
    </div>
  );
};
