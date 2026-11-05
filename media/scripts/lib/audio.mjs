// Small audio helpers for the soundtrack, plain JavaScript so every build gives the same bytes: a seeded noise source,
// 16-bit WAV in and out, loudness by ITU-R BS.1770-4 (the measure ffmpeg's `loudnorm` and the platforms use) and a
// look-ahead peak limiter.
import {readFileSync, writeFileSync} from 'node:fs';

export const RATE = 48000;

/** A seeded generator (mulberry32), so the noise in every whoosh is the same on every build. */
export function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A raised-cosine ramp from 0 at `x` 0 to 1 at `x` 1. */
export const fade = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : 0.5 - 0.5 * Math.cos(Math.PI * x));

/** Writes stereo 48 kHz 16-bit PCM. */
export function writeWav(path, L, R) {
  const n = L.length;
  const data = Buffer.alloc(n * 4);
  for (let i = 0; i < n; i++) {
    data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, L[i])) * 32767), i * 4);
    data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, R[i])) * 32767), i * 4 + 2);
  }
  const header = Buffer.alloc(44);
  header.write('RIFF', 0, 'latin1');
  header.writeUInt32LE(36 + data.length, 4);
  header.write('WAVEfmt ', 8, 'latin1');
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(2, 22);
  header.writeUInt32LE(RATE, 24);
  header.writeUInt32LE(RATE * 4, 28);
  header.writeUInt16LE(4, 32);
  header.writeUInt16LE(16, 34);
  header.write('data', 36, 'latin1');
  header.writeUInt32LE(data.length, 40);
  writeFileSync(path, Buffer.concat([header, data]));
}

/** Reads stereo 48 kHz 16-bit PCM (as ffmpeg writes it with `-ar 48000 -ac 2 -c:a pcm_s16le`). */
export function readWav(path) {
  const b = readFileSync(path);
  if (b.toString('latin1', 0, 4) !== 'RIFF' || b.toString('latin1', 8, 12) !== 'WAVE') throw new Error(`${path}: not a WAV`);
  let at = 12;
  let fmt = null;
  while (at + 8 <= b.length) {
    const id = b.toString('latin1', at, at + 4);
    const size = b.readUInt32LE(at + 4);
    if (id === 'fmt ') fmt = {format: b.readUInt16LE(at + 8), channels: b.readUInt16LE(at + 10), rate: b.readUInt32LE(at + 12), bits: b.readUInt16LE(at + 22)};
    if (id === 'data') {
      if (!fmt || fmt.format !== 1 || fmt.channels !== 2 || fmt.rate !== RATE || fmt.bits !== 16) {
        throw new Error(`${path}: needs 16-bit stereo PCM at ${RATE} Hz`);
      }
      const n = Math.floor(Math.min(size, b.length - at - 8) / 4);
      const L = new Float64Array(n);
      const R = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        L[i] = b.readInt16LE(at + 8 + i * 4) / 32768;
        R[i] = b.readInt16LE(at + 8 + i * 4 + 2) / 32768;
      }
      return [L, R];
    }
    at += 8 + size + (size % 2);
  }
  throw new Error(`${path}: no data chunk`);
}

/** A biquad filter run over a whole signal. */
function biquad(x, [b0, b1, b2], [, a1, a2]) {
  const y = new Float64Array(x.length);
  let x1 = 0;
  let x2 = 0;
  let y1 = 0;
  let y2 = 0;
  for (let i = 0; i < x.length; i++) {
    const v = b0 * x[i] + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
    x2 = x1;
    x1 = x[i];
    y2 = y1;
    y1 = v;
    y[i] = v;
  }
  return y;
}

// BS.1770-4's K-weighting at 48 kHz: a high shelf, then a high-pass (the standard's own coefficients).
const SHELF = [[1.53512485958697, -2.69169618940638, 1.19839281085285], [1, -1.69065929318241, 0.73248077421585]];
const HIGHPASS = [[1, -2, 1], [1, -1.99004745483398, 0.99007225036621]];

/** Integrated loudness in LUFS (BS.1770-4: 400 ms blocks, 75% overlap, absolute gate -70, relative gate -10). */
export function loudness(L, R) {
  const k = (x) => biquad(biquad(x, ...SHELF), ...HIGHPASS);
  const [l, r] = [k(L), k(R)];
  const block = Math.round(0.4 * RATE);
  const step = Math.round(0.1 * RATE);
  const z = [];
  for (let s = 0; s + block <= l.length; s += step) {
    let sum = 0;
    for (let i = s; i < s + block; i++) sum += l[i] * l[i] + r[i] * r[i];
    z.push(sum / block);
  }
  const lufs = (m) => -0.691 + 10 * Math.log10(m);
  const mean = (a) => a.reduce((p, q) => p + q, 0) / a.length;
  const abs = z.filter((m) => lufs(m) > -70);
  if (!abs.length) return -Infinity;
  const gate = lufs(mean(abs)) - 10;
  return lufs(mean(abs.filter((m) => lufs(m) > gate)));
}

export const peakOf = (L, R) => {
  let p = 0;
  for (let i = 0; i < L.length; i++) p = Math.max(p, Math.abs(L[i]), Math.abs(R[i]));
  return p;
};

/**
 * Holds every sample under `ceiling` with a smooth gain: the gain starts down 5 ms before a peak (a linear ramp, so
 * no click) and comes back over 80 ms after it. Returns how far it pulled, in dB.
 */
export function limit(L, R, ceiling) {
  const n = L.length;
  const need = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const p = Math.max(Math.abs(L[i]), Math.abs(R[i]));
    need[i] = p > ceiling ? ceiling / p : 1;
  }
  const ahead = Math.round(0.005 * RATE);
  const back = new Float64Array(n);
  back[n - 1] = need[n - 1];
  for (let i = n - 2; i >= 0; i--) back[i] = Math.min(need[i], back[i + 1] + 1 / ahead);
  const release = 1 / Math.round(0.08 * RATE);
  let g = 1;
  let deepest = 1;
  for (let i = 0; i < n; i++) {
    g = Math.min(back[i], g + release);
    deepest = Math.min(deepest, g);
    L[i] *= g;
    R[i] *= g;
  }
  return 20 * Math.log10(deepest);
}
