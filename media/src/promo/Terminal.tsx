import React from 'react';
import {COPY} from '../copy.ts';
import {MONO, SANS} from '../fonts.ts';
import {DARK} from '../theme.ts';

const LINE = 52;
const FONT = 34;
const BAR = 54;

/** A small turning arc, drawn, so the spinner looks the same in any font. */
const Spinner: React.FC<{f: number; size: number; colour: string; turning: boolean}> = ({f, size, colour, turning}) => (
  <svg width={size} height={size} viewBox="-10 -10 20 20" style={{display: 'block'}}>
    <circle r={7} fill="none" stroke={colour} strokeOpacity={0.25} strokeWidth={2.4} />
    <path d="M 0 -7 A 7 7 0 0 1 7 0" fill="none" stroke={colour} strokeWidth={2.4} strokeLinecap="round" transform={`rotate(${turning ? (f * 24) % 360 : 0})`} />
  </svg>
);

/** A check mark, drawn, for the done line. */
const Check: React.FC<{size: number; colour: string}> = ({size, colour}) => (
  <svg width={size} height={size} viewBox="-10 -10 20 20" style={{display: 'block'}}>
    <circle r={8.5} fill={colour} />
    <path d="M -4 0.5 L -1 3.5 L 4.5 -3" fill="none" stroke={DARK.terminal} strokeWidth={2.4} strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/** The window's title bar: a tab with a prompt sign and the title, and thin minimise, maximise and close marks. */
const TitleBar: React.FC<{title: string}> = ({title}) => (
  <div
    style={{
      height: BAR,
      display: 'flex',
      alignItems: 'flex-end',
      paddingLeft: 14,
      background: 'linear-gradient(180deg, #1F261C 0%, #192017 100%)',
      borderBottom: `1px solid ${DARK.panelLine}`,
      position: 'relative',
    }}
  >
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        height: BAR - 10,
        padding: '0 22px 0 16px',
        background: DARK.terminal,
        borderRadius: '10px 10px 0 0',
        fontFamily: SANS,
        fontSize: 21,
        color: DARK.text,
      }}
    >
      <svg width={22} height={22} viewBox="0 0 22 22">
        <rect x={1} y={2} width={20} height={18} rx={4} fill="none" stroke={DARK.dim} strokeWidth={1.6} />
        <path d="M6 8 L9 11 L6 14 M11 14.5 L16 14.5" fill="none" stroke={DARK.accent} strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      {title}
    </div>
    <svg width={150} height={BAR} viewBox={`0 0 150 ${BAR}`} style={{position: 'absolute', right: 6, top: 0}}>
      <g stroke={DARK.dim} strokeWidth={1.6} fill="none" strokeLinecap="round">
        <path d={`M18 ${BAR / 2} H32`} />
        <rect x={66} y={BAR / 2 - 7} width={14} height={14} rx={2} />
        <path d={`M114 ${BAR / 2 - 7} L128 ${BAR / 2 + 7} M128 ${BAR / 2 - 7} L114 ${BAR / 2 + 7}`} />
      </g>
    </svg>
  </div>
);

/**
 * A drawn terminal window with an agent at work: an illustration in our colours, not a copy of any app's screen. The
 * command and the prompt are typed. From `streamFrom` a tool line arrives every `every` frames and the oldest scroll
 * away, under a turning spinner; `preset` lines are there before that. With `waitUntil`, the prompt waits with a
 * blinking caret and nothing works until that frame (the Enter). From `stopAt` the spinner stops; with `doneAt`, a
 * done line shows from that frame. `rows` lines fit above the spinner. `soft` blurs the letters by that many pixels.
 */
export const Terminal: React.FC<{
  f: number;
  streamFrom: number;
  every: number;
  stopAt: number;
  preset?: number;
  waitUntil?: number;
  doneAt?: number;
  rows?: number;
  width?: number;
  soft?: number;
}> = ({f, streamFrom, every, stopAt, preset = 3, waitUntil, doneAt, rows: fit = 7, width = 1300, soft = 0}) => {
  const t = COPY.terminal;
  const waiting = waitUntil !== undefined && f < waitUntil;
  const now = Math.min(f, stopAt);
  const shown = waiting ? 0 : preset + (now < streamFrom ? 0 : Math.floor((now - streamFrom) / every) + 1);
  const first = Math.max(0, shown - fit);
  const rows = Array.from({length: shown - first}, (_, i) => t.lines[(first + i) % t.lines.length]);
  const done = doneAt !== undefined && f >= doneAt;
  const turning = f < stopAt && !waiting;
  const caret = Math.floor(f / 15) % 2 === 0;
  const line: React.CSSProperties = {height: LINE, whiteSpace: 'pre', display: 'flex', alignItems: 'center'};
  return (
    <div
      style={{
        width,
        background: DARK.terminal,
        border: '1px solid rgba(255,255,255,0.10)',
        borderRadius: 14,
        overflow: 'hidden',
        fontFamily: MONO,
        fontSize: FONT,
        color: DARK.terminalText,
        boxShadow: '0 50px 120px rgba(0,0,0,0.6), 0 0 0 1px rgba(0,0,0,0.4)',
        // Out of focus behind the title: every letter as its own blurred shadow, not a CSS filter, which rendered a
        // level apart from run to run.
        ...(soft > 0 ? {WebkitTextFillColor: 'transparent', textShadow: `0 0 ${soft}px currentColor`} : {}),
      }}
    >
      <TitleBar title={t.title} />
      <div style={{padding: '22px 36px 26px'}}>
        <div style={line}>
          <span style={{color: DARK.dim}}>{t.path} </span>
          {t.command}
        </div>
        <div style={{...line, marginBottom: 10}}>
          <span style={{color: DARK.accent}}>{'> '}</span>
          {t.prompt}
          {waiting ? <span style={{display: 'inline-block', width: '0.6em', height: FONT * 1.05, marginLeft: 4, background: DARK.terminalText, opacity: caret ? 0.9 : 0}} /> : null}
        </div>
        <div style={{height: fit * LINE, display: 'flex', flexDirection: 'column', justifyContent: 'flex-end'}}>
          {rows.map((r, i) => (
            <div key={first + i} style={line}>
              <span style={{color: r[0] === 'pass' ? DARK.accent : DARK.dim, width: '5.5ch', display: 'inline-block'}}>{r[0]}</span>
              {r[1]}
              {r[2] ? <span style={{color: DARK.dim}}>{`  ${r[2]}`}</span> : null}
            </div>
          ))}
        </div>
        <div style={{...line, gap: 16, marginTop: 6, color: done ? DARK.accent : turning ? DARK.accent : DARK.dim}}>
          {waiting ? null : done ? <Check size={FONT * 0.9} colour={DARK.accent} /> : <Spinner f={f} size={FONT * 0.9} colour={turning ? DARK.accent : DARK.dim} turning={turning} />}
          {waiting ? '' : done ? t.done : t.working}
        </div>
      </div>
    </div>
  );
};
