// One timeline for the picture and the sound. The scenes read their frames from here, `scripts/make-sounds.mjs` places
// every sound at the same frames and `scripts/check-pace.mjs` holds it all to the style rules of
// `docs/research/promo-video.md`, so moving a moment moves its sound and its check with it. Plain types only: Node
// runs this file too.
import {BEAT, beat} from './beat.ts';
import {COPY} from './copy.ts';

export const VIDEO = {width: 1920, height: 1080, fps: 30, frames: 900};
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
 * The eight shots of the storyboard (`docs/research/promo-video.md`), in frames. Round 2 (Haik's notes): the speed
 * numbers fold into the end card, the tray shot shows Claude Code turning the icon green and back (5 s), the menu
 * shot has 6 s.
 */
export const SCENES = {
  hook: {from: beat(0), len: beat(4)},
  light: {from: beat(4), len: beat(6)},
  tray: {from: beat(10), len: beat(10)},
  lid: {from: beat(20), len: beat(8)},
  settings: {from: beat(28), len: beat(6)},
  sleeps: {from: beat(34), len: beat(6)},
  menu: {from: beat(40), len: beat(12)},
  end: {from: beat(52), len: beat(8)},
} as const;

/** Every moment the scenes and the sound share, in absolute frames. */
export const T = {
  hook: {
    /** The first tool line; one more every `lineEvery` frames until the PC sleeps. */
    firstLine: 3,
    lineEvery: 3,
    sleep: beat(2),
    /** The stage dims to 10% over this many frames. */
    dimLen: 9,
  },
  light: {on: beat(4), name: beat(5), faintLines: beat(4) + 6},
  /** Claude Code starts and the icon turns holding; the tooltip under the cursor; the work ends and the icon lets go. */
  tray: {whip: beat(10), whipLen: 12, start: beat(11), hover: beat(13), drift: beat(14), done: beat(16)},
  /**
   * The lid shuts, Wakeshot locks (the badge comes with the chime, as the app locks at the lid close) and the work goes
   * on behind it: at `work` a chip rises above the shut laptop with the agent's step under a turning spinner; at `done`
   * it shows the work done and, at that same frame, the edge light goes off: the hold ends (Haik, 2026-11-06 00:56: the
   * work going on, then clearly done, then the sleep). One caption and one chip step, so the shot is not the densest
   * stretch again (review M1 of round 4).
   */
  lid: {cut: beat(20), settle: 15, close: beat(21), shut: beat(22), lock: beat(22), work: beat(24), orbit: beat(24), done: beat(27)},
  settings: {move: beat(28), card: beat(29), row1: beat(31), row2: beat(32)},
  /**
   * A time cut: the frame goes to black into the cut, "Later" lands on the black, and the picture comes back under the
   * rest of the line; then the laptop goes to sleep. `dip` holds the frames where the black starts, is full, starts to
   * lift and is gone: 8 frames going down, 7 black (the cut in the middle), 9 coming back, 0.8 s in all, and the
   * picture is fully back on the next beat. Black, not a dim: the settings shot before it is
   * already dimmed to 35% by the focus, and the old 0.2 s dip to 85% read as a dropped picture, not as time passing
   * (Haik, 2026-11-06). No clock and no countdown on screen: Wakeshot sleeps a closed laptop once nothing has happened
   * for 5 minutes (`docs/product.md`), and the video claims no time it does not keep.
   */
  sleeps: {cut: beat(34), dip: [beat(34) - 10, beat(34) - 1, beat(34) + 5, beat(35)], line: beat(34), sleep: beat(36), moon: beat(37)},
  menu: {
    cut: beat(40),
    click1: beat(41),
    keep: beat(43),
    hour: beat(46),
    pan: beat(46),
    click2: beat(48),
    ring: beat(48) + 3,
  },
  end: {cut: beat(52), line: beat(53), speed: beat(54), install: beat(55)},
} as const;

/** Camera moves between near places run this long; a whip is quicker (`promo-video.md`, "Motion"). */
export const MOVE_LEN = 15;
export const PAN_LEN = 12;

/**
 * The places in the one large world the camera works over, centre and zoom. Near places sit side by side, so the
 * camera slides between them; far places are reached by a hard cut.
 */
export const PLACES = {
  hook: {x: 0, y: 0, zoom: 1},
  tray: {x: 0, y: 1600, zoom: 1},
  trayDrift: {x: 90, y: 1620, zoom: 1},
  lid: {x: 6000, y: 0, zoom: 1},
  lidNear: {x: 6000, y: 0, zoom: 1.06},
  lidPush: {x: 6000, y: -20, zoom: 1.03},
  // A little wider than the lid's place (drawn parts only, so a zoom under 1 is allowed): the laptop, the badge, the
  // work chip and the card all fit without a cut edge.
  card: {x: 6500, y: 0, zoom: 0.92},
  menu: {x: 12000, y: 1600, zoom: 1},
  menuSub: {x: 11850, y: 1600, zoom: 1},
  end: {x: 20000, y: 0, zoom: 1},
} as const;
export type PlaceName = keyof typeof PLACES;

/**
 * The camera: from each key's frame it goes to `to`, at once (`cut`) or over `len` frames. A cut, a move and a whip
 * each start a new framing; a settle is the same framing coming to rest.
 */
export type CameraKey = {at: number; to: PlaceName; kind: 'cut' | 'move' | 'whip' | 'settle'; len: number};
export const CAMERA: CameraKey[] = [
  {at: SCENES.hook.from, to: 'hook', kind: 'cut', len: 0},
  {at: SCENES.light.from, to: 'hook', kind: 'cut', len: 0},
  {at: T.tray.whip, to: 'tray', kind: 'whip', len: T.tray.whipLen},
  {at: T.tray.drift, to: 'trayDrift', kind: 'move', len: MOVE_LEN},
  {at: T.lid.cut, to: 'lidNear', kind: 'cut', len: 0},
  {at: T.lid.cut, to: 'lid', kind: 'settle', len: T.lid.settle},
  {at: T.settings.move, to: 'card', kind: 'move', len: MOVE_LEN},
  {at: T.sleeps.cut, to: 'lid', kind: 'cut', len: 0},
  // The slow push of a hold on drawn parts (`promo-video.md`, "Motion"): no capture is in view here.
  {at: T.sleeps.cut, to: 'lidPush', kind: 'settle', len: SCENES.sleeps.len},
  {at: T.menu.cut, to: 'menu', kind: 'cut', len: 0},
  {at: T.menu.pan, to: 'menuSub', kind: 'move', len: PAN_LEN},
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
  {at: T.menu.ring, target: 'menuIcon', dim: false},
  {at: T.end.cut, target: null, cut: true},
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
  {
    id: 'hook',
    lines: [
      {text: COPY.hook[0], at: 0, style: 'big', already: true},
      {text: COPY.hook[1], at: T.hook.sleep, style: 'big'},
    ],
    until: SCENES.light.from,
    leave: 'cut',
  },
  {
    id: 'name',
    lines: [
      {text: COPY.name, at: T.light.name, style: 'name'},
      {text: COPY.tagline, at: T.light.name, style: 'caption', flow: true},
    ],
    until: T.tray.whip,
    leave: 'fade',
  },
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
    until: T.menu.cut,
    leave: 'cut',
  },
  {
    id: 'menu',
    lines: [
      {text: COPY.menu[0], at: T.menu.keep, style: 'caption'},
      {text: COPY.menu[1], at: T.menu.click2, style: 'caption'},
    ],
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
  ...range(T.hook.firstLine, T.hook.sleep, T.hook.lineEvery).map((at) => ({at, kind: 'stream' as const, what: 'tool line'})),
  {at: T.hook.sleep, kind: 'pop', what: 'sleeping chip; the stage dims'},
  {at: T.light.on, kind: 'light', what: 'the light comes on'},
  ...range(T.light.faintLines, T.tray.whip, 6).map((at) => ({at, kind: 'stream' as const, what: 'faint terminal line'})),
  {at: T.tray.whip, kind: 'whip', what: 'the glyph flies into the tray and dims'},
  {at: T.tray.whip + T.tray.whipLen, kind: 'swap', what: 'the glyph lands on the idle icon'},
  {at: T.tray.start, kind: 'light', what: 'Claude Code starts: the icon turns holding at the same frame'},
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
  {at: T.menu.cut + 1, kind: 'stream', what: 'cursor glides in'},
  {at: T.menu.click1, kind: 'click', what: 'click on the icon'},
  {at: T.menu.click1, kind: 'pop', what: 'the menu'},
  {at: T.menu.keep, kind: 'pop', what: 'Keep awake hovered, submenu'},
  {at: T.menu.hour, kind: 'swap', what: '1 hour hovered'},
  {at: T.menu.click2, kind: 'click', what: 'click on 1 hour, the icon holds'},
  {at: T.menu.ring, kind: 'ring', what: 'ring on the icon; a lens rises with the holding glyph'},
  {at: T.end.cut, kind: 'light', what: 'the app icon lights'},
  {at: T.end.line, kind: 'pop', what: 'end line'},
  {at: T.end.speed, kind: 'pop', what: 'speed line'},
  {at: T.end.install, kind: 'pop', what: 'install line'},
];

/** The end card: the gap rule does not apply from here. */
export const END_CARD = SCENES.end.from;

/** Each sound, at its frame. `make-sounds.mjs` makes them; whooshes peak at the move's fastest frame. */
export type SoundName =
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
export type Cue = {at: number; sound: SoundName; gain?: number; pan?: number};

export const SOUNDS: Cue[] = [
  {at: 0, sound: 'enter', gain: 0.5},
  ...range(T.hook.firstLine, T.hook.sleep, T.hook.lineEvery).map((at) => ({at, sound: 'tick' as const, gain: 0.08, pan: 0.42})),
  {at: T.hook.sleep, sound: 'pop', gain: 0.35, pan: 0.7},
  {at: T.light.on, sound: 'hit', gain: 0.15},
  {at: T.light.on, sound: 'lightOn', gain: 0.35},
  {at: T.tray.whip, sound: 'whip', gain: 0.5},
  {at: T.tray.start, sound: 'enter', gain: 0.45, pan: 0.4},
  {at: T.tray.start, sound: 'lightOn', gain: 0.3, pan: 0.6},
  {at: T.tray.hover, sound: 'pop', gain: 0.3, pan: 0.6},
  {at: T.tray.drift, sound: 'whoosh', gain: 0.18, pan: 0.6},
  {at: T.tray.done, sound: 'lightOff', gain: 0.3, pan: 0.6},
  {at: T.lid.cut, sound: 'whoosh', gain: 0.35},
  {at: T.lid.shut, sound: 'thud', gain: 0.5},
  {at: T.lid.shut + 2, sound: 'lidChime', gain: 1},
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
  {at: T.menu.cut, sound: 'whoosh', gain: 0.35},
  {at: T.menu.click1, sound: 'click', gain: 0.5, pan: 0.6},
  {at: T.menu.click1, sound: 'pop', gain: 0.22, pan: 0.6},
  {at: T.menu.keep, sound: 'pop', gain: 0.18, pan: 0.45},
  {at: T.menu.pan, sound: 'whoosh', gain: 0.18, pan: 0.4},
  {at: T.menu.click2, sound: 'click', gain: 0.5, pan: 0.45},
  {at: T.menu.ring, sound: 'lightOn', gain: 0.22, pan: 0.55},
  // The end card is calm: one soft swell of air as its light comes on, no whoosh, no pops; the music lets go.
  {at: T.end.cut, sound: 'glow', gain: 0.3},
];

/** What the music does when (`promo-video.md`, "Sound"); `make-sounds.mjs` builds the code bed from it. */
export const MUSIC = {
  /** Sparse until the PC sleeps; then it falls in pitch to a low hum; silent just before the light. */
  sparseTo: T.hook.sleep,
  silentFrom: 56,
  full: T.light.on,
  /** Thinned around the lid chime, so it sits alone. */
  thin: [T.lid.cut, T.settings.move] as const,
  lift: T.menu.cut,
  resolve: T.end.cut,
};

/** Frames per beat, for the scenes that pulse with it. */
export const BEAT_FRAMES = BEAT;
