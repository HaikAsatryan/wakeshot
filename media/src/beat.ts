// The video's beat grid. Cuts, camera moves, pops and caption starts land on it, so the picture and the music agree;
// `scripts/check-pace.mjs` holds the timeline to it. Plain types only: Node runs this file too.

export const FPS = 30;
export const BPM = 120;
/** Frames per beat: 15 at 120 BPM and 30 fps. */
export const BEAT = (60 / BPM) * FPS;
/** Frames per bar of four beats. */
export const BAR = 4 * BEAT;

/** The first frame of beat `n` (0 is the video's first frame). */
export const beat = (n: number) => Math.round(n * BEAT);
/** The first frame of bar `n`. */
export const bar = (n: number) => Math.round(n * BAR);
