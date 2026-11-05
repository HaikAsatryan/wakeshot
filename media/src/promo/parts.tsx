import React from 'react';
import {Easing, Img, interpolate, staticFile, useCurrentFrame} from 'remotion';
import shotsData from '../../data/shots.json';
import {COPY} from '../copy.ts';
import {MONO, SANS} from '../fonts.ts';
import {DARK, LIGHT, MOSS} from '../theme.ts';
import {TERMINAL} from '../timeline.ts';

type ShotInfo = {
  shot: string;
  what: string;
  theme: 'light' | 'dark';
  file: string | null;
  w: number;
  h: number;
  standIn: boolean;
  from: string;
};
const SHOTS = shotsData.shots as Record<string, ShotInfo>;

export const ease = (frame: number, from: number, to: number, out: [number, number] = [0, 1]) =>
  interpolate(frame, [from, to], out, {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
    easing: Easing.bezier(0.33, 0, 0.2, 1),
  });

/** Video-wide settings the scenes read. */
export const DraftContext = React.createContext(true);

/**
 * A real capture from `data/shots.json`, shown `width` pixels wide. At a whole-number zoom the pixels stay hard, so a
 * screen picture is shown as it was taken; at other zooms the browser smooths it, since uneven hard pixels garble
 * text. A stand-in carries a tag in the draft. A missing capture is a plain box of the expected size that names the
 * shot, so a draft runs before the captures exist and never draws an old picture; the final video throws instead.
 */
export const Shot: React.FC<{id: string; width: number; style?: React.CSSProperties}> = ({id, width, style}) => {
  const draft = React.useContext(DraftContext);
  const info = SHOTS[id];
  if (!info) throw new Error(`no shot ${id} in data/shots.json`);
  if (!info.file && !draft) throw new Error(`shot ${info.shot} ${id} has no capture; the final video needs it`);
  const zoom = width / info.w;
  // A missing light capture is drawn light, so a scene that wipes from light to dark still shows the wipe.
  const plain = info.theme === 'light' ? LIGHT : DARK;
  const box: React.CSSProperties = {width, height: info.h * zoom, position: 'relative', ...style};
  return (
    <div style={box}>
      {info.file ? (
        <Img
          src={staticFile(info.file)}
          style={{
            width: '100%',
            height: '100%',
            imageRendering: Number.isInteger(zoom) ? 'pixelated' : 'auto',
            borderRadius: 8 * Math.min(zoom, 2),
          }}
        />
      ) : (
        <div
          style={{
            boxSizing: 'border-box',
            width: '100%',
            height: '100%',
            background: plain.panel,
            border: `3px dashed ${plain.panelLine}`,
            borderRadius: 12,
            padding: 16,
            fontFamily: SANS,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            textAlign: 'center',
            gap: 6,
          }}
        >
          <div style={{color: plain.text, fontSize: 28, fontWeight: 600}}>
            {info.shot}: {info.what}
          </div>
          <div style={{color: plain.dim, fontSize: 22}}>{COPY.missing}</div>
        </div>
      )}
      {draft && info.standIn && info.file ? (
        <div
          style={{
            position: 'absolute',
            left: 6,
            top: 6,
            background: DARK.draft,
            color: DARK.onDraft,
            fontFamily: SANS,
            fontWeight: 600,
            fontSize: 18,
            padding: '2px 8px',
            borderRadius: 4,
          }}
        >
          {info.shot} {COPY.standIn}
        </div>
      ) : null}
    </div>
  );
};

/** A caption in the lower third that rises in at `from` (scene frames). */
export const Caption: React.FC<{text: string; from?: number; small?: string; smallFrom?: number}> = ({
  text,
  from = 0,
  small,
  smallFrom,
}) => {
  const f = useCurrentFrame();
  const show = ease(f, from, from + 14);
  const showSmall = small ? ease(f, smallFrom ?? from + 20, (smallFrom ?? from + 20) + 14) : 0;
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: 96,
        textAlign: 'center',
        fontFamily: SANS,
        color: DARK.text,
      }}
    >
      <div style={{fontSize: 56, fontWeight: 600, opacity: show, transform: `translateY(${(1 - show) * 24}px)`}}>
        {text}
      </div>
      {small ? (
        <div style={{fontSize: 30, color: DARK.dim, marginTop: 16, opacity: showSmall}}>{small}</div>
      ) : null}
    </div>
  );
};

// A check mark for the terminal's done line, built from its code point so the source stays plain ASCII.
const CHECK = String.fromCodePoint(0x2713);

