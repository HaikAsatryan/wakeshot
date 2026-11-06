// Fits a found music track to the video: finds its tempo and the moment it lifts out of its intro, conforms it to
// 120 BPM with ffmpeg's `atempo` (Remotion's own ffmpeg), and cuts the video's length on the storyboard: the music
// starts on the light coming on, on the track's lift (Part E of `p6-agents-video-readme`, Haik: "we start music when
// starting to introducing our app"), and plays at one level from there to the end card (Part D: Haik heard the old
// shape, soft under the hook and dipped under the lid chime, as the music going "more silent at some points"); on the
// end card it fades into a soft echo that dies before the video ends. Used by `make-sounds.mjs` for the picked track (`data/music.json`) and by `music-candidate.mjs` to
// try a new one.
import {spawnSync} from 'node:child_process';
import {mkdtempSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {RATE, fade, readWav, shortTerm} from './audio.mjs';
import {BEAT, FPS} from '../../src/beat.ts';
import {MUSIC, VIDEO} from '../../src/timeline.ts';

const media = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const ffmpeg = join(media, 'node_modules', '@remotion', 'compositor-win32-x64-msvc', 'ffmpeg.exe');
export const TARGET_BPM = (60 * FPS) / BEAT;

/**
 * Decodes with ffmpeg to 48 kHz stereo, after an optional filter chain. Through a WAV file: Remotion's ffmpeg has no
 * raw float output.
 */
export function decode(file, filter, loops = 0) {
  const dir = mkdtempSync(join(tmpdir(), 'wakeshot-music-'));
  try {
    const wav = join(dir, 'decoded.wav');
    const input = [...(loops ? ['-stream_loop', String(loops)] : []), '-i', file];
    const r = spawnSync(ffmpeg, ['-v', 'error', ...input, ...(filter ? ['-af', filter] : []), '-ar', String(RATE), '-ac', '2', '-c:a', 'pcm_s16le', wav]);
    if (r.status !== 0) throw new Error(`ffmpeg failed on ${file}: ${r.stderr}`);
    return readWav(wav);
  } finally {
    rmSync(dir, {recursive: true, force: true});
  }
}

const HOP = 512;
/** How sharply the sound rises in each hop (log energy of the bright part, rises only): where beats fall. */
function onsets(L, R) {
  const n = Math.floor(L.length / HOP);
  const e = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    let sum = 0;
    for (let i = k * HOP + 1; i < (k + 1) * HOP; i++) {
      const d = (L[i] + R[i] - L[i - 1] - R[i - 1]) / 2;
      sum += d * d;
    }
    e[k] = Math.log(1e-9 + sum);
  }
  const o = new Float64Array(n);
  for (let k = 1; k < n; k++) o[k] = Math.max(0, e[k] - e[k - 1]);
  return o;
}

/** The tempo by autocorrelation of the onsets, between 90 and 160 BPM. */
function tempo(o) {
  const hopS = HOP / RATE;
  let best = {bpm: 0, score: -Infinity};
  for (let bpm = 90; bpm <= 160; bpm += 0.25) {
    const lag = 60 / bpm / hopS;
    let score = 0;
    for (let k = 0; k + 4 * lag + 1 < o.length; k++) {
      const at = (m) => {
        const x = k + m * lag;
        const i = Math.floor(x);
        return o[i] + (o[i + 1] - o[i]) * (x - i);
      };
      score += o[k] * (at(1) + 0.5 * at(2) + 0.25 * at(4));
    }
    score /= o.length;
    if (score > best.score) best = {bpm, score};
  }
  return best.bpm;
}

/** Loudness (RMS in dB) of each `win` seconds. */
function levels(L, R, win) {
  const size = Math.round(win * RATE);
  const out = [];
  for (let s = 0; s + size <= L.length; s += size) {
    let sum = 0;
    for (let i = s; i < s + size; i++) sum += L[i] * L[i] + R[i] * R[i];
    out.push(10 * Math.log10(1e-12 + sum / (2 * size)));
  }
  return out;
}

/** The gain of the music at second `t` of the video: silent before the light, one level from it, the end card's fade. */
export function shapeAt(t) {
  const full = MUSIC.full / FPS;
  const end = MUSIC.resolve / FPS;
  const total = VIDEO.frames / VIDEO.fps;
  // A 10 ms fade in on the light: the track comes in on its lift, a downbeat, so it lands as a drop, without a click.
  let g = fade((t - full) / 0.01);
  // The end card: the track plays on for a beat, then fades over 2.5 s; the echo (`ending`) carries it into silence.
  const fadeFrom = end + 0.5;
  if (t >= fadeFrom) g *= Math.cos(Math.min(1, (t - fadeFrom) / 2.5) * (Math.PI / 2));
  if (t >= total) g = 0;
  return g;
}

/**
 * Evens the track out, the way a broadcast leveler does: where its 3 s loudness sits above or below its own median
 * (from the light to the end card) it is turned down or up, by at most `most` dB, with a gain that moves slowly. A
 * track can have softer or louder bars in those 26 s, and either reads as a dip somewhere. Only windows wholly inside
 * the music count: one that reaches back into the silence before the light would read low and push the track's first
 * bars up. The first bars keep the first window's gain, the end card the gain it reached.
 */
