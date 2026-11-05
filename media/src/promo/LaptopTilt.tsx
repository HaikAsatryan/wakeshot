import React from 'react';
import {DARK, MOSS} from '../theme.ts';
import {Terminal} from './Terminal.tsx';

/**
 * The laptop's size in pixels: width, the base's depth (the lid is as tall, so it covers the base when shut), the
 * base's and the lid's thickness, the screen's bezel.
 */
export const LAPTOP = {w: 760, d: 480, base: 16, lid: 7, bezel: 14};
/** The lid's angle: leaning back 15 degrees when open, flat on the base when shut. */
const OPEN = 15;
const SHUT = -90;

// Graphite, lit from the top left: a neutral metal reads as a real laptop and leaves the colour to the light.
const METAL = {
  deck: 'linear-gradient(170deg, #4A5056 0%, #3A3F44 55%, #33373B 100%)',
  shell: 'linear-gradient(150deg, #545A60 0%, #3C4146 45%, #2C3034 100%)',
  edge: 'linear-gradient(180deg, #5C636A 0%, #3A3E42 40%, #25282B 100%)',
  side: 'linear-gradient(180deg, #4C5258 0%, #2A2D30 100%)',
  rim: 'rgba(255,255,255,0.10)',
  key: '#1D2023',
  keyTop: '#25292D',
};

const face: React.CSSProperties = {position: 'absolute', left: 0, top: 0, backfaceVisibility: 'hidden'};

/** The keyboard: six rows of keys on the deck, the space bar wide, a little room around them. */
const Keys: React.FC<{w: number; h: number}> = ({w, h}) => {
  const rows = 6;
  const cols = 14;
  const gap = 5;
  const kw = (w - gap * (cols - 1)) / cols;
  const kh = (h - gap * (rows - 1)) / rows;
  const keys: React.ReactNode[] = [];
  for (let r = 0; r < rows; r++) {
    // The bottom row: a space bar where the middle six keys would be.
    const row = r === rows - 1 ? [[0, 1], [1, 1], [2, 1], [3, 1], [4, 6], [10, 1], [11, 1], [12, 1], [13, 1]] : Array.from({length: cols}, (_, c) => [c, 1]);
    for (const [c, span] of row) {
      keys.push(
        <div
          key={`${r}-${c}`}
          style={{
            position: 'absolute',
            left: c * (kw + gap),
            top: r * (kh + gap),
            width: span * kw + (span - 1) * gap,
            height: r === 0 ? kh * 0.7 : kh,
            borderRadius: 5,
            background: `linear-gradient(180deg, ${METAL.keyTop} 0%, ${METAL.key} 100%)`,
            boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.05), 0 1px 0 rgba(0,0,0,0.5)',
          }}
        />,
      );
    }
  }
  return <div style={{position: 'relative', width: w, height: h}}>{keys}</div>;
};

/**
 * A laptop in three-quarter view, drawn with CSS 3D (perspective 1800 px): the base with its keys, touchpad, edges and
 * a power light on the front; the lid on its hinge with the terminal on its screen, a shell and a front edge that
 * carries the holding-green light when shut. `shut` 0 is open, 1 shut; `yaw` turns the laptop (the slow orbit);
 * `edge` and `led` are the lights' strength. Our own drawing in neutral graphite; it stands for any laptop.
 */
