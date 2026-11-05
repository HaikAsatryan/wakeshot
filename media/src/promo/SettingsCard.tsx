import React from 'react';
import settingsData from '../../data/settings.json';
import {SANS} from '../fonts.ts';
import {SETTLE, progress} from '../motion/ease.ts';
import {DARK} from '../theme.ts';

export const CARD = {w: 860, pad: 40, head: 70, row: 116};
/** The odometer roll takes 8 frames. */
const ROLL = 8;
const PILL = 64;

/** Where row `i` sits inside the card, for the focus ring: x, y, w, h from the card's top left. */
export const rowBox = (i: number) => ({x: CARD.pad - 14, y: CARD.head + i * CARD.row + 10, w: CARD.w - 2 * CARD.pad + 28, h: CARD.row - 20});

/** A small plug, drawn, for the card's title. */
const Plug: React.FC = () => (
  <svg width={26} height={26} viewBox="0 0 24 24" style={{display: 'block'}}>
    <path d="M8 3 V8 M16 3 V8 M5 8 H19 V12 A7 7 0 0 1 12 19 A7 7 0 0 1 5 12 Z M12 19 V22" fill="none" stroke={DARK.dim} strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

/**
 * The two power settings Wakeshot changed for the work, from the capture session's log (`data/settings.json`, shot
 * S9). From each row's frame in `rolls`, its value rolls from Wakeshot's (green) to the user's own, as an odometer
 * does.
 */
export const SettingsCard: React.FC<{f: number; rolls: number[]}> = ({f, rolls}) => (
  <div
    style={{
      width: CARD.w,
      boxSizing: 'border-box',
      padding: `0 ${CARD.pad}px 18px`,
      background: 'linear-gradient(180deg, #232B1F 0%, #1A2017 100%)',
      border: '1px solid rgba(255,255,255,0.09)',
      borderRadius: 26,
      fontFamily: SANS,
      boxShadow: '0 40px 110px rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.06)',
    }}
  >
    <div style={{height: CARD.head, display: 'flex', alignItems: 'center', gap: 12, color: DARK.dim, fontSize: 28}}>
      <Plug />
      Power settings
    </div>
    {settingsData.rows.map((row, i) => {
      const t = progress(f, rolls[i], ROLL, SETTLE);
      return (
        <div
          key={row.label}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            height: CARD.row,
            borderTop: '1px solid rgba(255,255,255,0.07)',
          }}
        >
          <div style={{fontSize: 46, color: DARK.text}}>{row.label}</div>
          <div style={{position: 'relative', width: 330, height: PILL, overflow: 'hidden', borderRadius: PILL / 2}}>
            {[
              {text: row.held, colour: DARK.accent, back: 'rgba(182,224,90,0.12)', y: -t},
              {text: row.own, colour: DARK.text, back: 'rgba(255,255,255,0.07)', y: 1 - t},
            ].map((v) => (
              <div
                key={v.text}
                style={{
                  position: 'absolute',
                  inset: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  fontSize: 40,
                  fontWeight: 600,
                  color: v.colour,
                  background: v.back,
                  borderRadius: PILL / 2,
                  transform: `translateY(${v.y * PILL}px)`,
                }}
              >
                {v.text}
              </div>
            ))}
          </div>
        </div>
      );
    })}
  </div>
);
