import React from 'react';
import {COPY} from '../copy.ts';
import {MONO, SANS} from '../fonts.ts';
import {DARK} from '../theme.ts';

const LINE = 52;
const FONT = 34;
const BAR = 54;

// The terminal draws no SVG: under the hook's 3D tilt and the laptop's lid, SVG shapes rendered a level apart from one
// render to the next, so its marks are plain boxes and text, which render the same every time.

/** A bullet, built from its code point so the source stays plain ASCII. */
const DOT = String.fromCodePoint(0x2022);

/**
 * Three dots, the bright one stepping along every four frames: text, so it renders the same on every run (drawn dots
 * under the hook's 3D tilt came out a level apart from run to run).
 */
export const Spinner: React.FC<{f: number; colour: string; turning: boolean}> = ({f, colour, turning}) => {
  const lead = turning ? Math.floor(f / 4) % 3 : -1;
  return (
    <span style={{letterSpacing: '-0.15em'}}>
      {[0, 1, 2].map((i) => (
        <span key={i} style={{color: i === lead ? colour : DARK.dim}}>
          {DOT}
        </span>
      ))}
    </span>
  );
};

/** A filled disc with a short bar in it, for the done line: a plain mark that needs no check glyph. */
export const Check: React.FC<{size: number; colour: string}> = ({size, colour}) => (
  <div style={{width: size, height: size, borderRadius: size / 2, background: colour, display: 'flex', alignItems: 'center', justifyContent: 'center'}}>
    <div style={{width: size * 0.42, height: size * 0.22, borderLeft: `${Math.max(2, size * 0.09)}px solid ${DARK.terminal}`, borderBottom: `${Math.max(2, size * 0.09)}px solid ${DARK.terminal}`, transform: 'translateY(-15%) rotate(-45deg)'}} />
  </div>
);

/** The window's title bar: a tab with a prompt sign and the title, and thin minimise, maximise and close marks. */
export const TitleBar: React.FC<{title: string}> = ({title}) => (
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
      <div style={{fontFamily: MONO, fontSize: 18, lineHeight: '20px', padding: '0 4px', borderRadius: 4, border: `1.5px solid ${DARK.dim}`, color: DARK.accent}}>{'>_'}</div>
      {title}
    </div>
    <div style={{position: 'absolute', right: 26, top: 0, height: BAR, display: 'flex', alignItems: 'center', gap: 36}}>
      <div style={{width: 14, height: 2, background: DARK.dim}} />
      <div style={{width: 13, height: 13, border: `2px solid ${DARK.dim}`, borderRadius: 2, boxSizing: 'border-box'}} />
      <div style={{fontFamily: SANS, fontSize: 22, lineHeight: '22px', color: DARK.dim}}>x</div>
    </div>
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
  prompt?: string;
}> = ({f, streamFrom, every, stopAt, preset = 3, waitUntil, doneAt, rows: fit = 7, width = 1300, soft = 0, prompt}) => {
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
          {prompt ?? t.prompt}
          {waiting ? <span style={{display: 'inline-block', width: '0.6em', height: FONT * 1.05, marginLeft: 4, background: DARK.terminalText, opacity: caret ? 0.9 : 0}} /> : null}
        </div>
        <div style={{height: fit * LINE, display: fit > 0 ? 'flex' : 'none', flexDirection: 'column', justifyContent: 'flex-end'}}>
          {rows.map((r, i) => (
            <div key={first + i} style={line}>
              <span style={{color: r[0] === 'pass' ? DARK.accent : DARK.dim, width: '5.5ch', display: 'inline-block'}}>{r[0]}</span>
              {r[1]}
              {r[2] ? <span style={{color: DARK.dim}}>{`  ${r[2]}`}</span> : null}
            </div>
          ))}
        </div>
        <div style={{...line, gap: 16, marginTop: 6, color: done ? DARK.accent : turning ? DARK.accent : DARK.dim}}>
          {waiting ? null : done ? <Check size={FONT * 0.9} colour={DARK.accent} /> : <Spinner f={f} colour={DARK.accent} turning={turning} />}
          {waiting ? '' : done ? t.done : t.working}
        </div>
      </div>
    </div>
  );
};
