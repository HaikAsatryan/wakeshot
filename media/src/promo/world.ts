// Where each thing sits in the one large world the camera works over, in world pixels (the camera's place for a shot
// puts that place's centre in the middle of the frame, `PLACES` in timeline.ts). Kept apart from the scenes so the
// focus ring, the cursor and the stage's lift can find the same spots.
import {held} from '../art/heldLight.ts';
import {PLACES} from '../timeline.ts';
import {SETS, boxOf, type Box, type SetName} from './Capture.tsx';
import {CARD, rowBox} from './SettingsCard.tsx';

/** The hook's terminal: its centre, below the big words. */
export const HOOK_TERMINAL = {x: PLACES.hook.x, y: PLACES.hook.y + 150, w: 1300};
/** The tray shot's terminal, the cause: its top left and scale, left of the tray capture, the effect. */
export const TRAY_TERMINAL = {x: PLACES.tray.x - 830, y: PLACES.tray.y - 430, scale: 0.62};

/** The hero glyph of the light shot: 320 px, its ink box centred across and its middle at `inkMid`. */
export const HERO = (() => {
  const size = 320;
  const h = held(size);
  const inkMid = PLACES.hook.y - 170;
  const top = Math.round(inkMid - (h.top + h.lineY + h.t) / 2);
  return {size, left: PLACES.hook.x - size / 2, top, light: {x: PLACES.hook.x, y: top + h.top + h.d / 2}};
})();

/** Captures sit at whole-number zoom; the top left of each set in the world. */
export const SET_AT: Record<SetName, {x: number; y: number; zoom: number}> = {
  // At 2x the 200% capture shows the icon at 64 px and the tooltip's words large; its right side holds the icon.
  tray: {x: PLACES.tray.x + 900 - SETS.tray.w * 2, y: PLACES.tray.y - 320, zoom: 2},
  menu: {x: PLACES.menu.x - 472, y: PLACES.menu.y - 464, zoom: 1},
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

/** The settings card's top left: right of the laptop, so the camera slides to it. */
export const CARD_AT = {x: PLACES.card.x + 250 - CARD.w / 2, y: PLACES.card.y - 160};
export function cardRow(i: number): Box {
  const r = rowBox(i);
  return {x: CARD_AT.x + r.x, y: CARD_AT.y + r.y, w: r.w, h: r.h};
}
