// Makes the video's whole soundtrack and writes public/sound/soundtrack.wav (48 kHz, stereo, 16-bit, the video's exact
// length). Every sound sits on the frames of src/timeline.ts (`SOUNDS`, `MUSIC`).
//
// - Wakeshot's own chimes: the numbers are read from crates/wakeshot-windows/src/sound/chime.rs and rendered the way
//   its `render` does, so the video plays what the app plays. If that file changes shape, this script stops.
// - The effects and the music bed are made here from sine waves and seeded noise: no outside file, so no licence
//   question and the same bytes on every run. The bed is a 120 BPM sequencer (kick, hats, bass, pluck, pad) in C
//   major, because the chimes are C6 and G6.
// - The keys of the opening are real keystrokes (`public/keys/`, CC0, `CREDITS.md`), one take per key, placed on
//   the frames the letters appear; the room, the laptop's fan and its power going down are made here.
// - The music: the track named in `data/music.json`, fitted to the video on every build
//   (`lib/fit-music.mjs`); the code bed when that file names none. `--music <file.wav>` puts a track already fitted
//   by `scripts/music-candidate.mjs` in place of both; `--out <file.wav>` writes somewhere else.
// Before the light there is no music (Part E): only the keys, the room and the laptop going to sleep. Under each
// chime the music gives way only in the chime's own band (a 6 dB bell cut around 1.5 kHz), so its level does not dip;
// the mix is set to -15 LUFS integrated with its peaks held under -2.5 dBFS.
import {mkdirSync, readFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {RATE, fade, limit, loudness, peakOf, readWav, rng, shortTerm, svf, writeWav} from './lib/audio.mjs';
import {decode, fitTrack} from './lib/fit-music.mjs';
import {BAR, BEAT, FPS} from '../src/beat.ts';
import {END_CARD, MUSIC, SOUNDS, T, VIDEO} from '../src/timeline.ts';

const here = dirname(fileURLToPath(import.meta.url));
const media = join(here, '..');
const LENGTH = Math.round((VIDEO.frames / VIDEO.fps) * RATE);
const TOTAL = LENGTH / RATE;
const secondsOf = (frame) => frame / VIDEO.fps;
const BEAT_SECONDS = BEAT / FPS;
const BAR_SECONDS = BAR / FPS;
const sampleOf = (frame) => Math.round(secondsOf(frame) * RATE);
const arg = (name) => {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : null;
};

/** Target loudness of the music stem alone, so the code bed and a found track sit at the same level under the rest. */
const MUSIC_LUFS = -20;
const MIX_LUFS = -15;
// Sample peaks at -2.5 dBFS keep the true peak (between samples, after AAC) under -1 dBTP with room to spare: at -1.5
// it read -0.9, and at -2 the Part C cut read -1.01, so one more agent's tick could cross it.
const CEILING = 10 ** (-2.5 / 20);
/**
 * Under a chime the music is cut by `db` in the chime's band only (C6 and G6 and their overtones, 1 to 3 kHz), the
 * way a mixing engineer makes room for a voice: the chime reads clearly and the music keeps its level. The old 4 dB
 * cut of the whole music under each chime was heard as a dip.
 */
const CHIME_ROOM = {db: -6, hz: 1500, q: 0.8};
/**
 * The music's short-term loudness (3 s) may sit at most this far from its median, under or over, from the light, where
 * it starts, to the end card (no dips a listener can hear; a loud stretch makes the rest sound like a dip). The
 * build stops past it.
 */
const MOST_DIP_LU = 2;

// ---- Wakeshot's chimes, from chime.rs ----

function readChimes() {
  const src = readFileSync(join(media, '..', 'crates', 'wakeshot-windows', 'src', 'sound', 'chime.rs'), 'utf8');
  const num = (text, key) => {
    const m = text.match(new RegExp(`${key}:\\s*(-?[0-9.]+)`));
    if (!m) throw new Error(`chime.rs: no ${key}; the file changed shape, update make-sounds.mjs`);
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
  return {lidChime: shape('LID_CLOSED_HOLDING'), sleepChime: shape('GOING_TO_SLEEP')};
}

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

// ---- Sound sources ----

/** Noise through a one-pole low-pass and high-pass, shaped by `env(t)`; `lowAt` may sweep the low-pass. */
function noise(seconds, seed, lowHz, highHz, env, lowAt = null) {
  const r = rng(seed);
  const n = Math.round(seconds * RATE);
  const out = new Float64Array(n);
  const hp = Math.exp((-2 * Math.PI * highHz) / RATE);
  let l = 0;
  let h = 0;
  let prev = 0;
  for (let i = 0; i < n; i++) {
    const t = i / RATE;
    const lp = 1 - Math.exp((-2 * Math.PI * (lowAt ? lowAt(t) : lowHz)) / RATE);
    l += lp * (r() * 2 - 1 - l);
    h = hp * (h + l - prev);
    prev = l;
    out[i] = h * env(t);
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

const add = (a, b) => {
  const out = new Float64Array(Math.max(a.length, b.length));
  for (let i = 0; i < out.length; i++) out[i] = (a[i] ?? 0) + (b[i] ?? 0);
  return out;
};

/**
 * Each effect and where its loudest moment sits from its start, in seconds: a whoosh is placed so its peak lands on
 * the camera move's fastest frame, its first.
 */
const FX = {
  enter: [() => add(noise(0.06, 7, 3500, 300, (t) => 1.3 * Math.exp(-t * 80)), tone(0.05, () => 900, (t) => 0.2 * Math.exp(-t * 90))), 0],
  tick: [() => tone(0.06, () => 1760, (t) => 0.45 * fade(t / 0.002) * Math.exp(-t * 70), [1, 0.15]), 0],
  click: [() => add(noise(0.03, 17, 5000, 1200, (t) => Math.exp(-t * 220)), tone(0.03, () => 1300, (t) => 0.25 * Math.exp(-t * 150))), 0],
  // A soft rounded blip that rises a little: things popping in.
  pop: [() => tone(0.09, (t) => 520 + 380 * Math.min(1, t / 0.03), (t) => 0.5 * fade(t / 0.004) * Math.exp(-t * 45), [1, 0.2]), 0],
  // Filtered noise whose brightness follows its loudness: 0.45 s, the peak 0.12 s in.
  whoosh: [
    () =>
      noise(
        0.45,
        31,
        0,
        250,
        (t) => 0.7 * fade(t / 0.12) * fade((0.45 - t) / 0.33),
        (t) => 600 + 4200 * fade(t / 0.12) * fade((0.45 - t) / 0.33),
      ),
    0.12,
  ],
  whip: [
    () =>
      noise(
        0.32,
        37,
        0,
        400,
        (t) => 0.9 * fade(t / 0.06) * fade((0.32 - t) / 0.26),
        (t) => 900 + 6500 * fade(t / 0.06) * fade((0.32 - t) / 0.26),
      ),
    0.06,
  ],
  thud: [
    () =>
      add(
        tone(0.25, (t) => 110 - 40 * (t / 0.25), (t) => 0.9 * fade(t / 0.003) * Math.exp(-t * 22), [1, 0.4]),
        noise(0.12, 19, 900, 60, (t) => 0.8 * Math.exp(-t * 45)),
      ),
    0,
  ],
  lock: [
    () => {
      const one = (seed) => add(noise(0.025, seed, 7000, 2000, (t) => Math.exp(-t * 260)), tone(0.025, () => 3200, (t) => 0.2 * Math.exp(-t * 240)));
      const out = new Float64Array(Math.round(0.12 * RATE));
      out.set(one(23), 0);
      one(29).forEach((v, i) => (out[Math.round(0.07 * RATE) + i] += v));
      return out;
    },
    0,
  ],
  // A glide up and a glide down: the light coming on and going off.
  lightOn: [
    () =>
      add(
        tone(0.7, (t) => 330 + 330 * fade(t / 0.7), (t) => 0.35 * fade(t / 0.15) * fade((0.7 - t) / 0.3), [1, 0.3, 0.12]),
        noise(0.7, 11, 2500, 400, (t) => 0.5 * fade(t / 0.5) * fade((0.7 - t) / 0.2)),
      ),
    0,
  ],
  lightOff: [
    () =>
      add(
        tone(0.7, (t) => 660 - 330 * fade(t / 0.7), (t) => 0.3 * fade(t / 0.08) * fade((0.7 - t) / 0.35), [1, 0.3, 0.12]),
        noise(0.7, 13, 2000, 300, (t) => 0.4 * fade(t / 0.1) * fade((0.7 - t) / 0.5)),
      ),
    0,
  ],
  // A low hit under the light coming on: a falling sine and a breath of noise.
  hit: [
    () =>
      add(
        tone(0.9, (t) => 48 + 70 * Math.exp(-t * 30), (t) => fade(t / 0.004) * Math.exp(-t * 4.5), [1, 0.25]),
        noise(0.4, 41, 1800, 80, (t) => 0.35 * Math.exp(-t * 14)),
      ),
    0,
  ],
  // Air that swells as the end card's light comes on: low-passed noise, slow in, long out, no pitch, so it sits under
  // any music.
  glow: [
    () =>
      noise(
        1.8,
        43,
        0,
        120,
        (t) => 0.5 * fade(t / 0.35) * Math.exp(-Math.max(0, t - 0.35) * 2.2),
        (t) => 700 + 900 * fade(t / 0.35) * Math.exp(-Math.max(0, t - 0.35) * 1.5),
      ),
    0.1,
  ],
  // The soft landing of a number.
  settle: [() => tone(0.3, () => 392, (t) => 0.35 * fade(t / 0.003) * Math.exp(-t * 16), [1, 0.35, 0.1]), 0],
  // The room the laptop stands in: a soft, dark air under the opening, so the keys do not sound pasted on silence. It
  // goes under the music in the light's first beat.
  room: [
    () => {
      const len = secondsOf(MUSIC.full) + 0.5;
      return noise(len, 47, 1100, 70, (t) => 0.02 * fade(t / 0.05) * fade((len - t) / 0.5));
    },
    0,
  ],
  // The laptop's fan: a low airy band with a faint hum in it, until the laptop sleeps; then it winds down over 1.6 s,
  // the hum falling with it. Part of the laptop's sound of going to sleep, with `powerDown`.
  fan: [
    () => {
      const sleep = secondsOf(T.hook.sleep);
      const len = sleep + 1.8;
      const spin = (t) => (t < sleep ? 1 : Math.max(0, 1 - fade((t - sleep) / 1.6)));
      const air = noise(len, 53, 0, 160, (t) => 0.025 * fade(t / 0.05) * spin(t), (t) => 300 + 900 * spin(t));
      const hum = tone(len, (t) => 118 + 2 * Math.sin(2 * Math.PI * 0.3 * t) - 70 * (1 - spin(t)), (t) => 0.006 * fade(t / 0.05) * spin(t), [1, 0.5]);
      return add(air, hum);
    },
    0,
  ],
  // The screen going off and the laptop going to sleep: a soft tick (the backlight), a low glide falling two octaves
  // and a breath of air falling with it. Made here; Windows itself plays nothing when it sleeps.
  powerDown: [
    () =>
      add(
        add(
          noise(0.03, 59, 6000, 1500, (t) => 0.5 * Math.exp(-t * 260)),
          tone(1.3, (t) => 220 * 2 ** (-2.2 * fade(t / 1.1)), (t) => 0.45 * fade(t / 0.012) * Math.exp(-t * 2.4), [1, 0.25, 0.08]),
        ),
        noise(1.1, 61, 0, 60, (t) => 0.3 * fade(t / 0.02) * Math.exp(-t * 3), (t) => 2400 * 2 ** (-3.5 * fade(t / 1.0)) + 150),
      ),
    0,
  ],
  // Air that rises over the beat before the light and stops on it, where the music and the low hit come in.
  riser: [
    () =>
      noise(
        0.5,
        67,
        0,
        200,
        (t) => 0.8 * (t / 0.5) ** 2 * fade((0.5 - t) / 0.006),
        (t) => 400 + 9000 * (t / 0.5) ** 2,
      ),
    0.5,
  ],
};

// ---- The keys: real keystrokes, CC0 ----

/**
 * Takes of single keystrokes from "Keyboard Soundpack #1" by unicaegames (OpenGameArt, CC0; a Cherry KC 1000, a
 * low-profile office keyboard), each file unchanged in `public/keys/`. Picked by numbers, not by ear: the
 * letters are mid-level takes with their key-up after them; the space bar the one deep take (a fifth of it under
 * 300 Hz); Enter a firm take with more low end; shift a soft one with no key-up (it is held).
 */
const KEY_TAKES = {
  key: [2, 3, 5, 7, 8, 11, 12, 14, 15, 24, 25, 26, 27, 29, 32],
  keySpace: [20],
  keyReturn: [28],
  keyShift: [16],
};
/** Each take's peak is set to this, then each key varies by up to 1.5 dB either way, as real strokes do. */
const KEY_PEAK = 0.22;

const takeFile = (n) => join(media, 'public', 'keys', `keypress-${String(n).padStart(3, '0')}.wav`);
const takes = new Map();
/**
 * One keystroke: its take decoded to 48 kHz, cut from 3 ms before the stroke (so the cue's frame is the stroke) with a
 * 1 ms fade in, its top softened a little (heard from the seat, not from the close microphone), its peak set.
 */
function keystroke(sound, variant) {
  const list = KEY_TAKES[sound];
  const n = list[(variant * 7) % list.length];
  if (!takes.has(n)) {
    const [x] = decode(takeFile(n), null);
    let peak = 0;
    for (const v of x) peak = Math.max(peak, Math.abs(v));
    const on = Math.max(0, x.findIndex((v) => Math.abs(v) > 0.1 * peak) - Math.round(0.003 * RATE));
    const cut = x.subarray(on);
    const {low} = svf(cut, () => 6500, 0.7);
    const soft = cut.map((v, i) => (0.5 * v + 0.5 * low[i]) * fade(i / (0.001 * RATE)) * fade((cut.length - i) / (0.02 * RATE)));
    let top = 0;
    for (const v of soft) top = Math.max(top, Math.abs(v));
    takes.set(n, soft.map((v) => (v * KEY_PEAK) / top));
  }
  const vary = 10 ** ((1.5 * (2 * rng(1000 + variant)() - 1)) / 20);
  return takes.get(n).map((v) => v * vary);
}

// ---- The 120 BPM sequencer: the music bed ----

const midi = (m) => 440 * 2 ** ((m - 69) / 12);
const CHORD = {
  C: {notes: [60, 64, 67, 71, 74], bass: 36},
  Am: {notes: [57, 60, 64, 67, 71], bass: 45},
  F: {notes: [53, 57, 60, 64, 67], bass: 41},
  G: {notes: [55, 59, 62, 64, 69], bass: 43},
};
/** One chord per bar of 2 s: the hook, the light to the tray, the lid, the settings and the sleep, the menu, the numbers. */
const BARS = ['C', 'C', 'C', 'Am', 'F', 'F', 'Am', 'G', 'F', 'C', 'Am', 'F', 'G', 'C', 'C'];
const chordAt = (t) => CHORD[BARS[Math.min(BARS.length - 1, Math.floor(t / BAR_SECONDS))]];

/** How much of each part plays at second `t`: the storyboard's shape (`MUSIC` in timeline.ts); nothing before the light. */
function partsAt(t) {
  const s = secondsOf;
  if (t < s(MUSIC.full)) return {kick: 0, hat: 0, bass: 0, pluck: 0, pad: 0, open: 0};
  if (t < s(MUSIC.thin[0])) return {kick: 1, hat: 1, bass: 1, pluck: 1, pad: 1, open: 0};
  if (t < s(MUSIC.thin[1])) return {kick: 0, hat: 0, bass: 0.7, pluck: 0.5, pad: 1, open: 0};
  if (t < s(MUSIC.lift)) return {kick: 0.8, hat: 0.7, bass: 1, pluck: 1, pad: 1, open: 0};
  if (t < s(MUSIC.resolve)) return {kick: 1, hat: 1, bass: 1, pluck: 1.15, pad: 1, open: 1};
  return {kick: 0, hat: 0, bass: 0, pluck: 0, pad: 0, open: 0};
}

function codeBed() {
  const L = new Float64Array(LENGTH);
  const R = new Float64Array(LENGTH);
  const put = (samples, at, gain, pan = 0.5) => {
    const first = Math.round(at * RATE);
    for (let i = 0; i < samples.length && first + i < LENGTH; i++) {
      if (first + i < 0) continue;
      L[first + i] += samples[i] * gain * Math.sqrt(1 - pan);
      R[first + i] += samples[i] * gain * Math.sqrt(pan);
    }
  };
  const kick = tone(0.35, (t) => 45 + 75 * Math.exp(-t * 28), (t) => fade(t / 0.002) * Math.exp(-t * 9), [1, 0.15]);
  const hat = (seed, open) => noise(open ? 0.14 : 0.04, seed, 14000, 6000, (t) => fade(t / 0.001) * Math.exp(-t * (open ? 28 : 110)));
  const full = secondsOf(MUSIC.full);
  const resolve = secondsOf(MUSIC.resolve);
  const eighth = BEAT_SECONDS / 2;
  // Drums, bass and plucks on the eighth-note grid.
  for (let k = 0; k * eighth < resolve; k++) {
    const t = k * eighth;
    if (t < full) continue;
    const p = partsAt(t);
    const c = chordAt(t);
    const onBeat = k % 2 === 0;
    if (onBeat && p.kick) put(kick, t, 0.55 * p.kick);
    if (p.hat) put(hat(1000 + k, false), t, (onBeat ? 0.05 : 0.09) * p.hat, 0.62);
    if (!onBeat && p.open) put(hat(3000 + k, true), t, 0.05 * p.open, 0.38);
    if (p.bass) {
      const step = [0, 0, 12, 0, 0, 7, 12, 0][k % 8];
      const len = p.kick ? 0.22 : 0.48;
      if (p.kick || onBeat) {
        put(tone(len, () => midi(c.bass + step), (u) => fade(u / 0.006) * fade((len - u) / 0.05) * Math.exp(-u * 3), [1, 0.35, 0.1]), t, 0.32 * p.bass);
      }
    }
    if (p.pluck && (p.kick || onBeat)) {
      const order = [0, 2, 4, 2, 1, 3, 4, 3];
      const note = c.notes[order[k % 8]] + 12 + (p.open && k % 4 === 3 ? 12 : 0);
      put(tone(0.9, () => midi(note), (u) => fade(u / 0.004) * Math.exp(-u * 5.5), [1, 0.15]), t, 0.11 * p.pluck, k % 2 === 0 ? 0.35 : 0.65);
    }
  }
  // A ping-pong echo of a dotted eighth gives the plucks room without a reverb.
  const dl = Math.round(eighth * 1.5 * RATE);
  for (let i = dl; i < LENGTH; i++) {
    L[i] += 0.22 * R[i - dl];
    R[i] += 0.22 * L[i - dl];
  }
  // The pad: two detuned voices per note, a slow swell per chord, from the light on.
  // Bars of one chord in a row are one held chord; the resolve starts a chord of its own.
  const held = [];
  for (let b = 0; b * BAR_SECONDS < TOTAL; b++) {
    const start = b * BAR_SECONDS;
    const name = BARS[Math.min(b, BARS.length - 1)];
    const prev = held[held.length - 1];
    if (prev && prev.name === name && (start < resolve || prev.start >= resolve)) prev.span += BAR_SECONDS;
    else held.push({name, start, span: BAR_SECONDS});
  }
  for (const {name, start, span: whole} of held) {
    const c = CHORD[name];
    const last = start >= resolve;
    const span = Math.min(whole, TOTAL - start);
    for (const m of [...c.notes, c.bass + 12]) {
      for (const [cents, pan] of [
        [-6, 0.3],
        [6, 0.7],
      ]) {
        const n = Math.round((span + 1.0) * RATE);
        const first = Math.round(start * RATE);
        let phase = 0;
        for (let i = 0; i < n && first + i < LENGTH; i++) {
          const t = start + i / RATE;
          const u = i / RATE;
          if (t < full) continue;
          const hz = midi(m) * 2 ** (cents / 1200);
          phase += (2 * Math.PI * hz) / RATE;
          const level = partsAt(t).pad || (last ? 1 : 0);
          // The last chord rings out over the end card and is gone by the end.
          const ring = last ? Math.exp(-u * 0.9) * fade((TOTAL - t) / 0.6) : 1;
          const env = fade(u / (last ? 0.02 : 0.6)) * (u < span ? 1 : fade(1 - (u - span) / 1.0)) * ring;
          const s = (Math.sin(phase) + 0.2 * Math.sin(2 * phase)) * env * level * 0.045;
          L[first + i] += s * Math.sqrt(1 - pan);
          R[first + i] += s * Math.sqrt(pan);
        }
      }
    }
  }
  // The resolve: one slow, soft strum of the chord's notes, no drum, so the end rings out calm.
  CHORD.C.notes.forEach((m, i) => put(tone(3.5, () => midi(m + 12), (u) => fade(u / 0.03) * Math.exp(-u * 1.4), [1, 0.1]), resolve + i * 0.06, 0.07, 0.3 + i * 0.1));
  return [L, R];
}

/** A found track (already conformed and cut) as the music stem, at the video's length. */
function foundTrack(path) {
  const [l, r] = readWav(path);
  const L = new Float64Array(LENGTH);
  const R = new Float64Array(LENGTH);
  L.set(l.subarray(0, LENGTH));
  R.set(r.subarray(0, LENGTH));
  return [L, R];
}

// ---- Mix ----

const musicPath = arg('music');
const picked = JSON.parse(readFileSync(join(media, 'data', 'music.json'), 'utf8'));
let musicName = 'code bed';
let mL;
let mR;
if (musicPath) {
  [mL, mR] = foundTrack(resolve(musicPath));
  musicName = musicPath;
} else if (picked.file) {
  const fitted = fitTrack(join(media, 'public', picked.file), {bpm: picked.bpm, lift: picked.lift});
  [mL, mR] = [fitted.L, fitted.R];
  musicName = `${picked.file} at ${picked.bpm} BPM, lift ${picked.lift} s`;
} else [mL, mR] = codeBed();
const musicGain = 10 ** ((MUSIC_LUFS - loudness(mL, mR)) / 20);

const chimes = readChimes();
const rendered = {lidChime: renderChime(chimes.lidChime), sleepChime: renderChime(chimes.sleepChime)};
const fxL = new Float64Array(LENGTH);
const fxR = new Float64Array(LENGTH);
// How deep the chime band is cut at each sample, 0 to 1 of `CHIME_ROOM.db`.
const room = new Float64Array(LENGTH);
const made = {};
for (const cue of SOUNDS) {
  const chime = cue.sound === 'lidChime' || cue.sound === 'sleepChime';
  let samples;
  let lead = 0;
  if (chime) samples = rendered[cue.sound];
  else if (cue.sound in KEY_TAKES) samples = keystroke(cue.sound, cue.variant ?? 0);
  else {
    made[cue.sound] ??= FX[cue.sound][0]();
    samples = made[cue.sound];
    lead = FX[cue.sound][1];
  }
  const first = sampleOf(cue.at) - Math.round(lead * RATE);
  const gain = cue.gain ?? 1;
  const pan = cue.pan ?? 0.5;
  for (let i = 0; i < samples.length && first + i < LENGTH; i++) {
    if (first + i < 0) continue;
    fxL[first + i] += samples[i] * gain * Math.sqrt(1 - pan) * Math.SQRT2;
    fxR[first + i] += samples[i] * gain * Math.sqrt(pan) * Math.SQRT2;
  }
  if (chime) {
    // The band is cut while the chime sounds, easing in over 0.1 s and out over 0.3 s.
    const len = samples.length / RATE;
    for (let i = Math.max(0, first - 0.1 * RATE); i < Math.min(LENGTH, first + (len + 0.3) * RATE); i++) {
      const t = (i - first) / RATE;
      room[i] = Math.max(room[i], t < 0 ? fade(1 + t / 0.1) : t < len ? 1 : fade(1 - (t - len) / 0.3));
    }
  }
}

// The music with room made for the chimes: a bell cut, the signal plus (G - 1) times its unity-peak band.
const cut = 10 ** (CHIME_ROOM.db / 20) - 1;
const music = [mL, mR].map((x) => {
  const {band} = svf(x, () => CHIME_ROOM.hz, CHIME_ROOM.q);
  return x.map((v, i) => (v + cut * room[i] * band[i]) * musicGain);
});

// Evenness: the music alone, moment to moment, from the light, where it starts, to the end card: only windows wholly
// inside that stretch. Found tracks only: the code bed is the fallback.
const st = shortTerm(music[0], music[1]).filter((w) => w.t - 1.5 >= MUSIC.full / VIDEO.fps && w.t + 1.5 <= END_CARD / VIDEO.fps);
if (process.env.SOUND_DEBUG) console.log(st.map((w) => `${w.t.toFixed(1)}:${w.lufs.toFixed(1)}`).join(' '));
const sorted = st.map((w) => w.lufs).sort((a, b) => a - b);
const median = sorted[Math.floor(sorted.length / 2)];
const lowest = st.reduce((m, w) => (w.lufs < m.lufs ? w : m), {t: 0, lufs: Infinity});
const highest = st.reduce((m, w) => (w.lufs > m.lufs ? w : m), {t: 0, lufs: -Infinity});
const even =
  `music ${median.toFixed(1)} LUFS (3 s median) from the light; ${(lowest.lufs - median).toFixed(1)} LU at ${lowest.t.toFixed(1)} s ` +
  `to +${(highest.lufs - median).toFixed(1)} LU at ${highest.t.toFixed(1)} s`;
const uneven = median - lowest.lufs > MOST_DIP_LU || highest.lufs - median > MOST_DIP_LU;
if (musicName !== 'code bed' && uneven) {
  console.error(`the music is uneven: ${even}; at most ${MOST_DIP_LU} LU either way`);
  if (!process.env.SOUND_ALLOW_DIP) process.exit(1);
}

const L = new Float64Array(LENGTH);
const R = new Float64Array(LENGTH);
for (let i = 0; i < LENGTH; i++) {
  L[i] = music[0][i] + fxL[i];
  R[i] = music[1][i] + fxR[i];
}
const before = loudness(L, R);
const gain = 10 ** ((MIX_LUFS - before) / 20);
for (let i = 0; i < LENGTH; i++) {
  L[i] *= gain;
  R[i] *= gain;
}
if (process.env.SOUND_DEBUG) {
  const hot = new Map();
  for (let i = 0; i < LENGTH; i++) {
    const p = Math.max(Math.abs(L[i]), Math.abs(R[i]));
    if (p > CEILING) hot.set(Math.floor((i / RATE) * 10) / 10, Math.max(hot.get(Math.floor((i / RATE) * 10) / 10) ?? 0, 20 * Math.log10(p / CEILING)));
  }
  console.log([...hot].map(([t, db]) => `${t}s +${db.toFixed(1)}`).join(', '));
}
const pulled = limit(L, R, CEILING);
const out = arg('out') ? resolve(arg('out')) : join(media, 'public', 'sound', 'soundtrack.wav');
mkdirSync(dirname(out), {recursive: true});
writeWav(out, L, R);
console.log(
  `wrote ${out}: ${TOTAL.toFixed(2)} s, music ${musicName}, ` +
    `${loudness(L, R).toFixed(1)} LUFS, peak ${(20 * Math.log10(peakOf(L, R))).toFixed(1)} dBFS, limiter ${pulled.toFixed(1)} dB; ${even}`,
);
