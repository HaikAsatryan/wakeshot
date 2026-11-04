// Checks the rendered files against the specs in docs/research/launch.md part 4 with ffprobe (the one Remotion
// ships, so nothing else is installed): the video is H.264 and AAC, 1920 x 1080, 30 fps, 30 s plus or minus 2, under
// 10 MB (GitHub's free-plan attachment limit); each icon loop is a GIF of 4 to 6 s that loops forever, under 1 MB.
//   node scripts/check-video.mjs           the final video
//   node scripts/check-video.mjs --draft   the draft video
import {spawnSync} from 'node:child_process';
import {existsSync, readFileSync, statSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

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
  // Each frame's delay sits in its Graphic Control Extension (0x21 0xF9 0x04), in hundredths of a second.
  let cs = 0;
  for (let i = 0; i + 6 < bytes.length; i++) {
    if (bytes[i] === 0x21 && bytes[i + 1] === 0xf9 && bytes[i + 2] === 4) cs += bytes.readUInt16LE(i + 4);
  }
  const n = bytes.indexOf('NETSCAPE2.0');
  const loops = n > 0 ? bytes.readUInt16LE(n + 13) : null;
  check(bytes.subarray(0, 6).toString('latin1') === 'GIF89a', `${theme} loop is a GIF89a`);
  check(cs >= 400 && cs <= 600, `${theme} loop lasts 4 to 6 s (${cs / 100} s)`);
  check(loops === 0, `${theme} loop repeats forever (NETSCAPE loop count ${loops})`);
  check(bytes.length < 1e6, `${theme} loop under 1 MB (${(bytes.length / 1e6).toFixed(2)} MB)`);
}

process.exit(failed ? 1 : 0);