export const LaptopTilt: React.FC<{f: number; shut: number; yaw: number; edge: number; led: number; streamFrom: number}> = ({
  f,
  shut,
  yaw,
  edge,
  led,
  streamFrom,
}) => {
  const {w, d, base, lid, bezel} = LAPTOP;
  const angle = OPEN + (SHUT - OPEN) * shut;
  const display = {w: w - 2 * bezel, h: d - 2 * bezel - 12};
  const scale = display.w / 1300;
  return (
    <div style={{position: 'relative', width: w, height: d}}>
      {/* The floor's soft shadow, drawn flat: wider than the laptop, darkest under its middle. */}
      <div
        style={{
          position: 'absolute',
          left: -w * 0.18,
          width: w * 1.36,
          top: d * 0.62,
          height: d * 0.62,
          borderRadius: '50%',
          background: 'radial-gradient(closest-side, rgba(0,0,0,0.6), rgba(0,0,0,0.25) 60%, rgba(0,0,0,0))',
        }}
      />
      <div style={{position: 'absolute', inset: 0, perspective: '1800px'}}>
        <div
          style={{
            position: 'absolute',
            left: 0,
            top: d * 0.6,
            width: w,
            height: 0,
            transformStyle: 'preserve-3d',
            transform: `rotateX(-22deg) rotateY(${yaw}deg) translateZ(${-d / 2}px)`,
          }}
        >
          {/* The base's top: the deck, the keys, the touchpad. */}
          <div
            style={{
              ...face,
              width: w,
              height: d,
              transformOrigin: 'top center',
              transform: 'rotateX(90deg)',
              background: METAL.deck,
              borderRadius: '6px 6px 16px 16px',
              boxShadow: `inset 0 0 0 1px ${METAL.rim}`,
            }}
          >
            <div style={{position: 'absolute', left: 40, top: 30, padding: 10, borderRadius: 10, background: 'rgba(0,0,0,0.22)'}}>
              <Keys w={w - 100} h={d * 0.44} />
            </div>
            <div
              style={{
                position: 'absolute',
                left: w / 2 - 135,
                width: 270,
                top: d * 0.66,
                height: d * 0.27,
                borderRadius: 12,
                background: 'linear-gradient(180deg, #43484E 0%, #3A3E43 100%)',
                boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.35), inset 0 1px 0 rgba(255,255,255,0.06)',
              }}
            />
          </div>
          {/* The base's front edge, with the power light, and its right side. */}
          <div style={{...face, width: w, height: base, transform: `translateZ(${d}px)`, background: METAL.edge, borderRadius: '0 0 12px 12px'}}>
            <div
              style={{
                position: 'absolute',
                left: w / 2 - 6,
                top: base / 2 - 2,
                width: 12,
                height: 4,
                borderRadius: 2,
                background: MOSS.lightCore,
                opacity: led,
                boxShadow: `0 0 8px 1px ${MOSS.lightCore}`,
              }}
            />
          </div>
          <div style={{...face, backfaceVisibility: 'visible', left: w, width: d, height: base, transformOrigin: 'left top', transform: 'rotateY(-90deg)', background: METAL.side}} />
          {/* The lid on its hinge at the base's back edge: the screen side, the shell, and the front edge. */}
          <div
            style={{
              position: 'absolute',
              left: 0,
              top: -d,
              width: w,
              height: d,
              transformOrigin: 'bottom center',
              transform: `rotateX(${angle}deg)`,
              transformStyle: 'preserve-3d',
            }}
          >
            <div
              style={{
                ...face,
                width: w,
                height: d,
                transform: `translateZ(${lid / 2}px)`,
                background: '#0B0C0D',
                borderRadius: '16px 16px 4px 4px',
                boxShadow: `inset 0 0 0 1px ${METAL.rim}`,
                overflow: 'hidden',
              }}
            >
              <div style={{position: 'absolute', left: bezel, top: bezel, width: display.w, height: display.h, overflow: 'hidden', borderRadius: 3, background: DARK.terminal}}>
                <div style={{transform: `scale(${scale})`, transformOrigin: '0 0'}}>
                  <Terminal f={f} streamFrom={streamFrom} every={6} stopAt={Infinity} rows={9} />
                </div>
                {/* Glass: a soft sheen from the top left. */}
                <div style={{position: 'absolute', inset: 0, background: 'linear-gradient(125deg, rgba(255,255,255,0.07) 0%, rgba(255,255,255,0) 40%)'}} />
              </div>
              <div style={{position: 'absolute', left: w / 2 - 3, top: bezel / 2 - 3, width: 6, height: 6, borderRadius: 3, background: '#1E2124'}} />
            </div>
            <div
              style={{
                ...face,
                width: w,
                height: d,
                transform: `rotateY(180deg) translateZ(${lid / 2}px)`,
                background: METAL.shell,
                borderRadius: '16px 16px 4px 4px',
                boxShadow: `inset 0 0 0 1px ${METAL.rim}`,
              }}
            />
            <div
              style={{
                ...face,
                backfaceVisibility: 'visible',
                top: -lid / 2,
                width: w,
                height: lid,
                transform: 'rotateX(90deg)',
                background: METAL.edge,
                borderRadius: 3,
              }}
            >
              <div
                style={{
                  position: 'absolute',
                  left: 60,
                  right: 60,
                  top: 1,
                  bottom: 1,
                  borderRadius: 3,
                  background: MOSS.holdingOnDark,
                  opacity: edge,
                  boxShadow: `0 0 14px 3px ${MOSS.holdingOnDark}99`,
                }}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
