// Makes the video's whole soundtrack in code and writes public/sound/soundtrack.wav (48 kHz, stereo, 16-bit, the
// video's exact length). Every sound sits on the frames of src/timeline.ts.
//
// - Wakeshot's own chimes: the numbers are read from crates/wakeshot-windows/src/sound/chime.rs and rendered the way
//   its `render` does, so the video plays what the app plays. If that file changes shape, this script stops.
// - The music bed and the effects are made here from sine waves and seeded noise: no outside sound file, so no
//   licence question and the same bytes on every run. The bed is in C major because the chimes are C6 and G6.
import {mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {BEATS, SCENES, TERMINAL, VIDEO, at} from '../src/timeline.ts';

const here = dirname(fileURLToPath(import.meta.url));
const media = join(here, '..');
const RATE = 48000;
const LENGTH = Math.round((VIDEO.frames / VIDEO.fps) * RATE);
const frameAt = (f) => Math.round((f / VIDEO.fps) * RATE);

// ---- Wakeshot's chimes, from chime.rs ----

function readChimes() {
  const src = readFileSync(join(media, '..', 'crates', 'wakeshot-windows', 'src', 'sound', 'chime.rs'), 'utf8');
  const num = (text, key) => {
    const m = text.match(new RegExp(`${key}:\\s*(-?[0-9.]+)`));
    if (!m) throw new Error(`chime.rs: no ${key}; the file changed shape, update make-sounds.mjs`);
    return Number(m[1]);
  };
  const constant = (name) => {
    const m = src.match(new RegExp(`const ${name}: u32 = (\\d+);`));
    if (!m) throw new Error(`chime.rs: no ${name}`);
    return Number(m[1]);
  };
  const shape = (name) => {
    const m = src.match(new RegExp(`const ${name}: Shape = Shape \\{([\\s\\S]*?)\\n\\};`));
    if (!m) throw new Error(`chime.rs: no ${name}; the file changed shape, update make-sounds.mjs`);
    const body = m[1];
    const notes = [...body.matchAll(/Note \{([\s\S]*?)\}/g)].map((n) => ({
      start: num(n[1], 'start_ms'),
      length: num(n[1], 'length_ms'),
      hz: num(n[1], 'hz'),
      decay: num(n[1], 'decay'),
      overtone: num(n[1], 'overtone'),
    }));
    const tail = body.slice(body.lastIndexOf(']'));
    return {notes, peak: num(tail, 'peak_dbfs'), attack: num(tail, 'attack_ms'), release: num(tail, 'release_ms')};
  };
  return {
    lead: constant('LEAD_MS'),
    tail: constant('TAIL_MS'),
    lidClosed: shape('LID_CLOSED_HOLDING'),
    goingToSleep: shape('GOING_TO_SLEEP'),
  };
}

const fade = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : 0.5 - 0.5 * Math.cos(Math.PI * x));

/** `render` from chime.rs, without the lead and tail silence, in f64. */
function renderChime(shape) {
  const end = Math.max(...shape.notes.map((n) => n.start + n.length));
  const out = new Float64Array(Math.floor((end * RATE) / 1000));
  const attack = shape.attack / 1000;
  const release = shape.release / 1000;
  for (const n of shape.notes) {
    const first = Math.floor((n.start * RATE) / 1000);
    const count = Math.floor((n.length * RATE) / 1000);
    const length = n.length / 1000;
    for (let i = 0; i < count && first + i < out.length; i++) {
      const t = i / RATE;
      const env = fade(t / attack) * fade((length - t) / release) * Math.exp(-n.decay * t);
      const phase = 2 * Math.PI * n.hz * t;
      out[first + i] += (Math.sin(phase) + n.overtone * Math.sin(2 * phase)) * env;
    }
  }
  const loudest = out.reduce((m, s) => Math.max(m, Math.abs(s)), 0);
  const scale = 10 ** (shape.peak / 20) / loudest;
  return out.map((s) => s * scale);
}

// ---- Effects ----

