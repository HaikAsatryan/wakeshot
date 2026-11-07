// Tries a found music track on the video: fits it (`lib/fit-music.mjs`: tempo, lift, 120 BPM, the storyboard's cut)
// and writes a 48 kHz stereo 16-bit WAV that `make-sounds.mjs --music` takes in place of the bed.
//   node scripts/music-candidate.mjs <track> --out <file.wav> [--bpm 118] [--lift 31.5]
// `--bpm` and `--lift` (seconds into the original track) override what the script finds; it prints both, so a pick
// is rebuilt with the same numbers (put them in `data/music.json`).
import {resolve} from 'node:path';
import {writeWav} from './lib/audio.mjs';
import {TARGET_BPM, fitTrack} from './lib/fit-music.mjs';

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : null;
};
const input = args[0] && !args[0].startsWith('--') ? resolve(args[0]) : null;
const out = opt('out');
if (!input || !out) {
  console.error('usage: node scripts/music-candidate.mjs <track> --out <file.wav> [--bpm N] [--lift seconds]');
  process.exit(2);
}
const {L, R, info} = fitTrack(input, {bpm: opt('bpm') === null ? null : Number(opt('bpm')), lift: opt('lift') === null ? null : Number(opt('lift'))});
if (info.ratio < 0.85 || info.ratio > 1.15) console.warn(`warning: ${info.bpm} BPM is far from ${TARGET_BPM}; atempo ${info.ratio.toFixed(3)} changes its feel`);
writeWav(resolve(out), L, R);
console.log(
  `${out}: ${info.bpm} BPM conformed to ${TARGET_BPM} (atempo ${info.ratio.toFixed(4)}), --lift ${info.lift.toFixed(2)}, ` +
    `cut from ${info.start.toFixed(2)} s of the original; levels per 2 s: ${info.levels.map((x) => x.toFixed(0)).join(' ')}`,
);
