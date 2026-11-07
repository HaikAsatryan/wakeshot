// One timeline for the picture and the sound. The scenes read their frames from here, `scripts/make-sounds.mjs` places
// every sound at the same frames and `scripts/check-pace.mjs` holds it all to the style rules of
// `docs/research/promo-video.md`, so moving a moment moves its sound and its check with it. Plain types only: Node
// runs this file too.
import {chips} from './agents.ts';
import {BEAT, beat} from './beat.ts';
import {COPY} from './copy.ts';

// 38 s: the opening (a person types a prompt, the agent mostly waits and loads, the laptop sleeps, no music until
// Wakeshot comes in) takes 12 s, so the video may pass the 30 s line.
export const VIDEO = {width: 1920, height: 1080, fps: 30, frames: 1140};
/**
 * The README loop. `art` is the tray glyph's size in it: `tray_mask` drawn at 128 px, not scaled from a smaller one;
 * the README shows the loop at half size, so the glyph reads at 64 px and stays sharp on a HiDPI screen.
 */
export const LOOP = {width: 480, height: 240, fps: 25, frames: 125, art: 128};
/**
 * The README state table's icons: `tray_mask` drawn at 32 px, the tray at 200%. The README shows them at 16 px, the
 * tray's size at 100%, so they stay sharp on a HiDPI screen.
 */
export const STATE_ICON = 32;

/** How long the light takes to come on: 0.44 s, as in the README loop (11 frames at 25 fps). */
export const LIGHT_ON_FRAMES = 13;
/** How long the light takes to go off: the loop's off curve, 0.8 s. */
export const LIGHT_OFF_FRAMES = 24;

/**
 * The eight shots of the storyboard (`docs/research/promo-video.md`), in frames: the pain first (the laptop sleeps
 * mid-work, "Tired of sleepy laptops?"), the name, then the agents it works with; the tray proof, the lid, the settings
 * and the sleep follow. The menu shot left so the pain and the agents get their time. The pain takes 12 s (6 bars, so
 * the light, where the music now starts, lands on a bar line and the track's bars line up with the rest).
 */
export const SCENES = {
  hook: {from: beat(0), len: beat(24)},
  light: {from: beat(24), len: beat(6)},
  agents: {from: beat(30), len: beat(8)},
  tray: {from: beat(38), len: beat(10)},
  lid: {from: beat(48), len: beat(8)},
  settings: {from: beat(56), len: beat(6)},
  sleeps: {from: beat(62), len: beat(6)},
  end: {from: beat(68), len: beat(8)},
} as const;

/**
 * The person at the keyboard: the prompt (`COPY.terminal.prompt`) typed one key at a time by a fluent typist, about
 * 11 keys a second, a little slower after a space and before the capitals. Each number is the frames from one key to
 * the next; the first key lands at `KEY_FROM`. Hand-set, not random, so it reads the same in the code as on screen.
 */
const KEY_FROM = 12;
const KEY_GAPS = [3, 2, 3, 2, 3, 2, 3, 4, 2, 3, 2, 3, 2, 3, 2, 4, 2, 3, 5, 3, 2, 3, 4, 3];
if (KEY_GAPS.length !== COPY.terminal.prompt.length - 1) {
  throw new Error(`KEY_GAPS has ${KEY_GAPS.length} gaps for a ${COPY.terminal.prompt.length} character prompt: set one gap per key after the first`);
}
/** Each key of the prompt and the frame it lands on: the letter shows and the key sounds at that frame. */
export const KEYS: {at: number; ch: string}[] = [...COPY.terminal.prompt].map((ch, i) => ({
  ch,
  at: KEY_FROM + KEY_GAPS.slice(0, i).reduce((a, b) => a + b, 0),
}));
/** Shift goes down a little before the first capital and is held through the rest of the word ("API"). */
export const SHIFT_AT = (() => {
  const first = KEYS.find((k) => k.ch !== k.ch.toLowerCase());
  return first ? first.at - 2 : null;
})();

