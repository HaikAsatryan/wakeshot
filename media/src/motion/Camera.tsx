import React from 'react';
import {AbsoluteFill} from 'remotion';
import {DirectionalBlur} from './DirectionalBlur.tsx';
import {SETTLE, progress} from './ease.ts';

export type View = {x: number; y: number; zoom: number};
export type Key<P extends string> = {at: number; to: P; kind: 'cut' | 'move' | 'whip' | 'settle'; len: number};

/** Where the camera is at frame `f`: each key eases from where the camera was to its place. */
export function viewAt<P extends string>(keys: Key<P>[], places: Record<P, View>, f: number): View {
  let view: View = places[keys[0].to];
  for (const k of keys) {
    if (f < k.at) break;
    const to = places[k.to];
    const t = progress(f, k.at, k.len, SETTLE);
    view = {x: view.x + (to.x - view.x) * t, y: view.y + (to.y - view.y) * t, zoom: view.zoom + (to.zoom - view.zoom) * t};
  }
  return view;
}

/**
 * The camera's speed on screen, in pixels per frame, from half a frame before to half a frame after `f`. A cut has
 * no speed: the frames on each side of it are two different shots.
 */
export function velocityAt<P extends string>(keys: Key<P>[], places: Record<P, View>, f: number): {vx: number; vy: number} {
  if (keys.some((k) => k.kind === 'cut' && k.at > f - 0.5 && k.at <= f + 0.5)) return {vx: 0, vy: 0};
  const a = viewAt(keys, places, f - 0.5);
  const b = viewAt(keys, places, f + 0.5);
  // Only the slide blurs: a zoom of a few percent moves the frame's edges a few pixels, too little to smear.
  return {vx: (a.x - b.x) * b.zoom, vy: (a.y - b.y) * b.zoom};
}

/** World point to screen point for a view. */
export const toScreen = (view: View, size: {width: number; height: number}, x: number, y: number) => ({
  x: size.width / 2 + (x - view.x) * view.zoom,
  y: size.height / 2 + (y - view.y) * view.zoom,
});

/**
 * The world under a fixed frame: it slides and scales so the view's centre sits in the middle. At zoom 1 the offset
 * is rounded to whole pixels, so a capture or the art's pixels land on the screen's pixels and stay sharp. While the
 * camera moves fast, the world blurs along the move.
 */
export const World: React.FC<{
  view: View;
  velocity: {vx: number; vy: number};
  size: {width: number; height: number};
  children: React.ReactNode;
}> = ({view, velocity, size, children}) => {
  const whole = Math.abs(view.zoom - Math.round(view.zoom)) < 1e-6;
  let tx = size.width / 2 - view.x * view.zoom;
  let ty = size.height / 2 - view.y * view.zoom;
  if (whole) {
    tx = Math.round(tx);
    ty = Math.round(ty);
  }
  return (
    <DirectionalBlur vx={velocity.vx} vy={velocity.vy}>
      <AbsoluteFill style={{overflow: 'hidden'}}>
        <div
          style={{
            position: 'absolute',
            left: 0,
            top: 0,
            transformOrigin: '0 0',
            transform: `translate(${tx}px, ${ty}px) scale(${view.zoom})`,
          }}
        >
          {children}
        </div>
      </AbsoluteFill>
    </DirectionalBlur>
  );
};

/** A thing placed in the world, centred on (x, y) unless `corner` says its top left goes there. */
export const At: React.FC<{x: number; y: number; corner?: boolean; children: React.ReactNode; style?: React.CSSProperties}> = ({
  x,
  y,
  corner,
  children,
  style,
}) => (
  <div style={{position: 'absolute', left: x, top: y, transform: corner ? undefined : 'translate(-50%, -50%)', ...style}}>
    {children}
  </div>
);