const LEVEL = {most: 6, hop: 0.25};

function level(L, R) {
  const full = MUSIC.full / FPS;
  const end = MUSIC.resolve / FPS;
  const windows = shortTerm(L, R, 3, LEVEL.hop).filter((w) => w.t - 1.5 >= full && w.t <= end);
  const sorted = windows.map((w) => w.lufs).sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  const gainDb = windows.map((w) => Math.max(-LEVEL.most, Math.min(LEVEL.most, median - w.lufs)));
  const at = (t) => {
    // Between window centres the gain moves along a raised cosine; before the first and after the last it holds.
    const x = (t - windows[0].t) / LEVEL.hop;
    if (x <= 0) return gainDb[0];
    if (x >= gainDb.length - 1) return gainDb[gainDb.length - 1];
    const k = Math.floor(x);
    return gainDb[k] + (gainDb[k + 1] - gainDb[k]) * fade(x - k);
  };
  for (let i = 0; i < L.length; i++) {
    const g = 10 ** (at(i / RATE) / 20);
    L[i] *= g;
    R[i] *= g;
  }
}

/**
 * The end of the track: from the end card on, a soft ping-pong echo of a dotted eighth, darker on each repeat, so the
 * fading music trails off into silence instead of stopping; everything is gone by 0.1 s before the video ends.
 */
function ending(L, R) {
  const end = MUSIC.resolve / FPS;
  const total = VIDEO.frames / VIDEO.fps;
  const delay = Math.round(0.75 * (BEAT / FPS) * RATE);
  const lp = 1 - Math.exp((-2 * Math.PI * 2200) / RATE);
  const n = L.length;
  const dl = new Float64Array(n);
  const dr = new Float64Array(n);
  let fl = 0;
  let fr = 0;
  for (let i = Math.round(end * RATE); i < n; i++) {
    const inL = L[i] + (i >= delay ? 0.45 * dr[i - delay] : 0);
    const inR = R[i] + (i >= delay ? 0.45 * dl[i - delay] : 0);
    fl += lp * (inL - fl);
    fr += lp * (inR - fr);
    dl[i] = fl;
    dr[i] = fr;
  }
  for (let i = Math.round(end * RATE); i < n; i++) {
    const t = i / RATE;
    const out = fade((total - 0.1 - t) / 0.6);
    L[i] = (L[i] + 0.35 * (i >= delay ? dl[i - delay] : 0)) * out;
    R[i] = (R[i] + 0.35 * (i >= delay ? dr[i - delay] : 0)) * out;
  }
}

/**
 * The track fitted to the video, as stereo 48 kHz samples of the video's length. `bpm` and `lift` (seconds into the
 * original track) are found when not given; `info` says what was used, so a pick is rebuilt with the same numbers.
 */
export function fitTrack(input, {bpm: givenBpm = null, lift: givenLift = null} = {}) {
  const bpm = givenBpm ?? tempo(onsets(...decode(input, null)));
  const ratio = TARGET_BPM / bpm;
  // Played three times over: a loopable track then never runs out before the video ends (a picked lift late in a short
  // track left the end card in silence).
  const [L, R] = decode(input, `atempo=${ratio.toFixed(6)}`, 2);

  // The beat's phase in the conformed track: the offset in one beat where the onsets line up best.
  const o = onsets(L, R);
  const beatS = BEAT / FPS;
  const hopS = HOP / RATE;
  let phase = 0;
  let bestPhase = -Infinity;
  for (let p = 0; p < beatS; p += hopS) {
    let s = 0;
    for (let t = p; t / hopS < o.length; t += beatS) s += o[Math.round(t / hopS)];
    if (s > bestPhase) [bestPhase, phase] = [s, p];
  }

  // The lift: the biggest rise in loudness from one 2 s window to the next, in the first minute, on a beat.
  const lv = levels(L, R, 2);
  let lift;
  if (givenLift !== null) lift = givenLift / ratio;
  else {
    let rise = -Infinity;
    for (let k = 1; k < Math.min(lv.length, 30); k++) {
      if (lv[k] - lv[k - 1] > rise) [rise, lift] = [lv[k] - lv[k - 1], 2 * k];
    }
  }
  lift = phase + Math.round((lift - phase) / beatS) * beatS;
  // Where the video's first frame would sit in the track; before the light it is silent, so this may be negative.
  const start = lift - MUSIC.full / FPS;

  const n = Math.round((VIDEO.frames / VIDEO.fps) * RATE);
  const outL = new Float64Array(n);
  const outR = new Float64Array(n);
  const first = Math.round(start * RATE);
  for (let i = 0; i < n; i++) {
    const g = shapeAt(i / RATE);
    outL[i] = (L[first + i] ?? 0) * g;
    outR[i] = (R[first + i] ?? 0) * g;
  }
  level(outL, outR);
  ending(outL, outR);
  return {
    L: outL,
    R: outR,
    info: {bpm, ratio, lift: lift * ratio, start: start * ratio, levels: lv.slice(0, 16)},
  };
}