/** Every moment the scenes and the sound share, in absolute frames. */
export const T = {
  /**
   * The pain, as a person lives it: close on a terminal, a person types a prompt to an agent (`KEYS`) and sends it
   * (`enter`); the agent then mostly waits and loads, a spinner and a thinking line with its clock, a tool line now and
   * then (`lines`). Its clock runs in real time until `lapse`, then as a time lapse, so minutes go by while nobody
   * touches the laptop. A cut to the whole laptop with the same screen (`cut`); the idle timer dims the screen while
   * the agent still works (`dim`); then the screen goes off, the work stops and the power light starts to breathe
   * (`sleep`). The scene sinks into the dark (`sink`), so the question stands alone on the clean stage the light then
   * comes on in (a ghost of the laptop behind it read as a glitch).
   */
  hook: {
    enter: beat(6),
    lapse: beat(6) + 36,
    /** The tool lines, now and then and never evenly: real agents spend most of their time waiting. */
    lines: [132, 171, 219, 243],
    cut: beat(13),
    dim: beat(15),
    /** The screen dims over this many frames, then goes off over `offLen` at `sleep`. */
    dimLen: 10,
    sleep: beat(17),
    offLen: 6,
    sink: beat(19),
    /** The scene fades into the clean stage over this many frames, done before the question starts. */
    sinkLen: 12,
    question: beat(20),
  },
  light: {on: beat(24), name: beat(25)},
  /**
   * The agents: the light rises and shrinks to the top as the camera moves; the agent tiles come in one after another
   * (`tileStep` frames apart, read as one group like the words of a line); the line; the light pulses: it holds for
   * every one of them.
   */
  agents: {move: beat(30), tiles: beat(31), tileStep: 5, line: beat(33), pulse: beat(35)},
  /** The agent starts and the icon turns holding; the tooltip under the cursor; the work ends and the icon lets go. */
  tray: {whip: beat(38), whipLen: 12, start: beat(39), hover: beat(41), drift: beat(42), done: beat(44)},
  /**
   * The lid shuts, Wakeshot locks (the badge comes with the chime, as the app locks at the lid close) and the work goes
   * on behind it: at `work` a chip rises above the shut laptop with the agent's step under a turning spinner; at `done`
   * it shows the work done and, at that same frame, the edge light goes off: the hold ends (the work going on, then
   * clearly done, then the sleep). One caption and one chip step, so the shot is not the densest stretch again.
   */
  lid: {cut: beat(48), settle: 15, close: beat(49), shut: beat(50), lock: beat(50), work: beat(52), orbit: beat(52), done: beat(55)},
  settings: {move: beat(56), card: beat(57), row1: beat(59), row2: beat(60)},
  /**
   * A time cut: the frame goes to black into the cut, "Later" lands on the black, and the picture comes back under the
   * rest of the line; then the laptop goes to sleep. `dip` holds the frames where the black starts, is full, starts to
   * lift and is gone: 8 frames going down, 7 black (the cut in the middle), 9 coming back, 0.8 s in all, and the
   * picture is fully back on the next beat. Black, not a dim: the settings shot before it is
   * already dimmed to 35% by the focus, and the old 0.2 s dip to 85% read as a dropped picture, not as time passing.
   * No clock and no countdown on screen: Wakeshot sleeps a closed laptop once nothing has happened
   * for 5 minutes (`docs/product.md`), and the video claims no time it does not keep.
   */
  sleeps: {cut: beat(62), dip: [beat(62) - 10, beat(62) - 1, beat(62) + 5, beat(63)], line: beat(62), sleep: beat(64), moon: beat(65)},
  end: {cut: beat(68), line: beat(69), speed: beat(70), install: beat(71)},
} as const;

/** Camera moves between near places run this long; a whip is quicker (`promo-video.md`, "Motion"). */
export const MOVE_LEN = 15;

/**
 * The places in the one large world the camera works over, centre and zoom. Near places sit side by side, so the
 * camera slides between them; far places are reached by a hard cut.
 */
