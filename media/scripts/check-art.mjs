// Checks that the port of the art (src/art/heldLight.ts) draws exactly what the Rust art code draws, with no
// tolerance: every pixel of the three review sheets (tray glyphs at 16 to 48 px, four rows, real size and enlarged),
// every frame of assets/wakeshot.ico (the 256 frame is a PNG) and the notice picture, all written by art.rs
// (`write_art_files`). Run after any change to the art: `node scripts/check-art.mjs`.
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';
import {readPng, readIco} from './lib/png.mjs';
import * as art from '../src/art/heldLight.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const img = join(root, 'docs', 'research', 'img', 'ux');

let failed = false;
let pixels = 0;

/** Compares two RGBA images of `width` x `height`; prints the first differing pixel. */
function compare(label, width, height, want, got) {
  let off = 0;
  let max = 0;
  let first = '';
  for (let i = 0; i < width * height; i++) {
    let d = 0;
    for (let c = 0; c < 4; c++) d = Math.max(d, Math.abs(want[i * 4 + c] - got[i * 4 + c]));
    if (d === 0) continue;
    off++;
    max = Math.max(max, d);
    if (!first) {
      const at = (a) => Array.from(a.subarray(i * 4, i * 4 + 4)).join(',');
      first = `, first at ${i % width},${Math.floor(i / width)}: art.rs ${at(want)}, port ${at(got)}`;
    }
  }
  pixels += width * height;
  if (off) failed = true;
  console.log(`${off ? 'FAIL' : 'ok  '} ${label}: ${off ? `${off} pixels differ, by up to ${max}${first}` : 'every pixel equal'}`);
}

// The review sheets: `write_art_files` in art.rs, the same backgrounds and colours.
for (const [name, back, colours] of [
  ['dark', art.TASKBAR_DARK, art.ON_DARK],
  ['light', art.TASKBAR_LIGHT, art.ON_LIGHT],
  ['contrast', [0, 0, 0], art.one([0xff, 0xff, 0])],
]) {
  const file = readPng(join(img, `icon-sheet-${name}.png`));
  const want = art.sheet(back, colours);
  if (file.width !== want.width || file.height !== want.height) {
    failed = true;
    console.log(`FAIL sheet ${name}: ${file.width} x ${file.height}, the port's layout is ${want.width} x ${want.height}`);
    continue;
  }
  compare(`sheet ${name} (${file.width} x ${file.height})`, file.width, file.height, file.rgba, want.pixels);
}

const frames = readIco(join(root, 'assets', 'wakeshot.ico'));
const sizes = frames.map((f) => f.size).join(',');
const listOk = sizes === art.APP_ICON_SIZES.join(',');
if (!listOk) failed = true;
console.log(`${listOk ? 'ok  ' : 'FAIL'} wakeshot.ico frames ${sizes} (the port lists ${art.APP_ICON_SIZES.join(',')})`);
for (const frame of frames) {
  const want = art.appIcon(frame.size);
  compare(`app icon ${frame.size} px${frame.png ? ' (PNG frame)' : ''}`, frame.size, frame.size, frame.rgba, want.pixels);
}

// The notice header's picture: the flat drawing at 48 px (`notice_icon_png` in notify.rs calls `app_icon_flat(48)`).
const notice = readPng(join(root, 'assets', 'wakeshot-notice.png'));
compare('notice picture 48 px (flat)', notice.width, notice.height, notice.rgba, art.appIconFlat(notice.width).pixels);

console.log(failed ? 'FAIL: the port differs from art.rs' : `ok: the port matches art.rs in all ${pixels} pixels`);
process.exit(failed ? 1 : 0);