/** A terminal card drawn in code: the command, the prompt, then a working line or a done line. */
export const Terminal: React.FC<{
  typeCommand: number;
  typePrompt: number;
  enter: number;
  done?: number;
  doneText?: string;
}> = ({typeCommand, typePrompt, enter, done, doneText = 'Done. All tests pass.'}) => {
  const f = useCurrentFrame();
  const cmd = TERMINAL.command.slice(0, Math.max(0, Math.floor((f - typeCommand) / TERMINAL.commandRate)));
  const promptShown = f >= typePrompt - 4;
  const prompt = TERMINAL.prompt.slice(0, Math.max(0, Math.floor((f - typePrompt) / TERMINAL.promptRate)));
  const working = f >= enter;
  const isDone = done !== undefined && f >= done;
  const dots = '.'.repeat(1 + (Math.floor(f / 8) % 3));
  const caret = Math.floor(f / 15) % 2 === 0 ? String.fromCodePoint(0x2588) : ' ';
  const line: React.CSSProperties = {whiteSpace: 'pre', height: 44};
  return (
    <div
      style={{
        width: 980,
        height: 330,
        background: DARK.terminal,
        border: `2px solid ${DARK.panelLine}`,
        borderRadius: 16,
        overflow: 'hidden',
        fontFamily: MONO,
        fontSize: 30,
        color: DARK.terminalText,
        boxShadow: '0 30px 80px rgba(0,0,0,0.45)',
      }}
    >
      <div style={{height: 52, background: DARK.panel, display: 'flex', alignItems: 'center', paddingLeft: 22}}>
        <div
          style={{
            fontFamily: SANS,
            fontSize: 22,
            color: DARK.dim,
            background: DARK.terminal,
            padding: '10px 20px',
            borderRadius: '10px 10px 0 0',
            marginTop: 12,
          }}
        >
          Terminal
        </div>
      </div>
      <div style={{padding: '24px 32px'}}>
        <div style={line}>
          <span style={{color: DARK.dim}}>{TERMINAL.path} </span>
          {cmd}
          {!promptShown ? caret : ''}
        </div>
        {promptShown ? (
          <div style={line}>
            <span style={{color: DARK.dim}}>{'> '}</span>
            {prompt}
            {!working ? caret : ''}
          </div>
        ) : null}
        {working ? (
          <div style={{...line, color: isDone ? DARK.text : DARK.accent}}>
            {isDone ? `${CHECK} ${doneText}` : `* Working${dots}`}
          </div>
        ) : null}
      </div>
    </div>
  );
};

/**
 * A laptop seen from the side: the lid turns down onto the base as `closed` goes 0 to 1. `lock` shows a padlock and
 * `moon` a sleep sign above it.
 */
export const Laptop: React.FC<{closed: number; lock: number; moon?: number}> = ({closed, lock, moon = 0}) => {
  // Room above the hinge for the open lid, which reaches about 430 px up.
  const hinge = {x: 170, y: 470};
  const angle = interpolate(closed, [0, 1], [-104, 0]);
  return (
    <svg width={760} height={580} viewBox="0 0 760 580">
      <rect x={hinge.x - 10} y={hinge.y} width={480} height={22} rx={10} fill={DARK.dim} />
      <g transform={`rotate(${angle} ${hinge.x} ${hinge.y})`}>
        <rect x={hinge.x} y={hinge.y - 14} width={440} height={14} rx={7} fill={DARK.panel} stroke={DARK.dim} strokeWidth={5} />
        <rect x={hinge.x + 24} y={hinge.y - 4} width={392} height={4} rx={2} fill={MOSS.lightEdge} opacity={0.8 * (1 - closed)} />
      </g>
      <g opacity={lock} transform={`translate(400 ${340 - 10 * lock}) scale(${0.8 + 0.2 * lock})`}>
        <rect x={-50} y={-10} width={100} height={80} rx={14} fill={DARK.text} />
        <path d="M -30 -10 L -30 -40 A 30 30 0 0 1 30 -40 L 30 -10" fill="none" stroke={DARK.text} strokeWidth={14} />
        <circle cx={0} cy={26} r={10} fill={DARK.page} />
      </g>
      <g opacity={moon} transform={`translate(400 ${340 - 10 * moon})`}>
        <path d="M 0 -50 A 50 50 0 1 0 50 12 A 40 40 0 1 1 0 -50 Z" fill={DARK.dim} />
      </g>
    </svg>
  );
};

/** The draft mark: on every frame of a draft render, so a draft is never mistaken for the final video. */
export const DraftBanner: React.FC = () => (
  <div
    style={{
      position: 'absolute',
      top: 24,
      right: 24,
      background: DARK.draft,
      color: DARK.onDraft,
      fontFamily: SANS,
      fontWeight: 600,
      fontSize: 24,
      padding: '8px 16px',
      borderRadius: 8,
    }}
  >
    {COPY.draft}
  </div>
);
