// One timeline for the picture and the sound: the scenes read their frames from here and `scripts/make-sounds.mjs`
// places every sound at the same frames, so moving a scene moves its sounds with it.

export const VIDEO = {width: 1920, height: 1080, fps: 30, frames: 900};
/**
 * The README loop. `art` is the tray glyph's size in it: `tray_mask` drawn at 128 px, not scaled from a smaller one;
 * the README shows the loop at half size, so the glyph reads at 64 px and stays sharp on a HiDPI screen.
 */
export const LOOP = {width: 480, height: 240, fps: 25, frames: 125, art: 128};

/** Scene starts and lengths in video frames (30 fps). The storyboard is `docs/research/launch.md` part 4. */
export const SCENES = {
  title: {from: 0, len: 90},
  start: {from: 90, len: 150},
  lid: {from: 240, len: 120},
  end: {from: 360, len: 150},
  settings: {from: 510, len: 120},
  keepAwake: {from: 630, len: 120},
  speed: {from: 750, len: 90},
  outro: {from: 840, len: 60},
} as const;

/** Frames inside each scene where something happens that has a sound. */
export const BEATS = {
  title: {lightOn: 20},
  start: {typeCommand: 14, typePrompt: 34, enter: 76, morph: 84, tooltip: 98},
  lid: {close: 16, shut: 34, lock: 46},
  end: {done: 10, morph: 22, sleepChime: 78},
  settings: {tick1: 40, tick2: 58},
  keepAwake: {click1: 20, swap: 50, click2: 70},
  speed: {tick1: 10, tick2: 22, tick3: 34},
  outro: {resolve: 0},
} as const;

/** How long the light takes to come on in the video: 0.44 s, as in the README loop (11 frames at 25 fps). */
export const LIGHT_ON_FRAMES = 13;

/** What the terminal card types. An illustration of a Claude Code session, not a copy of its screen. */
export const TERMINAL = {
  path: 'PS C:\\code\\shop>',
  command: 'claude',
  prompt: 'Fix the failing tests and run the suite',
  /** Frames per typed character. */
  commandRate: 2,
  promptRate: 1,
};

/** The absolute frame of a beat. */
export function at<S extends keyof typeof BEATS>(scene: S, beat: keyof (typeof BEATS)[S]): number {
  return SCENES[scene].from + (BEATS[scene][beat] as number);
}
