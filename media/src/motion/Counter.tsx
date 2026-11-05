import React from 'react';
import {useCurrentFrame} from 'remotion';
import {POP, SETTLE, progress} from './ease.ts';

/**
 * A number that counts up from 0 to `value` over `len` frames from `at`, then settles with a 2% spring. Each digit
 * sits in a box of one fixed width (`digit` em), so the number does not shimmy while it counts, whatever the font's
 * figures are.
 */
export const Counter: React.FC<{
  at: number;
  len: number;
  value: number;
  digits: number;
  digit?: number;
  style?: React.CSSProperties;
}> = ({at, len, value, digits, digit = 0.62, style}) => {
  const f = useCurrentFrame();
  const t = progress(f, at, len, SETTLE);
  const settle = progress(f, at + len, 10, POP);
  const scale = f < at + len ? 1 : 1 + 0.02 * Math.sin(Math.min(1, settle) * Math.PI);
  const text = (value * t).toFixed(digits);
  return (
    <div style={{display: 'inline-flex', transform: `scale(${scale})`, opacity: progress(f, at, 4, (x) => x), ...style}}>
      {[...text].map((c, i) => (
        <span key={i} style={{display: 'inline-block', width: c === '.' || c === ',' ? '0.28em' : `${digit}em`, textAlign: 'center'}}>
          {c}
        </span>
      ))}
    </div>
  );
};
