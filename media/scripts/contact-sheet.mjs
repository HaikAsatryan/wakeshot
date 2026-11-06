// A contact sheet for review: renders about 4 frames a second (every 7.5, plus the last) from `from` to `to` of a
// composition at full size, shrinks each to a quarter, and tiles them in rows into one PNG, so a scene is checked in
// one look against its storyboard row. Prints which frame each tile is. Full size, then shrunk here: Chrome's
// backdrop filter (the focus dimming) draws mirrored copies when Remotion renders at `--scale=0.25`.
//   node scripts/contact-sheet.mjs <composition> <from> <to> <out.png> [--every 7.5] [--cols 6] [--shrink 4] [--final]
// `--final` renders the frames as the final video draws them, without the DRAFT mark (it refuses stand-ins).
import {spawnSync} from 'node:child_process';
import {mkdtempSync, readdirSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {encodePng, readPng} from './lib/png.mjs';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const cli = join(media, 'node_modules', '@remotion', 'cli', 'remotion-cli.js');
const args = process.argv.slice(2);
const flag = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? Number(args[i + 1]) : fallback;
};
const [comp, fromText, toText, out] = args;
if (!comp || !out) {
  console.error('usage: node scripts/contact-sheet.mjs <composition> <from> <to> <out.png> [--every 7.5] [--cols 6] [--shrink 4] [--final]');
  process.exit(2);
}
const from = Number(fromText);
const to = Number(toText);
const every = flag('every', 7.5);
const cols = flag('cols', 6);
const shrink = flag('shrink', 4);
const picks = [];
for (let k = 0; from + Math.round(k * every) <= to; k++) picks.push(from + Math.round(k * every));
if (picks[picks.length - 1] !== to) picks.push(to);

/** A picture shrunk by `shrink` each way, each pixel the mean of its square. */
function small(img) {
  const w = Math.floor(img.width / shrink);
  const h = Math.floor(img.height / shrink);
  const rgba = new Uint8Array(w * h * 4);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      for (let c = 0; c < 4; c++) {
        let sum = 0;
        for (let j = 0; j < shrink; j++) for (let i = 0; i < shrink; i++) sum += img.rgba[((y * shrink + j) * img.width + x * shrink + i) * 4 + c];
        rgba[(y * w + x) * 4 + c] = Math.round(sum / (shrink * shrink));
      }
    }
  }
  return {width: w, height: h, rgba};
}

const dir = mkdtempSync(join(tmpdir(), 'wakeshot-sheet-'));
try {
  const r = spawnSync(
    process.execPath,
    [
      cli,
      'render',
      'src/index.ts',
      comp,
      dir,
      '--sequence',
      `--frames=${picks.join(',')}`,
      '--image-format=png',
      `--concurrency=${process.env.MEDIA_CONCURRENCY ?? '4'}`,
      `--props={"draft":${!args.includes('--final')}}`,
      '--log=error',
    ],
    {cwd: media, stdio: 'inherit'},
  );
  if (r.status !== 0) process.exit(r.status ?? 1);
  const files = new Map(
    readdirSync(dir)
      .filter((f) => f.endsWith('.png'))
      .map((f) => [Number(f.match(/(\d+)\.png$/)[1]), join(dir, f)]),
  );
  const tiles = picks.map((f) => {
    const p = files.get(f);
    if (!p) throw new Error(`frame ${f} was not rendered`);
    return small(readPng(p));
  });
  const w = tiles[0].width;
  const h = tiles[0].height;
  const gap = 4;
  const rows = Math.ceil(tiles.length / cols);
  const W = cols * w + (cols + 1) * gap;
  const H = rows * h + (rows + 1) * gap;
  const sheet = new Uint8Array(W * H * 4).fill(255);
  tiles.forEach((t, i) => {
    const x0 = gap + (i % cols) * (w + gap);
    const y0 = gap + Math.floor(i / cols) * (h + gap);
    for (let y = 0; y < h; y++) sheet.set(t.rgba.subarray(y * w * 4, (y + 1) * w * 4), ((y0 + y) * W + x0) * 4);
  });
  writeFileSync(out, encodePng(W, H, sheet));
  console.log(`${out}: ${tiles.length} tiles, ${cols} a row, frames ${picks.join(' ')}`);
} finally {
  rmSync(dir, {recursive: true, force: true});
}
