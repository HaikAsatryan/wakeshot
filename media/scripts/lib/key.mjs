// Finds the key of a stretch of music, so a chord added over a found track sits in tune with it: the energy of each
// pitch class (Goertzel filters on every semitone from C2 to B6, in 0.1 s blocks), matched against the major and minor
// key profiles of Krumhansl and Kessler.
import {RATE} from './audio.mjs';

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const MAJOR = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MINOR = [6.33, 2.68, 3.52, 5.38, 2.6, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

/** Energy per pitch class of the mono mix of `L` and `R` between `from` and `to` seconds. */
export function chroma(L, R, from, to) {
  const out = new Float64Array(12);
  const block = Math.round(0.1 * RATE);
  for (let midi = 36; midi < 96; midi++) {
    const hz = 440 * 2 ** ((midi - 69) / 12);
    const c = 2 * Math.cos((2 * Math.PI * hz) / RATE);
    for (let s = Math.round(from * RATE); s + block <= Math.round(to * RATE); s += block) {
      let a = 0;
      let b = 0;
      for (let i = s; i < s + block; i++) {
        const x = (L[i] + R[i]) / 2;
        const y = x + c * a - b;
        b = a;
        a = y;
      }
      out[midi % 12] += Math.sqrt(Math.max(0, a * a + b * b - c * a * b));
    }
  }
  return out;
}

/** The best matching key: its tonic (0 is C), whether it is minor, and the correlation of the match. */
export function keyOf(ch) {
  const corr = (p, shift) => {
    const xs = [...ch];
    const ys = p.map((_, i) => p[(i - shift + 12) % 12]);
    const mx = xs.reduce((u, v) => u + v, 0) / 12;
    const my = ys.reduce((u, v) => u + v, 0) / 12;
    let num = 0;
    let dx = 0;
    let dy = 0;
    for (let i = 0; i < 12; i++) {
      num += (xs[i] - mx) * (ys[i] - my);
      dx += (xs[i] - mx) ** 2;
      dy += (ys[i] - my) ** 2;
    }
    return num / Math.sqrt(dx * dy);
  };
  let best = {tonic: 0, minor: false, r: -Infinity};
  for (let k = 0; k < 12; k++) {
    for (const [minor, p] of [
      [false, MAJOR],
      [true, MINOR],
    ]) {
      const r = corr(p, k);
      if (r > best.r) best = {tonic: k, minor, r};
    }
  }
  return {...best, name: `${NAMES[best.tonic]} ${best.minor ? 'minor' : 'major'}`};
}
