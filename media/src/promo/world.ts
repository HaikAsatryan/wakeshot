// Where each thing sits in the one large world the camera works over, in world pixels (the camera's place for a shot
// puts that place's centre in the middle of the frame, `PLACES` in timeline.ts). Kept apart from the scenes so the
// focus ring, the cursor and the stage's lift can find the same spots.
import {held} from '../art/heldLight.ts';
import {PLACES} from '../timeline.ts';
import {OPEN_TERMINAL} from './AgentTerminal.tsx';
import {SETS, boxOf, type Box, type SetName} from './Capture.tsx';
import {CARD, rowBox} from './SettingsCard.tsx';

/** The pain's flat terminal window: its top left, centred on its place. */
export const OPEN_TERMINAL_AT = {x: PLACES.terminal.x - OPEN_TERMINAL.w / 2, y: PLACES.terminal.y - Math.round(OPEN_TERMINAL.h / 2)};
/** The pain's laptop: the top left of its box; its middle sits at about (hook.x, 189), where the camera looks. */
export const PAIN_LAPTOP_AT = {x: PLACES.hook.x - 416, y: 63};
/** The pain's "Sleeping" chip: its left middle, beside the lid's top corner, in the world so it sinks with the laptop. */
export const SLEEP_CHIP_AT = {x: PLACES.hook.x + 546, y: -163};
/**
 * The tray shot's terminal, the cause: its top left, scale and width (terminal pixels), left of the tray capture, the
 * effect, level with the tooltip. At 0.88 its 34 px text reads at 30 px, the spec's least (review m1 of round 4); it
 * shows the prompt and one status line, so it is narrow enough to end left of the capture after the drift (m5).
 */
export const TRAY_TERMINAL = {x: PLACES.tray.x - 840, y: PLACES.tray.y - 170, scale: 0.88, width: 660};

/** The hero glyph of the light shot: 320 px, its ink box centred across and its middle at `inkMid`. */
export const HERO = (() => {
  const size = 320;
  const h = held(size);
  const inkMid = PLACES.light.y - 170;
  const top = Math.round(inkMid - (h.top + h.lineY + h.t) / 2);
  return {size, left: PLACES.light.x - size / 2, top, inkMid, light: {x: PLACES.light.x, y: top + h.top + h.d / 2}};
})();

/** In the agents shot the hero glyph shrinks to this size as it rises to the top of the frame. */
export const HUB_SIZE = 160;

/** Where a glyph of `size` sits so its ink is centred on (x, inkMid): its top left, and its light's centre. */
export function glyphAt(size: number, x: number, inkMid: number): {left: number; top: number; light: {x: number; y: number}} {
  const h = held(size);
  const top = Math.round(inkMid - (h.top + h.lineY + h.t) / 2);
  const left = Math.round(x - size / 2);
  return {left, top, light: {x: left + size / 2, y: top + h.top + h.d / 2}};
}

/**
 * The agent tiles, in world pixels around the agents place: one row up to three, two rows from four (the top row
 * takes the odd one), centred in the band between the light and the caption, so any count from 1 to 6 lays out with
 * no new design.
 */
export const TILE = {w: 540, h: 150, gapX: 36, gapY: 32};
export function tileBoxes(n: number): Box[] {
  const rows = n <= 3 ? 1 : 2;
  const top = Math.ceil(n / rows);
  const midY = PLACES.agents.y;
  const height = rows * TILE.h + (rows - 1) * TILE.gapY;
  const out: Box[] = [];
  for (let r = 0; r < rows; r++) {
    const count = r === 0 ? top : n - top;
    const width = count * TILE.w + (count - 1) * TILE.gapX;
    for (let c = 0; c < count; c++) {
      out.push({
        x: Math.round(PLACES.agents.x - width / 2 + c * (TILE.w + TILE.gapX)),
        y: Math.round(midY - height / 2 + r * (TILE.h + TILE.gapY)),
        w: TILE.w,
        h: TILE.h,
      });
    }
  }
  return out;
}

/** Captures sit at whole-number zoom; the top left of each set in the world. */
export const SET_AT: Record<SetName, {x: number; y: number; zoom: number}> = {
  // At 2x the 200% capture shows the icon at 64 px and the tooltip's words large; right of the terminal, its
  // taskbar clear of the captions.
  tray: {x: PLACES.tray.x - 228, y: PLACES.tray.y - 210, zoom: 2},
  // The menu set is not in the main cut since 2026-11-06 (its shot made room for the pain and the agents); its spot is
  // kept for the longer cut. At 1x the 200% set fills the frame's height.
  menu: {x: 12000 - 485, y: 1600 - 560, zoom: 1},
};

/** A box of a set, in world pixels; `grow` widens it on every side. */
export function setBox(set: SetName, name: string, grow = 0): Box {
  const at = SET_AT[set];
  const b = boxOf(set, name);
  return {x: at.x + (b.x - grow) * at.zoom, y: at.y + (b.y - grow) * at.zoom, w: (b.w + 2 * grow) * at.zoom, h: (b.h + 2 * grow) * at.zoom};
}

/** The laptop: the top left of its box, drawn a little larger than its own size: as large as the open lid allows above the captions. */
export const LAPTOP_AT = {x: PLACES.lid.x - 420, y: PLACES.lid.y - 256};
export const LAPTOP_SCALE = 1;
/** Above the laptop, where the lock badge and then the moon appear. */
export const ABOVE_LAPTOP = {x: PLACES.lid.x - 10, y: PLACES.lid.y - 215};
/**
 * While the lid is shut and the work goes on, the lock badge and the work chip sit in one row above the laptop,
 * centred on it: the badge's centre and the chip's top left. After the time cut the badge is alone, at `ABOVE_LAPTOP`.
 */
export const WORK_CHIP = {w: 680, h: 76};
export const LID_ROW = (() => {
  const badge = 108;
  const gap = 24;
  const left = ABOVE_LAPTOP.x - (badge + gap + WORK_CHIP.w) / 2;
  return {lock: {x: left + badge / 2, y: ABOVE_LAPTOP.y}, chip: {x: left + badge + gap, y: ABOVE_LAPTOP.y - WORK_CHIP.h / 2}};
})();

/**
 * The settings card's top left: right of the laptop, so the camera slides to it, and clear of the work chip, which
 * stays in view at the frame's left with the work's done line.
 */
export const CARD_AT = {x: PLACES.card.x + 400 - CARD.w / 2, y: PLACES.card.y - 160};
export function cardRow(i: number): Box {
  const r = rowBox(i);
  return {x: CARD_AT.x + r.x, y: CARD_AT.y + r.y, w: r.w, h: r.h};
}