function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Noise through a one-pole low-pass and high-pass, shaped by `env(t)`. */
function noise(seconds, seed, lowHz, highHz, env) {
  const r = rng(seed);
  const n = Math.round(seconds * RATE);
  const out = new Float64Array(n);
  const lp = 1 - Math.exp((-2 * Math.PI * lowHz) / RATE);
  const hp = Math.exp((-2 * Math.PI * highHz) / RATE);
  let l = 0;
  let h = 0;
  let prev = 0;
  for (let i = 0; i < n; i++) {
    l += lp * (r() * 2 - 1 - l);
    h = hp * (h + l - prev);
    prev = l;
    out[i] = h * env(i / RATE);
  }
  return out;
}

function tone(seconds, hzAt, env, harmonics = [1]) {
  const n = Math.round(seconds * RATE);
  const out = new Float64Array(n);
  let phase = 0;
  for (let i = 0; i < n; i++) {
    const t = i / RATE;
    phase += (2 * Math.PI * hzAt(t)) / RATE;
    let s = 0;
    harmonics.forEach((a, k) => (s += a * Math.sin((k + 1) * phase)));
    out[i] = s * env(t);
  }
  return out;
}

const add = (a, b) => a.map((v, i) => v + (b[i] ?? 0));

const FX = {
  key: (seed) => {
    const r = rng(seed);
    const pitch = 0.85 + r() * 0.3;
    return add(
      noise(0.035, seed, 6000 * pitch, 900, (t) => Math.exp(-t * 140)),
      tone(0.03, () => 2400 * pitch, (t) => 0.12 * Math.exp(-t * 200)),
    );
  },
  enter: () => add(noise(0.06, 7, 3500, 300, (t) => 1.3 * Math.exp(-t * 80)), tone(0.05, () => 900, (t) => 0.2 * Math.exp(-t * 90))),
  // A glide up and a glide down: the light coming on and going off.
  lightOn: () =>
    add(
      tone(0.7, (t) => 330 + 330 * fade(t / 0.7), (t) => 0.35 * fade(t / 0.15) * fade((0.7 - t) / 0.3), [1, 0.3, 0.12]),
      noise(0.7, 11, 2500, 400, (t) => 0.5 * fade(t / 0.5) * fade((0.7 - t) / 0.2)),
    ),
  lightOff: () =>
    add(
      tone(0.7, (t) => 660 - 330 * fade(t / 0.7), (t) => 0.3 * fade(t / 0.08) * fade((0.7 - t) / 0.35), [1, 0.3, 0.12]),
      noise(0.7, 13, 2000, 300, (t) => 0.4 * fade(t / 0.1) * fade((0.7 - t) / 0.5)),
    ),
  tick: () => tone(0.08, () => 1760, (t) => 0.45 * fade(t / 0.002) * Math.exp(-t * 60), [1, 0.15]),
  click: () => add(noise(0.03, 17, 5000, 1200, (t) => Math.exp(-t * 220)), tone(0.03, () => 1300, (t) => 0.25 * Math.exp(-t * 150))),
  thud: () =>
    add(
      tone(0.25, (t) => 110 - 40 * (t / 0.25), (t) => 0.9 * fade(t / 0.003) * Math.exp(-t * 22), [1, 0.4]),
      noise(0.12, 19, 900, 60, (t) => 0.8 * Math.exp(-t * 45)),
    ),
  lock: () => {
    const one = (seed) => add(noise(0.025, seed, 7000, 2000, (t) => Math.exp(-t * 260)), tone(0.025, () => 3200, (t) => 0.2 * Math.exp(-t * 240)));
    const a = one(23);
    const out = new Float64Array(Math.round(0.12 * RATE));
    out.set(a, 0);
    const b = one(29);
    b.forEach((v, i) => (out[Math.round(0.07 * RATE) + i] += v));
    return out;
  },
  whoosh: () => noise(0.5, 31, 3000, 500, (t) => 0.6 * fade(t / 0.25) * fade((0.5 - t) / 0.25)),
};

// ---- Music bed ----