export const PLACES = {
  // The pain's terminal: a flat window 1500 x 906 centred here (`OPEN_TERMINAL` in world.ts), seen close, straight on,
  // so the typed prompt reads at about 52 px. First the input box and the line above it while the person types, the
  // window's left edge and title bar in view so it reads as a window; then, from the Enter, most of the window: the
  // prompt, the tool lines, the thinking line and its clock.
  terminal: {x: -9000, y: 0, zoom: 1},
  typeClose: {x: -9171, y: -280, zoom: 1.45},
  typeClosePush: {x: -9171, y: -280, zoom: 1.5},
  workClose: {x: -9150, y: -190, zoom: 1.25},
  workClosePush: {x: -9150, y: -186, zoom: 1.29},
  // The pain's laptop: a laptop of its own, far from the lid shot's, so each keeps its own state. The same screen as
  // the terminal above at the cut; then the camera eases back as it all sinks into the dark.
  // The laptop centred in the frame, as large as it fits with room for the chip: no words above it since Part E.
  hook: {x: -6000, y: 189, zoom: 1},
  hookPush: {x: -6000, y: 185, zoom: 1.03},
  hookAway: {x: -6000, y: 215, zoom: 0.88},
  light: {x: 0, y: 0, zoom: 1},
  // Below the light: the camera moves down, so the light rises to the top of the frame over the agent tiles.
  agents: {x: 0, y: 210, zoom: 1},
  agentsPush: {x: 0, y: 210, zoom: 1.03},
  tray: {x: 0, y: 1600, zoom: 1},
  trayDrift: {x: 90, y: 1620, zoom: 1},
  lid: {x: 6000, y: 0, zoom: 1},
  lidNear: {x: 6000, y: 0, zoom: 1.06},
  lidPush: {x: 6000, y: -20, zoom: 1.03},
  // A little wider than the lid's place (drawn parts only, so a zoom under 1 is allowed): the laptop, the badge, the
  // work chip and the card all fit without a cut edge.
  card: {x: 6500, y: 0, zoom: 0.92},
  end: {x: 20000, y: 0, zoom: 1},
} as const;
export type PlaceName = keyof typeof PLACES;

/**
 * The camera: from each key's frame it goes to `to`, at once (`cut`) or over `len` frames. A cut, a move and a whip
 * each start a new framing; a settle is the same framing coming to rest.
 */
export type CameraKey = {at: number; to: PlaceName; kind: 'cut' | 'move' | 'whip' | 'settle'; len: number};
export const CAMERA: CameraKey[] = [
  {at: SCENES.hook.from, to: 'typeClose', kind: 'cut', len: 0},
  // The slow push of a hold on drawn parts (`promo-video.md`, "Motion"), while the person types.
  {at: SCENES.hook.from, to: 'typeClosePush', kind: 'settle', len: T.hook.enter},
  {at: T.hook.enter, to: 'workClose', kind: 'move', len: MOVE_LEN},
  {at: T.hook.enter + MOVE_LEN, to: 'workClosePush', kind: 'settle', len: T.hook.cut - T.hook.enter - MOVE_LEN},
  // A cut, not a pull back: the close shots are a flat window, the laptop is seen at three-quarters.
  {at: T.hook.cut, to: 'hook', kind: 'cut', len: 0},
  {at: T.hook.cut, to: 'hookPush', kind: 'settle', len: T.hook.sink - T.hook.cut},
  {at: T.hook.sink, to: 'hookAway', kind: 'move', len: MOVE_LEN},
  {at: SCENES.light.from, to: 'light', kind: 'cut', len: 0},
  {at: T.agents.move, to: 'agents', kind: 'move', len: MOVE_LEN},
  {at: T.agents.tiles, to: 'agentsPush', kind: 'settle', len: T.tray.whip - T.agents.tiles},
  {at: T.tray.whip, to: 'tray', kind: 'whip', len: T.tray.whipLen},
  {at: T.tray.drift, to: 'trayDrift', kind: 'move', len: MOVE_LEN},
  {at: T.lid.cut, to: 'lidNear', kind: 'cut', len: 0},
  {at: T.lid.cut, to: 'lid', kind: 'settle', len: T.lid.settle},
  {at: T.settings.move, to: 'card', kind: 'move', len: MOVE_LEN},
  {at: T.sleeps.cut, to: 'lid', kind: 'cut', len: 0},
  // The slow push of a hold on drawn parts (`promo-video.md`, "Motion"): no capture is in view here.
  {at: T.sleeps.cut, to: 'lidPush', kind: 'settle', len: SCENES.sleeps.len},
  {at: T.end.cut, to: 'end', kind: 'cut', len: 0},
];

/**
 * The focus ring: from each key's frame it wraps the named target (the scenes say where each target is), or goes
 * away (`null`): faded over a few frames, or at once on a cut. `dim` false keeps the rest of the frame as it is: in
 * the tray shot the terminal, the cause, stays in full view.
 */
export type FocusKey = {at: number; target: string | null; cut?: boolean; dim?: boolean};
export const FOCUS: FocusKey[] = [
  {at: T.tray.hover, target: 'tip', dim: false},
  {at: T.tray.done, target: 'tipIdle', dim: false},
  {at: T.lid.cut, target: null, cut: true},
  {at: T.settings.row1, target: 'row1'},
  {at: T.settings.row2, target: 'row2'},
  {at: T.sleeps.cut, target: null, cut: true},
];

