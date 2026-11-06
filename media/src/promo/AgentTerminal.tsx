import React from 'react';
import {COPY} from '../copy.ts';
import {MONO} from '../fonts.ts';
import {DARK} from '../theme.ts';
import {KEYS, T} from '../timeline.ts';
import {Spinner, TitleBar} from './Terminal.tsx';

/**
 * The pain's terminal, in terminal pixels: the window, its title bar, the padding, one line's height and the type
 * size. One size for both places it shows (the flat window seen close and the laptop's screen), so a cut between them
 * shows the same screen. 1500 x 906 is the laptop display's shape.
 */
export const OPEN_TERMINAL = {w: 1500, h: 906, bar: 54, padX: 44, padTop: 30, line: 56, font: 36, boxPad: 14, gap: 20};

/** A bullet, built from its code point so the source stays plain ASCII. */
const DOT = String.fromCodePoint(0x2022);

/**
 * How fast the agent's clock runs, in clock seconds per video second: real time until `T.hook.lapse`, then it eases
 * up over `RAMP` frames to `LAPSE` (a time lapse), so the minutes go by while the person is away. At the cut it reads
 * about 4 minutes, when the laptop sleeps about 9.
 */
const LAPSE = 150;
const RAMP = 45;
const speedAt = (g: number) => {
  const x = Math.min(1, Math.max(0, (g - T.hook.lapse) / RAMP));
  return 1 + (LAPSE - 1) * x * x * (3 - 2 * x);
};

/** The agent's clock at frame `f`, in seconds: it stops when the laptop sleeps. */
export function agentClock(f: number): number {
  let s = 0;
  for (let g = T.hook.enter; g < Math.min(f, T.hook.sleep); g++) s += speedAt(g) / 30;
  return s;
}

const clockText = (s: number) => {
  const whole = Math.floor(s);
  return whole < 60 ? `${whole}s` : `${Math.floor(whole / 60)}m ${String(whole % 60).padStart(2, '0')}s`;
};

const hexRgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const mix = (a: string, b: string, t: number) => {
  const [x, y] = [hexRgb(a), hexRgb(b)];
  return `rgb(${x.map((v, i) => Math.round(v + (y[i] - v) * t)).join(',')})`;
};

/**
 * The waiting word with a slow shimmer: a bright band runs across its letters again and again, the way an agent's
 * thinking line shows it is still busy when nothing else moves. Each letter's colour comes from the frame, so every
 * render draws the same.
 */
const Shimmer: React.FC<{f: number; text: string; moving: boolean}> = ({f, text, moving}) => (
  <span>
    {[...text].map((ch, i) => {
      const period = text.length + 10;
      const at = (((f * 0.4 - i) % period) + period) % period;
      const glow = moving ? Math.exp(-((at - 2) ** 2) / 3) : 0;
      return (
        <span key={i} style={{color: mix(DARK.dim, DARK.text, glow)}}>
          {ch}
        </span>
      );
    })}
  </span>
);

/**
 * A drawn terminal with an agent waiting for a prompt, then working on it: our illustration in our colours, not a
 * copy of any app's screen. The command was run before the video starts; the input box waits with its caret; the
 * prompt comes in key by key (`KEYS`); at the Enter the box gives way to the sent prompt and a thinking line with a
 * turning spinner and the agent's clock; a tool line comes now and then. From `T.hook.sleep` nothing moves.
 */
export const AgentTerminal: React.FC<{f: number}> = ({f}) => {
  const g = OPEN_TERMINAL;
  const t = COPY.terminal;
  const h = T.hook;
  const typed = KEYS.filter((k) => f >= k.at)
    .map((k) => k.ch)
    .join('');
  const sent = f >= h.enter;
  const last = KEYS.filter((k) => f >= k.at).pop()?.at ?? -Infinity;
  // A terminal's caret stays solid while keys come and blinks once they stop.
  const caret = f - last < 15 || Math.floor(f / 15) % 2 === 0;
  const shown = h.lines.filter((at) => f >= at).length;
  const moving = f < h.sleep;
  const row: React.CSSProperties = {height: g.line, whiteSpace: 'pre', display: 'flex', alignItems: 'center'};
  return (
    <div
      style={{
        position: 'relative',
        width: g.w,
        height: g.h,
        boxSizing: 'border-box',
        background: DARK.terminal,
        border: '1px solid rgba(255,255,255,0.10)',
        borderRadius: 14,
        overflow: 'hidden',
        fontFamily: MONO,
        fontSize: g.font,
        color: DARK.terminalText,
        boxShadow: '0 50px 120px rgba(0,0,0,0.6), 0 0 0 1px rgba(0,0,0,0.4)',
      }}
    >
      <TitleBar title={t.title} />
      <div style={{padding: `${g.padTop}px ${g.padX}px 0`}}>
        <div style={row}>
          <span style={{color: DARK.dim}}>{t.path} </span>
          {t.command}
        </div>
        <div
          style={{
            ...row,
            marginTop: g.gap,
            padding: `${g.boxPad}px 18px`,
            borderRadius: 10,
            // The input box: its frame goes at the Enter, and the prompt stays where it was, now part of the log.
            border: `2px solid ${sent ? 'transparent' : 'rgba(163,173,153,0.45)'}`,
          }}
        >
          <span style={{color: sent ? DARK.dim : DARK.accent}}>{'> '}</span>
          {typed}
          {!sent ? <span style={{display: 'inline-block', width: '0.6em', height: g.font * 1.05, marginLeft: 2, background: DARK.terminalText, opacity: caret ? 0.9 : 0}} /> : null}
        </div>
        <div style={{marginTop: g.gap - 6, paddingLeft: 20}}>
          {t.now.slice(0, shown).map((n) => {
            const r = t.lines[n];
            return (
              <div key={n} style={row}>
                <span style={{color: DARK.accent, width: '1.6ch', display: 'inline-block'}}>{DOT}</span>
                <span style={{color: DARK.dim, width: '5.5ch', display: 'inline-block'}}>{r[0]}</span>
                {r[1]}
                {r[2] ? <span style={{color: DARK.dim}}>{`  ${r[2]}`}</span> : null}
              </div>
            );
          })}
          {sent ? (
            <div style={{...row, marginTop: shown ? 12 : 0, gap: 18}}>
              <Spinner f={f} colour={DARK.accent} turning={moving} />
              <Shimmer f={f} text={t.thinking} moving={moving} />
              <span style={{color: DARK.dim}}>({clockText(agentClock(f))})</span>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
};
