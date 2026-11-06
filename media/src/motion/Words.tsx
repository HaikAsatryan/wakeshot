import React from 'react';
import {useCurrentFrame} from 'remotion';
import {MONO, SANS} from '../fonts.ts';
import {DARK} from '../theme.ts';
import {WORD_IN, WORD_OUT, wordStarts, wordsOf, type CaptionBlock, type TextStyle} from '../timeline.ts';
import {SETTLE, progress} from './ease.ts';

/** Instrument Sans: cap height 720 of 1000 units per em (OS/2 `sCapHeight`); JetBrains Mono: 730. */
const CAP = {sans: 0.72, mono: 0.73};

/** Each style's size and look (`promo-video.md`, "Text"). */
export const STYLE: Record<TextStyle, {size: number; weight: number; colour: string; family: string; cap: number}> = {
  big: {size: 140, weight: 600, colour: DARK.text, family: SANS, cap: CAP.sans},
  name: {size: 140, weight: 600, colour: DARK.text, family: SANS, cap: CAP.sans},
  caption: {size: 60, weight: 400, colour: DARK.text, family: SANS, cap: CAP.sans},
  line: {size: 44, weight: 400, colour: DARK.dim, family: SANS, cap: CAP.sans},
  mono: {size: 40, weight: 400, colour: DARK.accent, family: MONO, cap: CAP.mono},
  // The end card's speed line: 40 px, so the measured numbers still read at the README's 900 px (review m8).
  small: {size: 40, weight: 400, colour: DARK.text, family: SANS, cap: CAP.sans},
};

/** Captions: the last line's baseline at 86% of the frame height, a line above it 72 px higher. */
export const CAPTION_BASELINE = 928;
export const CAPTION_STEP = 72;

/** A word split around its emphasis marks: `*on*,` is "", "on", ",". */
function split(word: string): {pre: string; em: string | null; post: string} {
  const m = word.match(/^(.*)\*(.+)\*(.*)$/);
  return m ? {pre: m[1], em: m[2], post: m[3]} : {pre: word, em: null, post: ''};
}

/** The plain text of a line, without its emphasis marks. */
export const plain = (text: string) => text.replaceAll('*', '');

/**
 * A block of lines, each placed by its baseline (`baselines`, screen pixels) and centred on `centreX`. Words come in
 * one by one: each fades in from an 8 px blur and rises 16 px over 9 frames, the next 3 frames later; a `whole` line
 * comes in at once. The block leaves at `until`: a 6 frame fade with 6 px of blur when the camera moves on, gone at
 * once when it cuts. The blur is the word's own text shadow, faded against the sharp word.
 */
export const Words: React.FC<{block: CaptionBlock; baselines: number[]; centreX?: number; fill?: Record<string, string>}> = ({
  block,
  baselines,
  centreX = 960,
  fill = {},
}) => {
  const f = useCurrentFrame();
  if (f >= block.until + (block.leave === 'fade' ? WORD_OUT : 0) && block.leave !== 'stay') return null;
  const out = block.leave === 'fade' ? progress(f, block.until, WORD_OUT, (t) => t) : 0;
  const starts = wordStarts(block);
  return (
    <div style={{position: 'absolute', inset: 0}}>
      {block.lines.map((line, i) => {
        const s = STYLE[line.style];
        const boxed = line.style === 'mono';
        return (
          <div
            key={i}
            style={{
              position: 'absolute',
              left: centreX - 960,
              width: 1920,
              top: baselines[i] - s.cap * s.size,
              textAlign: 'center',
              whiteSpace: 'nowrap',
              fontFamily: s.family,
              fontSize: s.size,
              fontWeight: s.weight,
              color: s.colour,
              letterSpacing: line.style === 'name' || line.style === 'big' ? '-0.01em' : undefined,
              textBox: 'trim-both cap alphabetic',
            }}
          >
            {wordsOf(line.text.replace(/\{(\w+)\}/g, (m, key: string) => fill[key] ?? m)).map((word, k) => {
              const t = line.already ? 1 : progress(f, starts[i][k], line.whole ? 4 : WORD_IN, line.whole ? (x) => x : SETTLE);
              const {pre, em, post} = split(word);
              const blur = Math.max(line.whole ? 0 : 8 * (1 - t), 6 * out);
              const shown = t * (1 - out);
              const sharp = 1 - blur / 8;
              const text = (
                <>
                  {pre}
                  {em !== null ? <span style={{color: DARK.accent, fontWeight: 600}}>{em}</span> : null}
                  {post}
                </>
              );
              return (
                <React.Fragment key={k}>
                  {k > 0 ? ' ' : null}
                  <span
                    style={{
                      display: 'inline-block',
                      position: 'relative',
                      transform: line.whole ? undefined : `translateY(${16 * (1 - t)}px)`,
                      ...(boxed ? {padding: '18px 32px', background: DARK.panel, border: `2px solid ${DARK.panelLine}`, borderRadius: 12} : {}),
                      opacity: boxed ? shown : undefined,
                    }}
                  >
                    <span style={{opacity: boxed ? 1 : shown * sharp}}>{text}</span>
                    {blur > 0.05 && !boxed ? (
                      // The blurred word is the text's own shadow with its fill hidden, not a CSS filter: a filter
                      // gets its own GPU layer, and those frames came out a level apart from one render to the next.
                      <span
                        style={{
                          position: 'absolute',
                          inset: 0,
                          opacity: shown * (1 - sharp),
                          WebkitTextFillColor: 'transparent',
                          textShadow: `0 0 ${blur.toFixed(2)}px currentColor`,
                        }}
                      >
                        {text}
                      </span>
                    ) : null}
                  </span>
                </React.Fragment>
              );
            })}
          </div>
        );
      })}
    </div>
  );
};
