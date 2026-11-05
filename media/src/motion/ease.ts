// The easings of `promo-video.md` "Motion", in one place, so every move in the video settles the same way.
import {Easing, interpolate} from 'remotion';

/** Camera moves, word entries and the ring's glide: a fast start and a long settle. */
export const SETTLE = Easing.bezier(0.16, 1, 0.3, 1);
/** The cursor's glide. */
export const GLIDE = Easing.inOut(Easing.cubic);
/** The light coming on, as a lamp does: the README loop's curve. */
export const LAMP_ON = Easing.out(Easing.cubic);
/** The light going off: the loop's off curve. */
export const LAMP_OFF = Easing.inOut(Easing.cubic);
/** Pops: about 4% overshoot (mass 0.6, damping 12, stiffness 200). */
export const POP = Easing.spring({mass: 0.6, damping: 12, stiffness: 200});

const clamp = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

/** 0 before `from`, 1 after `from + len`, eased in between. */
export const progress = (f: number, from: number, len: number, easing: (t: number) => number = SETTLE) =>
  len <= 0 ? (f >= from ? 1 : 0) : interpolate(f, [from, from + len], [0, 1], {...clamp, easing});

/** `progress` mapped onto `[a, b]`. */
export const between = (f: number, from: number, len: number, a: number, b: number, easing?: (t: number) => number) =>
  a + (b - a) * progress(f, from, len, easing);
