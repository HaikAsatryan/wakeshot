// The brand's colours: Moss, Haik's pick of 2026-11-05 (`docs/research/brand-directions-2.md`, "Step 2: the colour").
// `scripts/check.mjs` holds every pair in TEXT_PAIRS to WCAG AA and every pair in GRAPHIC_PAIRS to 3:1, so a colour
// change that hurts reading fails the check, not the viewer.
import * as art from './art/heldLight.ts';
import {hex} from './art/motion.ts';

/**
 * The brand itself, from the art port (one source, art.rs): the app icon's plate ramp and its light, and the tray
 * glyph's colours. Only the light takes the brand colour while holding; the line under it keeps the glyph colour.
 */
export const MOSS = {
  plateLight: hex(art.PLATE_LIGHT),
  plateDark: hex(art.PLATE_DARK),
  lightCore: hex(art.LIGHT_CORE),
  lightEdge: hex(art.LIGHT_EDGE),
  holdingOnDark: hex(art.HOLDING_ON_DARK),
  holdingOnLight: hex(art.HOLDING_ON_LIGHT),
  glyphOnDark: hex(art.GLYPH_ON_DARK),
  glyphOnLight: hex(art.GLYPH_ON_LIGHT),
};

/**
 * GitHub's page colour in its default light and dark themes (`--bgColor-default` in github.com's own `light` and
 * `dark` theme CSS, read 2026-11-05; `docs/research.md`), so the loop GIF on the README shows no box around it.
 */
export const GITHUB = {light: '#FFFFFF', dark: '#0D1117'};

/** The video's colours: the video is dark, on the brand's dark page. */
export const DARK = {
  page: '#121610',
  /** The centre of the video's soft background light. */
  pageLift: '#1A2117',
  panel: '#1B2118',
  panelLine: '#2D3628',
  terminal: '#0C0F0B',
  text: '#EEF2E8',
  terminalText: '#E4E9DE',
  dim: '#A3AD99',
  /** Emphasis on dark: the brand's holding colour, the same green the tray's light takes while work runs. */
  accent: MOSS.holdingOnDark,
  /** The draft mark: loud on purpose and outside the brand, so a draft frame is never taken for a finished one. */
  draft: '#B42318',
  onDraft: '#FFFFFF',
};

/**
 * Light theme colours, for what the video shows of Windows in light mode: today only the plain boxes that stand for
 * the light captures until they exist. The brand's light text (`#1A1D17`) on a light panel of the same green-grey.
 */
export const LIGHT = {
  panel: '#F1F4EC',
  panelLine: '#C5CDBA',
  text: '#1A1D17',
  dim: '#5C6655',
};

/** Every text colour on every background it is drawn on in the video: [what, text, background]. */
export const TEXT_PAIRS: [string, string, string][] = [
  ['text on page', DARK.text, DARK.page],
  ['text on the background light', DARK.text, DARK.pageLift],
  ['text on panel', DARK.text, DARK.panel],
  ['dim on page', DARK.dim, DARK.page],
  ['dim on the background light', DARK.dim, DARK.pageLift],
  ['dim on panel', DARK.dim, DARK.panel],
  ['dim on terminal', DARK.dim, DARK.terminal],
  ['terminal text', DARK.terminalText, DARK.terminal],
  ['accent on page', DARK.accent, DARK.page],
  ['accent on panel', DARK.accent, DARK.panel],
  ['accent on terminal', DARK.accent, DARK.terminal],
  ['draft mark', DARK.onDraft, DARK.draft],
  ['light text on light panel', LIGHT.text, LIGHT.panel],
  ['light dim on light panel', LIGHT.dim, LIGHT.panel],
];

/** Shapes that carry meaning, on what they sit (WCAG 1.4.11, 3:1): the loop's glyph on GitHub's pages. */
export const GRAPHIC_PAIRS: [string, string, string][] = [
  ['loop light while holding, dark', MOSS.holdingOnDark, GITHUB.dark],
  ['loop light while holding, light', MOSS.holdingOnLight, GITHUB.light],
  ['loop line and outline, dark', MOSS.glyphOnDark, GITHUB.dark],
  ['loop line and outline, light', MOSS.glyphOnLight, GITHUB.light],
];
