// Holds the video's timeline (src/timeline.ts) to the pace rules of docs/research/promo-video.md, from the data, not the
// pixels:
// - every cut, camera move, pop, click, light, count and caption start lands on the 120 BPM beat grid (within 1 frame);
// - something new on screen at least every 2 s, the end card excepted;
// - at most 5 things to take in within any 2 s before the end card: cuts, moves, pops, clicks, lights, counts, rings,
//   swaps and the start of each line of words (streamed tool lines, single words and a line that flows on from the
//   one before not counted; a cut counts once with what it opens on). Round 2 spread the
//   stretches Haik found "a little fast" (6 in 2 s); round 4 filled the lid shot to 8 again and this check, which had
//   no upper limit, stayed green (review M1);
// - every framing (one camera place) lasts 2 to 4 s;
// - a caption holds at least 1.4 s after its last word lands; big words of 1 to 3 words at least 0.5 s;
// - a caption line has 2 to 6 words (big words and the name 1 to 6; the end card's lines, which pop in whole, are free);
// - at most two lines of words on screen at once before the end card (it holds still for 3 s and may show three);
//   the name on screen by 13 s (Part C, 2026-11-06: the pain and Haik's question come first, 4 s; Part D: Haik found
//   that too short, so the pain took 8 s; Part E: his opening, a prompt typed, the agent waiting, the laptop asleep,
//   takes 12 s and the name lands at 12.5 s); the end card is the last 4 s;
// - frame 0 is a full picture: words already in, or the camera on the opening terminal from frame 0 with its command
//   line and input box already on it (Part E has no words over the pain).
import {BEAT} from '../src/beat.ts';
import {CAMERA, CAPTIONS, END_CARD, EVENTS, KEYS, ON_GRID, SCENES, VIDEO, lineIn, wordStarts, wordsOf} from '../src/timeline.ts';

const fps = VIDEO.fps;
const fails = [];
const fail = (text) => fails.push(text);
const offGrid = (f) => Math.abs(f - Math.round(f / BEAT) * BEAT);
const s = (f) => `${(f / fps).toFixed(2)} s (frame ${f})`;

// The beat grid.
const gridded = [
  ...EVENTS.filter((e) => ON_GRID.includes(e.kind)).map((e) => ({at: e.at, what: `${e.kind}: ${e.what}`})),
  ...CAMERA.filter((k) => k.kind !== 'settle').map((k) => ({at: k.at, what: `camera ${k.kind} to ${k.to}`})),
  ...CAPTIONS.flatMap((b) => b.lines.filter((l, i) => !(l.flow && i > 0)).map((l) => ({at: l.at, what: `words "${l.text}"`}))),
];
for (const g of gridded) if (offGrid(g.at) > 1) fail(`off the beat by ${offGrid(g.at)} frames at ${s(g.at)}: ${g.what}`);

// Something new at least every 2 s.
const moments = [
  ...EVENTS.map((e) => e.at),
  ...CAMERA.map((k) => k.at),
  ...CAPTIONS.flatMap((b) => wordStarts(b).flat()),
].sort((a, b) => a - b);
let longest = 0;
for (let i = 1; i < moments.length; i++) {
  if (moments[i - 1] >= END_CARD) break;
  const gap = Math.min(moments[i], END_CARD) - moments[i - 1];
  longest = Math.max(longest, gap);
  if (gap > 2 * fps) fail(`nothing new for ${(gap / fps).toFixed(2)} s after ${s(moments[i - 1])}`);
}

