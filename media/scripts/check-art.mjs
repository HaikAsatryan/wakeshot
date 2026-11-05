// Checks that the TypeScript port of the Sunrise (src/art/sunrise.ts) draws what the Rust art code draws: the tray
// glyphs against the review sheets and the app icon against assets/wakeshot.ico, both written by art.rs
// (`write_art_files`). Run after any change to the art: `node scripts/check-art.mjs`.
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';
import {readPng, readIco} from './lib/png.mjs';
import * as art from '../src/art/sunrise.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const img = join(root, 'docs', 'research', 'img', 'ux');

// The sheet layout from `sheet` in art.rs: rows holding, idle, off.
const SIZES = [16, 20, 24, 32, 40, 48];
const ZOOM = 8;
const GAP = 16;
const MARGIN = 32;
const ROW = 48 * ZOOM;
const STATES = ['holding', 'idle', 'off'];

// Rust samples in f32 and this port in f64, so a sample point that lands exactly on an edge can fall the other way:
// one sample of 64 moves a pixel by 4 levels of 255. Anything larger is a real difference in the art.
const MAX_DIFF = 8;

let worst = 0;
let failed = false;

function compare(label, want, got, count) {
  let max = 0;
  let off = 0;
  for (let i = 0; i < count; i++) {
    let d = 0;
    for (let c = 0; c < 4; c++) d = Math.max(d, Math.abs(want[i * 4 + c] - got[i * 4 + c]));
    if (d > 1) off++;
    max = Math.max(max, d);
  }
  worst = Math.max(worst, max);
  const ok = max <= MAX_DIFF;
  if (!ok) failed = true;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}: max diff ${max}, pixels off by more than 1: ${off} of ${count}`);
}

for (const [name, back, ink] of [
  ['dark', art.TASKBAR_DARK, art.GLYPH_ON_DARK],
  ['light', art.TASKBAR_LIGHT, art.GLYPH_ON_LIGHT],
]) {
  const sheet = readPng(join(img, `icon-sheet-${name}.png`));
  let top = MARGIN;
  for (const state of STATES) {
    let left = MARGIN;
    for (const size of SIZES) {
      const alpha = art.raster(size, size, art.glyphShape(state, art.grid(0, 0, size)));
      const y0 = top + Math.floor((ROW - size) / 2);
      const want = new Uint8Array(size * size * 4);
      const got = new Uint8Array(size * size * 4);
      for (let y = 0; y < size; y++) {
        for (let x = 0; x < size; x++) {
          const i = y * size + x;
          want.set([...art.mix(back, ink, alpha[i] / 255), 255], i * 4);
          const s = ((y0 + y) * sheet.width + left + x) * 4;
          got.set(sheet.rgba.subarray(s, s + 4), i * 4);
        }
      }
      compare(`tray ${state} ${size} px on ${name}`, want, got, size * size);
      left += size + GAP;
    }
    top += ROW + GAP;
  }
}

for (const frame of readIco(join(root, 'assets', 'wakeshot.ico'))) {
  compare(`app icon ${frame.size} px`, art.appIconRgba(frame.size), frame.rgba, frame.size * frame.size);
}

console.log(failed ? `FAIL: the port differs from art.rs (worst ${worst})` : `ok: the port matches art.rs (worst ${worst})`);
process.exit(failed ? 1 : 0);