/**
 * Words on screen. Each line starts at its own frame and comes in word by word (or whole); every line of a block
 * leaves at `until`: with a short fade when a camera move starts there, at once when a cut does. `*word*` marks the
 * line's one emphasis word. `flow` lines continue the word stagger of the line above instead of starting their own.
 * `{name}` is filled in by the scene (the speed numbers).
 */
export type TextStyle = 'big' | 'name' | 'caption' | 'line' | 'mono' | 'small';
export type CaptionLine = {text: string; at: number; style: TextStyle; flow?: boolean; whole?: boolean; already?: boolean};
export type CaptionBlock = {id: string; lines: CaptionLine[]; until: number; leave: 'fade' | 'cut' | 'stay'};
export const CAPTIONS: CaptionBlock[] = [
  // No words over the pain until the question (Part E): the typed prompt, the thinking line and the "Sleeping" chip
  // tell it, and big words over the terminal would cover the prompt being typed.
  {id: 'question', lines: [{text: COPY.question, at: T.hook.question, style: 'big'}], until: SCENES.light.from, leave: 'cut'},
  {
    id: 'name',
    lines: [
      {text: COPY.name, at: T.light.name, style: 'name'},
      {text: COPY.tagline, at: T.light.name, style: 'caption', flow: true},
    ],
    until: T.agents.move,
    leave: 'fade',
  },
  {id: 'agents', lines: [{text: COPY.agents, at: T.agents.line, style: 'caption'}], until: T.tray.whip, leave: 'fade'},
  {id: 'trayStart', lines: [{text: COPY.tray[0], at: T.tray.start, style: 'caption'}], until: T.tray.done, leave: 'fade'},
  {id: 'trayDone', lines: [{text: COPY.tray[1], at: T.tray.done, style: 'caption'}], until: T.lid.cut, leave: 'cut'},
  {
    id: 'lid',
    lines: [
      {text: COPY.lid, at: T.lid.close, style: 'caption'},
    ],
    // It leaves as the work is done, so no line says the work goes on while the chip says it is done.
    until: T.lid.done,
    leave: 'fade',
  },
  {
    id: 'settings',
    lines: [{text: COPY.settings, at: T.settings.card, style: 'caption'}],
    until: T.sleeps.cut,
    leave: 'cut',
  },
  {
    id: 'sleeps',
    lines: [{text: COPY.sleeps, at: T.sleeps.line, style: 'caption'}],
    until: T.end.cut,
    leave: 'cut',
  },
  {
    id: 'end',
    lines: [
      {text: COPY.endLine, at: T.end.line, style: 'line', whole: true},
      {text: COPY.speedLine, at: T.end.speed, style: 'small', whole: true},
      {text: COPY.install, at: T.end.install, style: 'mono', whole: true},
    ],
    until: VIDEO.frames,
    leave: 'stay',
  },
];

/** Word reveal: each word comes in over `WORD_IN` frames, the next `WORD_STEP` frames later. */
export const WORD_IN = 9;
export const WORD_STEP = 3;
/** A line fades out over this many frames when the camera moves on. */
export const WORD_OUT = 6;

/** The words of a line, as the reveal counts them. */
export const wordsOf = (text: string) => text.split(' ').filter((w) => w.length > 0);

/** The frame each word of each line of a block starts to come in. */
export function wordStarts(block: CaptionBlock): number[][] {
  let carry = 0;
  return block.lines.map((line, i) => {
    const start = line.flow && i > 0 ? carry : line.at;
    const words = wordsOf(line.text);
    const starts = words.map((_, k) => (line.whole || line.already ? line.at : start + k * WORD_STEP));
    carry = starts[starts.length - 1] + WORD_STEP;
    return starts;
  });
}

/** The frame a line is fully in: its last word landed. */
export function lineIn(block: CaptionBlock, i: number): number {
  const line = block.lines[i];
  if (line.already) return line.at;
  const starts = wordStarts(block)[i];
  return starts[starts.length - 1] + (line.whole ? 4 : WORD_IN);
}

/** Something new on screen, for the pace check: the kind says whether it must land on a beat. */
export type EventKind = 'cut' | 'move' | 'whip' | 'pop' | 'caption' | 'click' | 'light' | 'count' | 'ring' | 'swap' | 'word' | 'stream';
export const ON_GRID: EventKind[] = ['cut', 'move', 'whip', 'pop', 'caption', 'click', 'light', 'count'];
export type PaceEvent = {at: number; kind: EventKind; what: string};

