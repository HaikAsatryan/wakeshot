import React from 'react';
import {AbsoluteFill} from 'remotion';

/** The longest smear, in pixels along the move (`promo-video.md`, "Whip"). */
export const MAX_BLUR = 40;
/** Pixels of smear per pixel per frame of speed: a slide of 160 px a frame reaches the cap. */
const PER_SPEED = 0.25;

/** How long the smear is for a speed, in pixels. */
export const blurLength = (vx: number, vy: number) => Math.min(MAX_BLUR, Math.hypot(vx, vy) * PER_SPEED);

/**
 * Blurs its children along the camera's move with an SVG Gaussian blur stretched on the move's axis: cheap, the same
 * pixels on every render, and none at all while the camera rests, so a capture at rest is never touched. A move that
 * is not straight across or down blurs on both axes in proportion; every move in the video is one or the other.
 */
export const DirectionalBlur: React.FC<{vx: number; vy: number; id?: string; children: React.ReactNode}> = ({
  vx,
  vy,
  id = 'camera-blur',
  children,
}) => {
  const length = blurLength(vx, vy);
  if (length < 0.5) return <>{children}</>;
  const speed = Math.hypot(vx, vy);
  // A Gaussian's visible reach is about two deviations each way, so half the length is the deviation.
  const sx = ((Math.abs(vx) / speed) * length) / 2;
  const sy = ((Math.abs(vy) / speed) * length) / 2;
  return (
    <AbsoluteFill>
      <svg width={0} height={0} style={{position: 'absolute'}}>
        <filter id={id} x="-5%" y="-5%" width="110%" height="110%" colorInterpolationFilters="sRGB">
          <feGaussianBlur stdDeviation={`${sx.toFixed(2)} ${sy.toFixed(2)}`} edgeMode="duplicate" />
        </filter>
      </svg>
      <AbsoluteFill style={{filter: `url(#${id})`}}>{children}</AbsoluteFill>
    </AbsoluteFill>
  );
};
