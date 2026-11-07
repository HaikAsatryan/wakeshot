import React, {useLayoutEffect, useRef} from 'react';
import {AbsoluteFill} from 'remotion';
import {DARK} from '../theme.ts';

/** The radial lift's width: a soft light about 900 px across behind what the viewer should look at. */
const LIFT = 900;
/** The dither's strength: up to 4 levels of 255 either way (about 1.5%). */
const DITHER = 4 / 255;

/** A small seeded generator (mulberry32): two million calls of Remotion's string-seeded `random` take too long. */
function seeded(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// One noise picture per render tab, the same on every frame and every render: a fixed seed.
let noise: ImageData | null = null;
function noiseFor(width: number, height: number): ImageData {
  if (noise && noise.width === width && noise.height === height) return noise;
  const next = seeded(1109);
  const data = new Uint8ClampedArray(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    const v = next() * 2 - 1;
    // White for light grain, black for dark, the amount in the alpha: monochrome, centred on no change.
    const c = v > 0 ? 255 : 0;
    data[i * 4] = c;
    data[i * 4 + 1] = c;
    data[i * 4 + 2] = c;
    data[i * 4 + 3] = Math.round(Math.abs(v) * DITHER * 255);
  }
  noise = new ImageData(data, width, height);
  return noise;
}

const Dither: React.FC<{width: number; height: number}> = ({width, height}) => {
  const ref = useRef<HTMLCanvasElement>(null);
  // Drawn before the browser paints, so the frame Remotion captures already holds it.
  useLayoutEffect(() => {
    ref.current?.getContext('2d')?.putImageData(noiseFor(width, height), 0, 0);
  }, [width, height]);
  return <canvas ref={ref} width={width} height={height} style={{position: 'absolute', inset: 0}} />;
};

/**
 * The stage: the dark page, a soft lift behind the focal point (screen pixels, it follows the camera's subject), and a
 * fixed, seeded dither over both so the dark gradient does not band in H.264. The dither does not move with the
 * camera: still grain costs the encoder almost nothing, moving grain costs bits on every frame.
 */
export const Stage: React.FC<{focus: {x: number; y: number}; lift?: number; width: number; height: number}> = ({
  focus,
  lift = 1,
  width,
  height,
}) => (
  <AbsoluteFill style={{background: DARK.page}}>
    <AbsoluteFill
      style={{
        opacity: lift,
        background: `radial-gradient(${LIFT}px ${LIFT * 0.75}px at ${focus.x}px ${focus.y}px, ${DARK.pageLift} 0%, ${DARK.page} 100%)`,
      }}
    />
    <Dither width={width} height={height} />
  </AbsoluteFill>
);
