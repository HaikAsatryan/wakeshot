// Checks the rendered files against the specs in docs/research/launch.md part 4 with ffprobe (the one Remotion
// ships, so nothing else is installed): the video is H.264 and AAC, 1920 x 1080, 30 fps, 30 s plus or minus 2, under
// 10 MB (GitHub's free-plan attachment limit). Each icon loop is read frame by frame (scripts/lib/gif.mjs): a GIF of
// 4 to 6 s that loops forever, under 1 MB, its last frame equal to its first (no jump), a 40 px border of every
// frame exactly GitHub's page colour for its theme (no visible box on the README), and its idle and holding frames
// exactly the art port's tray glyph (`tray_mask` at `LOOP.art` px, composed on the page as the review sheets are).
//   node scripts/check-video.mjs           the final video
//   node scripts/check-video.mjs --draft   the draft video
import {spawnSync} from 'node:child_process';
import {existsSync, readFileSync, statSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {ON_DARK, ON_LIGHT} from '../src/art/heldLight.ts';
import {rgbOf, trayFade, trayPlace} from '../src/art/motion.ts';
import {GITHUB} from '../src/theme.ts';
import {LOOP} from '../src/timeline.ts';
import {readGif} from './lib/gif.mjs';

const media = join(dirname(fileURLToPath(import.meta.url)), '..');
const ffprobe = join(media, 'node_modules', '@remotion', 'compositor-win32-x64-msvc', 'ffprobe.exe');
const draft = process.argv.includes('--draft');
const video = join(media, 'out', draft ? 'draft/wakeshot-promo-draft.mp4' : 'wakeshot-promo.mp4');

let failed = false;
const check = (ok, text) => {
  if (!ok) failed = true;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`);
};

function probe(file) {
  const r = spawnSync(ffprobe, ['-v', 'error', '-show_streams', '-show_format', '-of', 'json', file], {encoding: 'utf8'});
  if (r.status !== 0) throw new Error(`ffprobe failed on ${file}: ${r.stderr}`);
  return JSON.parse(r.stdout);
}

if (!existsSync(video)) {
  check(false, `${video} exists (build it first)`);
} else {
  const p = probe(video);
  const v = p.streams.find((s) => s.codec_type === 'video');
  const a = p.streams.find((s) => s.codec_type === 'audio');
  const seconds = Number(p.format.duration);
  const mb = statSync(video).size / 1e6;
  check(v?.codec_name === 'h264', `video codec h264 (${v?.codec_name})`);
  check(v?.width === 1920 && v?.height === 1080, `1920 x 1080 (${v?.width} x ${v?.height})`);
  check(v?.r_frame_rate === '30/1', `30 fps (${v?.r_frame_rate})`);
  check(v?.pix_fmt === 'yuv420p', `yuv420p (${v?.pix_fmt})`);
  check(a?.codec_name === 'aac', `audio track aac (${a?.codec_name ?? 'none'})`);
  check(Math.abs(seconds - 30) <= 2, `30 s plus or minus 2 (${seconds.toFixed(2)} s)`);
  check(mb < 10, `under 10 MB (${mb.toFixed(2)} MB)`);
}

for (const theme of ['light', 'dark']) {
  const gif = join(media, 'out', `icon-loop-${theme}.gif`);
  if (!existsSync(gif)) {
    check(false, `${gif} exists (build it with the loop command)`);
    continue;
  }
  const bytes = readFileSync(gif);
  const g = readGif(gif);
  const cs = g.frames.reduce((sum, f) => sum + f.delay, 0);
  check(g.version === 'GIF89a', `${theme} loop is a GIF89a`);
  check(g.width === 480 && g.height === 240, `${theme} loop is 480 x 240 (${g.width} x ${g.height})`);
  check(cs >= 400 && cs <= 600, `${theme} loop lasts 4 to 6 s (${cs / 100} s in ${g.frames.length} frames)`);
  check(g.loops === 0, `${theme} loop repeats forever (NETSCAPE loop count ${g.loops})`);
  check(bytes.length < 1e6, `${theme} loop under 1 MB (${(bytes.length / 1e6).toFixed(3)} MB, ${bytes.length} bytes)`);
  const first = g.frames[0].rgba;
  const last = g.frames[g.frames.length - 1].rgba;
  check(Buffer.from(first).equals(Buffer.from(last)), `${theme} loop ends on its first frame (seamless)`);
  const want = GITHUB[theme];
  const rgb = [1, 3, 5].map((i) => parseInt(want.slice(i, i + 2), 16));
  const BORDER = 40;
  let off = 0;
  let seen = '';
  for (const f of g.frames) {
    for (let y = 0; y < g.height; y++) {
      for (let x = 0; x < g.width; x++) {
        if (x >= BORDER && x < g.width - BORDER && y >= BORDER && y < g.height - BORDER) continue;
        const i = (y * g.width + x) * 4;
        if (f.rgba[i] !== rgb[0] || f.rgba[i + 1] !== rgb[1] || f.rgba[i + 2] !== rgb[2]) {
          off++;
          seen ||= `#${[0, 1, 2].map((k) => f.rgba[i + k].toString(16).padStart(2, '0')).join('')}`;
        }
      }
    }
  }
  const detail = off ? `${off} border pixels differ, e.g. ${seen}` : 'all border pixels';
  check(off === 0, `${theme} loop background is GitHub's ${want} in every frame (${detail})`);

  // The whole frame against the port: the glyph's square holds its pixels, everything else is the page.
  const fade = trayFade(LOOP.art, theme === 'dark' ? ON_DARK : ON_LIGHT, rgb);
  const at = trayPlace(g.width, g.height, LOOP.art);
  const expect = (glyph) => {
    const out = new Uint8Array(g.width * g.height * 4);
    for (let i = 0; i < g.width * g.height; i++) out.set([...rgb, 255], i * 4);
    for (let y = 0; y < LOOP.art; y++) {
      const s = y * LOOP.art * 4;
      out.set(glyph.subarray(s, s + LOOP.art * 4), ((at.top + y) * g.width + at.left) * 4);
    }
    return Buffer.from(out);
  };
  const [idle, holding] = [expect(fade.from), expect(fade.to)];
  check(Buffer.from(first).equals(idle), `${theme} loop starts on the port's idle glyph, every pixel`);
  const held = g.frames.filter((f) => Buffer.from(f.rgba).equals(holding)).reduce((sum, f) => sum + f.delay, 0);
  check(held > 0, `${theme} loop shows the port's holding glyph, every pixel (for ${held / 100} s)`);
}

process.exit(failed ? 1 : 0);