const range = (from: number, to: number, step: number) => {
  const out: number[] = [];
  for (let f = from; f < to; f += step) out.push(f);
  return out;
};

/** Every new thing on screen except the words, which `check-pace.mjs` adds from `CAPTIONS`. */
export const EVENTS: PaceEvent[] = [
  // Each word of the prompt as its first key lands: words, so off the grid and not counted as things to take in.
  ...KEYS.filter((k, i) => k.ch !== ' ' && (i === 0 || KEYS[i - 1].ch === ' ')).map((k) => ({at: k.at, kind: 'word' as const, what: 'a word of the prompt typed'})),
  {at: T.hook.enter, kind: 'pop', what: 'Enter: the prompt is sent, the spinner and the thinking line come in'},
  ...T.hook.lines.map((at) => ({at, kind: 'stream' as const, what: 'a tool line, now and then'})),
  {at: T.hook.dim, kind: 'light', what: 'the idle timer dims the screen; the agent still works'},
  {at: T.hook.sleep, kind: 'pop', what: 'the laptop sleeps: the screen goes off, the work stops, the sleeping chip'},
  {at: T.light.on, kind: 'light', what: 'the light comes on'},
  {at: T.agents.tiles, kind: 'pop', what: 'the agent tiles, one after another'},
  {at: T.agents.pulse, kind: 'ring', what: 'the light pulses: it holds for every agent'},
  {at: T.tray.whip, kind: 'whip', what: 'the glyph flies into the tray and dims'},
  {at: T.tray.whip + T.tray.whipLen, kind: 'swap', what: 'the glyph lands on the idle icon'},
  {at: T.tray.start, kind: 'light', what: 'the agent starts: the icon turns holding at the same frame'},
  {at: T.tray.hover, kind: 'pop', what: 'the holding tooltip under the cursor'},
  {at: T.tray.drift, kind: 'move', what: 'the camera drifts toward the tray'},
  {at: T.tray.done, kind: 'light', what: 'the work ends: the icon lets go, the idle tooltip'},
  {at: T.lid.close, kind: 'move', what: 'the lid closes'},
  {at: T.lid.shut, kind: 'light', what: 'lid shut, the edge light breathes'},
  {at: T.lid.lock, kind: 'pop', what: 'lock badge'},
  {at: T.lid.work, kind: 'pop', what: 'the work chip rises: the agent goes on behind the lid'},
  {at: T.lid.done, kind: 'light', what: 'the work is done: the chip says so and the edge light goes off'},
  {at: T.settings.card, kind: 'pop', what: 'settings card'},
  {at: T.settings.row1, kind: 'count', what: 'first value rolls back'},
  {at: T.settings.row2, kind: 'count', what: 'second value rolls back'},
  {at: T.sleeps.sleep, kind: 'light', what: 'the power light fades: the laptop sleeps'},
  {at: T.sleeps.moon, kind: 'pop', what: 'moon'},
  {at: T.end.cut, kind: 'light', what: 'the app icon lights'},
  {at: T.end.line, kind: 'pop', what: 'end line'},
  {at: T.end.speed, kind: 'pop', what: 'speed line'},
  {at: T.end.install, kind: 'pop', what: 'install line'},
];

/** The end card: the gap rule does not apply from here. */
export const END_CARD = SCENES.end.from;

/** Each sound, at its frame. `make-sounds.mjs` makes them; whooshes peak at the move's fastest frame. */
export type SoundName =
  | 'key'
  | 'keySpace'
  | 'keyShift'
  | 'keyReturn'
  | 'room'
  | 'fan'
  | 'powerDown'
  | 'riser'
  | 'enter'
  | 'tick'
  | 'pop'
  | 'whoosh'
  | 'whip'
  | 'click'
  | 'thud'
  | 'lock'
  | 'lightOn'
  | 'lightOff'
  | 'hit'
  | 'lidChime'
  | 'sleepChime'
  | 'settle'
  | 'glow';
/** `variant` picks one of a sound's takes (each key of the prompt sounds a little different). */
export type Cue = {at: number; sound: SoundName; gain?: number; pan?: number; variant?: number};

/**
 * Where a key sits across the keyboard, 0 at the left end to 1 at the right, so each key comes from its own side of
 * the stereo picture as it does for the person typing.
 */