const midi = (m) => 440 * 2 ** ((m - 69) / 12);
// C4 = 60. Each chord: start second, notes (MIDI), bass (MIDI).
const CHORDS = [
  {at: 0, notes: [60, 64, 67, 71, 74], bass: 36}, // Cmaj9
  {at: 5, notes: [57, 60, 64, 67, 71], bass: 45}, // Am9
  {at: 10, notes: [53, 57, 60, 64, 67], bass: 41}, // Fmaj9
  {at: 15, notes: [55, 59, 62, 64, 69], bass: 43}, // G6add9
  {at: 20, notes: [57, 60, 64, 67, 72], bass: 45}, // Am7
  {at: 25, notes: [53, 57, 60, 64, 69], bass: 41}, // Fmaj7
  {at: 28, notes: [60, 64, 67, 72, 74], bass: 36}, // C add9, the resolve under the end card
];
const BPM = 96;
const EIGHTH = 60 / BPM / 2;

function bed() {
  const L = new Float64Array(LENGTH);
  const R = new Float64Array(LENGTH);
  const total = LENGTH / RATE;
  CHORDS.forEach((c, k) => {
    const start = c.at;
    const end = k + 1 < CHORDS.length ? CHORDS[k + 1].at : total + 1.5;
    const dur = end - start;
    const voices = [...c.notes.map((m) => ({hz: midi(m), level: 0.06})), {hz: midi(c.bass), level: 0.1}];
    for (const v of voices) {
      for (const [cents, pan] of [
        [-5, 0.25],
        [5, 0.75],
      ]) {
        const hz = v.hz * 2 ** (cents / 1200);
        const first = Math.round(start * RATE);
        const n = Math.round((dur + 1.2) * RATE);
        for (let i = 0; i < n && first + i < LENGTH; i++) {
          const t = i / RATE;
          // Slow swell in, and a release that overlaps the next chord, so changes glide.
          const env = fade(t / 0.9) * (t < dur ? 1 : fade(1 - (t - dur) / 1.2));
          const ph = 2 * Math.PI * hz * (start + t);
          const s = (Math.sin(ph) + 0.25 * Math.sin(2 * ph) + 0.08 * Math.sin(3 * ph)) * env * v.level;
          L[first + i] += s * Math.sqrt(1 - pan);
          R[first + i] += s * Math.sqrt(pan);
        }
      }
    }
  });
  // A soft plucked arpeggio from the moment work starts, so the middle has motion; it stops for the end card.
  const pluckFrom = SCENES.start.from / VIDEO.fps;
  const pluckTo = SCENES.outro.from / VIDEO.fps;
  const pattern = [0, 2, 4, 2, 1, 3, 4, 3];
  const dl = Math.round(EIGHTH * 1.5 * RATE);
  const tapL = new Float64Array(LENGTH);
  const tapR = new Float64Array(LENGTH);
  for (let k = 0, t0 = pluckFrom; t0 < pluckTo; k++, t0 += EIGHTH) {
    const chord = [...CHORDS].reverse().find((c) => c.at <= t0 + 1e-9);
    const hz = midi(chord.notes[pattern[k % pattern.length]] + 12);
    const first = Math.round(t0 * RATE);
    const pan = k % 2 === 0 ? 0.35 : 0.65;
    const n = Math.round(0.9 * RATE);
    for (let i = 0; i < n && first + i < LENGTH; i++) {
      const t = i / RATE;
      const ph = 2 * Math.PI * hz * t;
      const s = (Math.sin(ph) + 0.15 * Math.sin(2 * ph)) * fade(t / 0.004) * Math.exp(-t * 5.5) * 0.045;
      tapL[first + i] += s * Math.sqrt(1 - pan);
      tapR[first + i] += s * Math.sqrt(pan);
    }
  }
  // A ping-pong echo gives the plucks room without a reverb.
  for (let i = 0; i < LENGTH; i++) {
    if (i >= dl) {
      tapL[i] += 0.35 * tapR[i - dl];
      tapR[i] += 0.35 * tapL[i - dl];
    }
    L[i] += tapL[i];
    R[i] += tapR[i];
  }
  // Fade in over the title, out over the last second.
  for (let i = 0; i < LENGTH; i++) {
    const t = i / RATE;
    const g = fade(t / 1.5) * fade((total - t) / 1.0);
    L[i] *= g;
    R[i] *= g;
  }
  return [L, R];
}

