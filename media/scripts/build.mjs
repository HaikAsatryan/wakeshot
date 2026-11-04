// Builds the media files. One mode per command (media/README.md):
//   node scripts/build.mjs loop     out/icon-loop-light.gif, out/icon-loop-dark.gif
//   node scripts/build.mjs draft    out/draft/wakeshot-promo-draft.mp4 and its poster (stand-ins allowed, marked)
//   node scripts/build.mjs video    out/wakeshot-promo.mp4 and out/wakeshot-promo-poster.png (refuses stand-ins)
//   node scripts/build.mjs poster   out/wakeshot-promo-poster.png only (refuses stand-ins)
// MEDIA_CONCURRENCY sets how many frames render at once (default 4, so other work on the laptop keeps its cores).
import {spawnSync} from 'node:child_process';
import {existsSync, mkdirSync, readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const cli = join(media, 'node_modules', '@remotion', 'cli', 'remotion-cli.js');
const concurrency = process.env.MEDIA_CONCURRENCY ?? '4';
const POSTER_FRAME = 75;

function run(args) {
  const r = spawnSync(process.execPath, args, {cwd: media, stdio: 'inherit'});
  if (r.status !== 0) process.exit(r.status ?? 1);
}

const remotion = (...args) => run([cli, ...args]);
const sounds = () => run([join(media, 'scripts', 'make-sounds.mjs')]);
const json = (p) => JSON.parse(readFileSync(join(media, p), 'utf8'));

/** The final video shows only real captures and measured numbers; this lists what is not there yet. */
function missingForFinal() {
  const out = [];
  for (const [id, s] of Object.entries(json('data/shots.json').shots)) {
    if (s.standIn) out.push(`shot ${s.shot} ${id} is a stand-in`);
    else if (!s.file || !existsSync(join(media, 'public', s.file))) out.push(`shot ${s.shot} ${id} has no file`);
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

const VIDEO_FLAGS = [
  '--codec=h264',
  '--audio-codec=aac',
  '--audio-bitrate=192k',
  '--crf=20',
  '--pixel-format=yuv420p',
  '--image-format=png',
  `--concurrency=${concurrency}`,
];

const mode = process.argv[2];
mkdirSync(join(media, 'out', 'draft'), {recursive: true});
switch (mode) {
  case 'loop':
    for (const theme of ['Light', 'Dark']) {
      remotion('render', 'src/index.ts', `IconLoop${theme}`, `out/icon-loop-${theme.toLowerCase()}.gif`, '--codec=gif', `--concurrency=${concurrency}`);
    }
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