function keyAcross(ch: string): number {
  const rows = ['1234567890', 'qwertyuiop', 'asdfghjkl', 'zxcvbnm'];
  const c = ch.toLowerCase();
  for (const [r, row] of rows.entries()) {
    const i = row.indexOf(c);
    if (i >= 0) return (i + 0.5 + 0.3 * r) / 11;
  }
  return 0.5;
}

export const SOUNDS: Cue[] = [
  // Before the light, no music and no effects: the room, the laptop's fan, the keys, the laptop going to sleep, then a
  // riser into the light.
  {at: 0, sound: 'room'},
  {at: 0, sound: 'fan'},
  ...KEYS.map((k, i) =>
    k.ch === ' ' ? {at: k.at, sound: 'keySpace' as const, gain: 0.9, variant: i} : {at: k.at, sound: 'key' as const, gain: 0.8, pan: 0.38 + 0.24 * keyAcross(k.ch), variant: i},
  ),
  ...(SHIFT_AT === null ? [] : [{at: SHIFT_AT, sound: 'keyShift' as const, gain: 0.5, pan: 0.4}]),
  {at: T.hook.enter, sound: 'keyReturn', gain: 1, pan: 0.62},
  // The screen goes off and the fan winds down (`fan` knows the frame): the laptop's own sound of going to sleep.
  {at: T.hook.sleep, sound: 'powerDown', gain: 0.2},
  // Over the beat before the light; it peaks on the light, where the music comes in.
  {at: T.light.on, sound: 'riser', gain: 0.35},
  {at: T.light.on, sound: 'hit', gain: 0.15},
  {at: T.light.on, sound: 'lightOn', gain: 0.35},
  {at: T.agents.move, sound: 'whoosh', gain: 0.25},
  // A soft tick per tile, at most 8 a second (`promo-video.md`, "Sound"): tiles 5 frames apart are 6 a second.
  ...range(T.agents.tiles, T.agents.tiles + chips().length * T.agents.tileStep, T.agents.tileStep).map((at, i) => ({
    at,
    sound: 'tick' as const,
    gain: 0.16,
    pan: 0.3 + 0.08 * i,
  })),
  {at: T.agents.pulse, sound: 'lightOn', gain: 0.22},
  {at: T.tray.whip, sound: 'whip', gain: 0.5},
  {at: T.tray.start, sound: 'enter', gain: 0.45, pan: 0.4},
  {at: T.tray.start, sound: 'lightOn', gain: 0.3, pan: 0.6},
  {at: T.tray.hover, sound: 'pop', gain: 0.3, pan: 0.6},
  {at: T.tray.drift, sound: 'whoosh', gain: 0.18, pan: 0.6},
  {at: T.tray.done, sound: 'lightOff', gain: 0.3, pan: 0.6},
  {at: T.lid.cut, sound: 'whoosh', gain: 0.35},
  {at: T.lid.shut, sound: 'thud', gain: 0.35},
  {at: T.lid.shut + 2, sound: 'lidChime', gain: 0.85},
  // No lock sound: the badge comes with the lid chime, which sits alone (`promo-video.md`, "Sound").
  {at: T.lid.work, sound: 'pop', gain: 0.22, pan: 0.55},
  {at: T.lid.done, sound: 'lightOff', gain: 0.35},
  {at: T.settings.move, sound: 'whoosh', gain: 0.3, pan: 0.65},
  {at: T.settings.card, sound: 'pop', gain: 0.3, pan: 0.6},
  {at: T.settings.row1, sound: 'tick', gain: 0.3},
  {at: T.settings.row2, sound: 'tick', gain: 0.3},
  // The time cut: a soft whoosh into it, no more.
  {at: T.sleeps.cut, sound: 'whoosh', gain: 0.2, pan: 0.4},
  {at: T.sleeps.sleep, sound: 'sleepChime', gain: 1},
  // The end card is calm: one soft swell of air as its light comes on, no whoosh, no pops; the music lets go.
  {at: T.end.cut, sound: 'glow', gain: 0.3},
];

/** What the music does when (`promo-video.md`, "Sound"). */
export const MUSIC = {
  /**
   * The music starts here, the found track's lift on the light, on a bar line. Before it there is no music at all;
   * from it the track plays at one level to the end card.
   */
  full: T.light.on,
  resolve: T.end.cut,
  /** The code bed only: thinned around the lid chime. */
  thin: [T.lid.cut, T.settings.move] as const,
  /** The code bed only: it lifts once the lid chime has passed, up to the end card. */
  lift: T.settings.move,
};

/** Frames per beat, for the scenes that pulse with it. */
export const BEAT_FRAMES = BEAT;
