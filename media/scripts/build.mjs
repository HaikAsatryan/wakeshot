// Builds the media files. One mode per command (media/README.md):
//   node scripts/build.mjs loop     out/icon-loop-light.gif, out/icon-loop-dark.gif (frames from Remotion, the GIF
//                                   from scripts/lib/gif.mjs so its colours stay exact)
//   node scripts/build.mjs draft    out/draft/wakeshot-promo-draft.mp4 and its poster (stand-ins allowed, marked)
//   node scripts/build.mjs video    out/wakeshot-promo.mp4 and out/wakeshot-promo-poster.png (refuses stand-ins)
//   node scripts/build.mjs poster   out/wakeshot-promo-poster.png only (refuses stand-ins)
// MEDIA_CONCURRENCY sets how many frames render at once (default 4, so other work on the laptop keeps its cores).
import {spawnSync} from 'node:child_process';
import {existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {LOOP} from '../src/timeline.ts';
import {writeGif} from './lib/gif.mjs';
import {readPng} from './lib/png.mjs';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const cli = join(media, 'node_modules', '@remotion', 'cli', 'remotion-cli.js');
const concurrency = process.env.MEDIA_CONCURRENCY ?? '4';
/** The poster: the light on and the name with its line in, the frame the README player shows before play. */
const POSTER_FRAME = 120;

function run(args) {
  const r = spawnSync(process.execPath, args, {cwd: media, stdio: 'inherit'});
  if (r.status !== 0) process.exit(r.status ?? 1);
}

const remotion = (...args) => run([cli, ...args]);
const sounds = () => run([join(media, 'scripts', 'make-sounds.mjs')]);
const json = (p) => JSON.parse(readFileSync(join(media, p), 'utf8'));

/** Renders one loop composition as PNG frames and writes them as a GIF with exact colours. */
function loop(theme) {
  const delay = 100 / LOOP.fps;
  if (!Number.isInteger(delay)) throw new Error(`a GIF frame lasts whole hundredths of a second; ${LOOP.fps} fps does not fit`);
  const frames = mkdtempSync(join(tmpdir(), 'wakeshot-loop-'));
  try {
    const flags = ['--sequence', '--image-format=png', `--concurrency=${concurrency}`];
    remotion('render', 'src/index.ts', `IconLoop${theme}`, frames, ...flags);
    const files = readdirSync(frames).filter((f) => f.endsWith('.png')).sort();
    if (files.length !== LOOP.frames) throw new Error(`expected ${LOOP.frames} frames, Remotion wrote ${files.length}`);
    const out = join(media, 'out', `icon-loop-${theme.toLowerCase()}.gif`);
    const r = writeGif(out, LOOP.width, LOOP.height, files.map((f) => ({rgba: readPng(join(frames, f)).rgba, delay})));
    console.log(
      `${out}: ${r.frames} GIF frames from ${files.length}, at most ${r.mostColours} colours in one frame, ` +
        `merged edge colours moved at most ${r.worstMerge} levels`,
    );
  } finally {
    rmSync(frames, {recursive: true, force: true});
  }
}

/** The final video shows only real captures and measured numbers; this lists what is not there yet. */
function missingForFinal() {
  const out = [];
  for (const [set, info] of Object.entries(json('data/shots.json').sets)) {
    for (const [id, s] of Object.entries(info.shots)) {
      if (!s.file) out.push(`set ${set}, shot ${id} has no capture yet`);
      else if (s.standIn) out.push(`set ${set}, shot ${id} is a stand-in`);
      else if (!existsSync(join(media, 'public', s.file))) out.push(`set ${set}, shot ${id}: public/${s.file} is missing`);
    }
  }
  const speed = json('data/speed.json');
  if (!speed.measured) out.push('data/speed.json: not measured');
  for (const k of ['build', 'date', 'idleRamMb', 'idleCpuMsPer10Min', 'startDelayS']) {
    if (speed[k] === null || speed[k] === undefined) out.push(`data/speed.json: no ${k}`);
  }
  if (json('data/settings.json').standIn) out.push('data/settings.json: stand-in values');
  return out;
}

function assertFinal() {
  const missing = missingForFinal();
  if (missing.length) {
    console.error(`Not ready for the final render (use "draft" until then):\n- ${missing.join('\n- ')}`);
    process.exit(1);
  }
}

// CRF 18, as `promo-video.md` "Formats" says once a draft passes 8 MB at 16: the round 2 draft was 8.5 MB at 16. Not
// 20: at 20 the app icon's plate broke into blocks while its light came on (Part A).
const VIDEO_FLAGS = [
  '--codec=h264',
  '--audio-codec=aac',
  '--audio-bitrate=192k',
  '--crf=18',
  '--pixel-format=yuv420p',
  '--image-format=png',
  `--concurrency=${concurrency}`,
];

const mode = process.argv[2];
mkdirSync(join(media, 'out', 'draft'), {recursive: true});
switch (mode) {
  case 'loop':
    loop('Light');
    loop('Dark');
    break;
  case 'draft':
    sounds();
    remotion('render', 'src/index.ts', 'Promo', 'out/draft/wakeshot-promo-draft.mp4', '--props={"draft":true}', ...VIDEO_FLAGS);
    remotion('still', 'src/index.ts', 'Promo', 'out/draft/wakeshot-promo-poster-draft.png', `--frame=${POSTER_FRAME}`, '--props={"draft":true}');
    break;
  case 'video':
    assertFinal();
    sounds();
    remotion('render', 'src/index.ts', 'Promo', 'out/wakeshot-promo.mp4', '--props={"draft":false}', ...VIDEO_FLAGS);
    remotion('still', 'src/index.ts', 'Promo', 'out/wakeshot-promo-poster.png', `--frame=${POSTER_FRAME}`, '--props={"draft":false}');
    break;
  case 'poster':
    assertFinal();
    remotion('still', 'src/index.ts', 'Promo', 'out/wakeshot-promo-poster.png', `--frame=${POSTER_FRAME}`, '--props={"draft":false}');
    break;
  default:
    console.error('usage: node scripts/build.mjs loop|draft|video|poster');
    process.exit(2);
}