// ---- Mix ----

const [L, R] = bed();
const BED_GAIN = 0.4;
for (let i = 0; i < LENGTH; i++) {
  L[i] *= BED_GAIN;
  R[i] *= BED_GAIN;
}

function place(samples, frame, gain, pan = 0.5) {
  const first = frameAt(frame);
  for (let i = 0; i < samples.length && first + i < LENGTH; i++) {
    L[first + i] += samples[i] * gain * Math.sqrt(1 - pan);
    R[first + i] += samples[i] * gain * Math.sqrt(pan);
  }
}

/** Lowers the bed around a chime, so Wakeshot's own sound is heard clearly. */
function duck(frame, seconds, db) {
  const first = frameAt(frame);
  const n = Math.round(seconds * RATE);
  const g = 10 ** (db / 20);
  for (let i = Math.max(0, first - 4800); i < Math.min(LENGTH, first + n + 9600); i++) {
    const t = (i - first) / RATE;
    const depth = t < 0 ? fade(1 + t / 0.1) : t < seconds ? 1 : fade(1 - (t - seconds) / 0.2);
    const k = 1 - (1 - g) * depth;
    L[i] *= k;
    R[i] *= k;
  }
}

const chimes = readChimes();
const lidChime = renderChime(chimes.lidClosed);
const sleepChime = renderChime(chimes.goingToSleep);

const s = BEATS.start;
for (let i = 0; i < TERMINAL.command.length; i++) {
  place(FX.key(100 + i), SCENES.start.from + s.typeCommand + i * TERMINAL.commandRate, 0.35, 0.45);
}
for (let i = 0; i < TERMINAL.prompt.length; i += 2) {
  place(FX.key(200 + i), SCENES.start.from + s.typePrompt + i * TERMINAL.promptRate, 0.28, 0.45);
}
place(FX.enter(), at('start', 'enter'), 0.45, 0.45);
place(FX.lightOn(), at('title', 'lightOn'), 0.5);
place(FX.lightOn(), at('start', 'morph'), 0.5, 0.6);
place(FX.thud(), at('lid', 'shut'), 0.6);
duck(at('lid', 'shut') + 2, chimes.lidClosed.notes.at(-1).start / 1000 + 0.5, -9);
place(lidChime, at('lid', 'shut') + 2, 1);
place(FX.lock(), at('lid', 'lock'), 0.5);
place(FX.tick(), at('end', 'done'), 0.3, 0.4);
place(FX.lightOff(), at('end', 'morph'), 0.5, 0.6);
duck(at('end', 'sleepChime'), chimes.goingToSleep.notes.at(-1).start / 1000 + 0.6, -9);
place(sleepChime, at('end', 'sleepChime'), 1);
place(FX.tick(), at('settings', 'tick1'), 0.35);
place(FX.tick(), at('settings', 'tick2'), 0.35);
place(FX.click(), at('keepAwake', 'click1'), 0.5);
place(FX.whoosh(), at('keepAwake', 'swap'), 0.35);
place(FX.click(), at('keepAwake', 'click2'), 0.5);
for (const b of ['tick1', 'tick2', 'tick3']) place(FX.tick(), at('speed', b), 0.3);

// One gain for the whole mix: the loudest sample at -1 dBFS, so nothing clips and nothing is compressed.
let peak = 0;
for (let i = 0; i < LENGTH; i++) peak = Math.max(peak, Math.abs(L[i]), Math.abs(R[i]));
const gain = 10 ** (-1 / 20) / peak;

const data = Buffer.alloc(LENGTH * 4);
for (let i = 0; i < LENGTH; i++) {
  data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, L[i] * gain)) * 32767), i * 4);
  data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, R[i] * gain)) * 32767), i * 4 + 2);
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
mkdirSync(join(media, 'public', 'sound'), {recursive: true});
const out = join(media, 'public', 'sound', 'soundtrack.wav');
writeFileSync(out, Buffer.concat([header, data]));
console.log(`wrote ${out}: ${(LENGTH / RATE).toFixed(2)} s, mix gain ${(20 * Math.log10(gain)).toFixed(1)} dB`);