// At most 5 things to take in within any 2 s. A camera key counts once with the event that starts on its frame.
const MOST_IN_2S = 5;
const take = [
  ...EVENTS.filter((e) => e.kind !== 'stream' && e.kind !== 'word').map((e) => ({at: e.at, kind: e.kind, what: `${e.kind}: ${e.what}`})),
  ...CAMERA.filter((k) => k.kind !== 'settle' && !EVENTS.some((e) => e.at === k.at && e.kind !== 'stream')).map((k) => ({
    at: k.at,
    kind: k.kind,
    what: `camera ${k.kind} to ${k.to}`,
  })),
  ...CAPTIONS.flatMap((b) => b.lines.filter((l, i) => !l.already && !(l.flow && i > 0)).map((l) => ({at: l.at, kind: 'line', what: `line "${l.text}"`}))),
].filter((x) => x.at < END_CARD);
let densest = 0;
for (let f = 0; f < END_CARD; f++) {
  const inside = take.filter((x) => x.at >= f && x.at < f + 2 * fps);
  densest = Math.max(densest, inside.length);
  if (inside.length > MOST_IN_2S) {
    fail(`${inside.length} things within 2 s from ${s(f)}, at most ${MOST_IN_2S}: ${inside.map((x) => `${x.what} (${x.at})`).join('; ')}`);
    break;
  }
}

// Framings: 2 to 4 s each.
const starts = [...new Set(CAMERA.filter((k) => k.kind !== 'settle').map((k) => k.at))].sort((a, b) => a - b);
const lengths = starts.map((at, i) => (i + 1 < starts.length ? starts[i + 1] : VIDEO.frames) - at);
lengths.forEach((len, i) => {
  if (len < 2 * fps || len > 4 * fps) fail(`framing at ${s(starts[i])} lasts ${(len / fps).toFixed(2)} s, not 2 to 4 s`);
});

// Holds, word counts, lines on screen.
const endStyles = new Set(['line', 'mono', 'small']);
const spans = [];
for (const b of CAPTIONS) {
  b.lines.forEach((l, i) => {
    const n = wordsOf(l.text).length;
    const done = lineIn(b, i);
    const hold = b.until - done;
    spans.push({from: l.at, to: b.until, what: l.text});
    if (endStyles.has(l.style)) return;
    const min = l.style === 'caption' ? 2 : 1;
    if (n < min || n > 6) fail(`"${l.text}" has ${n} words, not ${min} to 6`);
    const need = l.style !== 'caption' && n <= 3 ? 0.5 * fps : 1.4 * fps;
    if (hold < need) fail(`"${l.text}" holds ${(hold / fps).toFixed(2)} s after its last word, needs ${(need / fps).toFixed(1)} s`);
  });
}
for (let f = 0; f < END_CARD; f++) {
  const on = spans.filter((x) => f >= x.from && f < x.to);
  if (on.length > 2) {
    fail(`${on.length} lines on screen at ${s(f)}: ${on.map((x) => `"${x.what}"`).join(', ')}`);
    break;
  }
}
const name = CAPTIONS.flatMap((b) => b.lines).find((l) => l.style === 'name');
if (!name || name.at > 13 * fps) fail(`the name is not on screen by 13 s (${name ? s(name.at) : 'never'})`);
if (VIDEO.frames - SCENES.end.from !== 4 * fps) fail(`the end card lasts ${((VIDEO.frames - SCENES.end.from) / fps).toFixed(2)} s, not 4 s`);
const wordsAtZero = CAPTIONS.some((b) => b.lines.some((l) => l.already && l.at === 0));
// The terminal draws its command line and input box from its first frame; the first key may not land on frame 0, so
// the picture is whole before anything moves.
const terminalAtZero = CAMERA[0].at === 0 && CAMERA[0].kind === 'cut' && CAMERA[0].to === 'typeClose' && KEYS.length > 0 && KEYS[0].at > 0;
if (!wordsAtZero && !terminalAtZero) fail('frame 0 shows no words already in and does not open on the terminal');

const events = moments.filter((m) => m < END_CARD).length;
console.log(
  fails.length
    ? fails.map((x) => `FAIL ${x}`).join('\n')
    : `ok: ${gridded.length} starts on the beat, ${starts.length} framings of ${(Math.min(...lengths) / fps).toFixed(1)} to ` +
        `${(Math.max(...lengths) / fps).toFixed(1)} s, ${events} new things before the end card (one every ` +
        `${(END_CARD / fps / events).toFixed(2)} s, the longest gap ${(longest / fps).toFixed(2)} s), at most ${densest} things ` +
        `to take in within 2 s, ` +
        `${CAPTIONS.flatMap((b) => b.lines).length} lines of words`,
);
process.exit(fails.length ? 1 : 0);
