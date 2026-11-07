import React, {useLayoutEffect, useRef} from 'react';
import {appIcon, type GlyphColours, type Rgb} from './heldLight.ts';
import {appIconFade, fadeAt, trayFade, type Fade} from './motion.ts';

/**
 * Straight RGBA pixels drawn 1:1 on a canvas, so the art reaches the frame as the port drew it, with no vector redraw
 * that would anti-alias it differently. Place it on whole pixels: anywhere else the browser resamples it.
 */
export const Pixels: React.FC<{
  width: number;
  height: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
  style?: React.CSSProperties;
}> = ({width, height, rgba, style}) => {
  const ref = useRef<HTMLCanvasElement>(null);
  // Drawn before the browser paints, so the frame Remotion captures already holds the pixels.
  useLayoutEffect(() => {
    const context = ref.current?.getContext('2d');
    if (!context) throw new Error('no 2D canvas');
    context.putImageData(new ImageData(rgba, width, height), 0, 0);
  }, [rgba, width, height]);
  return <canvas ref={ref} width={width} height={height} style={{display: 'block', width, height, ...style}} />;
};

// A 256 px icon takes about a second to work out, so each render tab works out each picture once.
const made = new Map<string, unknown>();
function once<T>(key: string, make: () => T): T {
  if (!made.has(key)) made.set(key, make());
  return made.get(key) as T;
}

const litIcon = (size: number) => once(`icon ${size}`, () => appIcon(size));

/**
 * The tray glyph, the held light, at `size` px on the opaque `page` (or on nothing, `null`), as the tray draws it:
 * `on` 0 is idle (the light as an outline), 1 holding (the light filled in `colours.light`), and in between the light
 * fades on in place.
 */
export const TrayGlyph: React.FC<{size: number; on: number; colours: GlyphColours; page: Rgb | null; style?: React.CSSProperties}> = ({
  size,
  on,
  colours,
  page,
  style,
}) => {
  const fade = once<Fade>(`tray ${size} ${colours.ink} ${colours.light} ${page}`, () => trayFade(size, colours, page));
  return <Pixels width={size} height={size} rgba={fadeAt(fade, on)} style={style} />;
};

/**
 * The app icon at `size` px: `on` 1 (the default) is the app's own icon, `app_icon(size)`; below 1 the light is
 * coming on, from an outline with no glow at 0.
 */
export const AppIcon: React.FC<{size: number; on?: number; style?: React.CSSProperties}> = ({size, on = 1, style}) => {
  const rgba =
    on >= 1
      ? once(`icon pixels ${size}`, () => new Uint8ClampedArray(litIcon(size).pixels))
      : fadeAt(once<Fade>(`icon fade ${size}`, () => appIconFade(size, litIcon(size))), on);
  return <Pixels width={size} height={size} rgba={rgba} style={style} />;
};
