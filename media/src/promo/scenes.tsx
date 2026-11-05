import React from 'react';
import {AbsoluteFill, Easing, interpolate, useCurrentFrame} from 'remotion';
import settingsData from '../../data/settings.json';
import speedData from '../../data/speed.json';
import {AppIcon} from '../art/Glyph.tsx';
import {plated} from '../art/heldLight.ts';
import {COPY} from '../copy.ts';
import {MONO, SANS} from '../fonts.ts';
import {DARK} from '../theme.ts';
import {BEATS, LIGHT_ON_FRAMES, SCENES, VIDEO} from '../timeline.ts';
import {Caption, DraftContext, Laptop, Shot, Terminal, ease} from './parts.tsx';

/** Fades a scene in and out over a few frames, so cuts are soft. */
export const SceneFade: React.FC<{len: number; children: React.ReactNode}> = ({len, children}) => {
  const f = useCurrentFrame();
  const o = Math.min(ease(f, 0, 8), ease(f, len - 8, len, [1, 0]));
  return <AbsoluteFill style={{opacity: o}}>{children}</AbsoluteFill>;
};

/** The light coming on from `from`: quick and settling, as a lamp does, the same curve as the README loop. */
const lightOn = (f: number, from: number) =>
  interpolate(f, [from, from + LIGHT_ON_FRAMES], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
    easing: Easing.out(Easing.cubic),
  });

const center: React.CSSProperties = {alignItems: 'center', justifyContent: 'center'};

/**
 * The stacked lockup, the app icon over the name (`docs/brand.md`), in sides of the plate, so the title and the outro
 * are one lockup at two sizes: the name's cap height 0.36 (the name set at half the plate's side), 0.26 from the
 * plate's bottom to the name's cap line (the row lockup's gap), the name's ink centred under the plate.
 */
const STACKED = {cap: 0.36, gap: 0.26};

/** What follows the lockup, in sides of the plate: a line's cap line under the baseline above it, a box's top. */
const BELOW = {line: 0.2, box: 0.3};

/** Instrument Sans: cap height 720 of 1000 units per em (OS/2 `sCapHeight`). */
const CAP_EM = 0.72;

/**
 * How far right of its box's centre the name's ink sits, in em: "W" starts 0.039 em into its advance and "t" ends
 * 0.017 em before the box's end (0.027 em of its own, less the -0.01 em tracking the browser adds after it).
 * Measured in the frames: the ink's centre within a quarter of a pixel of the plate's (it was 0.9 px right).
 */
const NAME_INK_RIGHT = 0.011;

/** A part under the icon, top to bottom: a text line (its cap height tall) or a box, and the space above it. */
type Part = {gap: number; height: number; text: boolean};

/**
 * Where the icon and each part go: centred in the frame from the plate's top to the last part's bottom. The icon's
 * top is a whole pixel, since the icon is the port's pixels drawn 1:1; a text line's baseline is a whole pixel, where
 * the browser draws it anyway, so the spaces stay as set.
 */
function stack(icon: number, parts: Part[]): {iconTop: number; tops: number[]} {
  const p = plated(icon, true);
  const total = p.side + parts.reduce((sum, q) => sum + q.gap + q.height, 0);
  const iconTop = Math.round((VIDEO.height - total) / 2 - p.margin);
  let y = iconTop + p.margin + p.side;
  const tops = parts.map((q) => {
    const top = q.text ? Math.round(y + q.gap + q.height) - q.height : Math.round(y + q.gap);
    y = top + q.height;
    return top;
  });
  return {iconTop, tops};
}

/** The name's place under a plate of `side` px: the stacked lockup's one part below the icon. */
const nameOf = (side: number): Part => ({gap: STACKED.gap * side, height: STACKED.cap * side, text: true});

/** A text line by its cap line and baseline: `text-box` trims the line box to them, so `top` is the cap line. */
const Line: React.FC<{top: number; size: number; style?: React.CSSProperties; children: React.ReactNode}> = ({top, size, style, children}) => (
  <div
    style={{
      position: 'absolute',
      left: 0,
      right: 0,
      top,
      fontSize: size,
      textAlign: 'center',
      whiteSpace: 'nowrap',
      textBox: 'trim-both cap alphabetic',
      ...style,
    }}
  >
    {children}
  </div>
);

/** The name in the stacked lockup: the wordmark (SemiBold, -0.01 em), its ink centred. */
const Name: React.FC<{top: number; side: number; style?: React.CSSProperties}> = ({top, side, style}) => (
  <Line
    top={top}
    size={(STACKED.cap * side) / CAP_EM}
    style={{fontWeight: 600, letterSpacing: '-0.01em', left: `${-NAME_INK_RIGHT}em`, right: `${NAME_INK_RIGHT}em`, ...style}}
  >
    {COPY.name}
  </Line>
);

const TAGLINE = 44;

export const Title: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.title;
  const icon = 256;
  const side = plated(icon, true).side;
  const {iconTop, tops} = stack(icon, [nameOf(side), {gap: BELOW.line * side, height: CAP_EM * TAGLINE, text: true}]);
  return (
    <SceneFade len={SCENES.title.len}>
      <AbsoluteFill style={{fontFamily: SANS, color: DARK.text}}>
        <AppIcon
          size={icon}
          on={lightOn(f, b.lightOn)}
          style={{position: 'absolute', left: (VIDEO.width - icon) / 2, top: iconTop, transform: `translateY(${ease(f, 0, 18, [30, 0])}px)`}}
        />
        <Name top={tops[0]} side={side} style={{opacity: ease(f, 30, 44)}} />
        <Line top={tops[1]} size={TAGLINE} style={{color: DARK.dim, opacity: ease(f, 42, 56)}}>
          {COPY.tagline}
        </Line>
      </AbsoluteFill>
    </SceneFade>
  );
};

/** The tray crop at 600 px wide; the icon swaps from one state's capture to the other's at `swap`. */
const Tray: React.FC<{from: 'idle' | 'holding'; swap: number}> = ({from, swap}) => {
  const f = useCurrentFrame();
  const t = ease(f, swap, swap + 8);
  const [a, b] = from === 'idle' ? ['tray-dark-idle', 'tray-dark-holding'] : ['tray-dark-holding', 'tray-dark-idle'];
  return (
    <div style={{position: 'relative'}}>
      <Shot id={a} width={TRAY_W} style={{position: 'absolute', bottom: 0, opacity: 1 - t}} />
      <Shot id={b} width={TRAY_W} style={{position: 'absolute', bottom: 0, opacity: t}} />
    </div>
  );
};

const TRAY_W = 600;
/** The tray area's bottom edge on screen: the tray crop and the tooltip capture (which shows the tray too) share it. */
const TRAY_BOTTOM = 860;

/** The tray, then the tooltip capture over it from `tip`, as when the pointer rests on the icon. */
const TrayWithTip: React.FC<{from: 'idle' | 'holding'; swap: number; tip: number; tipShot: string}> = ({
  from,
  swap,
  tip,
  tipShot,
}) => {
  const f = useCurrentFrame();
  const t = ease(f, tip, tip + 10);
  return (
    <div style={{position: 'absolute', left: 1180, top: TRAY_BOTTOM, width: TRAY_W}}>
      <div style={{opacity: 1 - t}}>
        <Tray from={from} swap={swap} />
      </div>
      <Shot id={tipShot} width={TRAY_W} style={{position: 'absolute', bottom: 0, opacity: t}} />
    </div>
  );
};

export const WorkStarts: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.start;
  return (
    <SceneFade len={SCENES.start.len}>
      <div style={{position: 'absolute', left: 100, top: 150}}>
        <Terminal typeCommand={b.typeCommand} typePrompt={b.typePrompt} enter={b.enter} />
      </div>
      <div style={{opacity: ease(f, 40, 56)}}>
        <TrayWithTip from="idle" swap={b.morph} tip={b.tooltip} tipShot="tooltip-dark-holding" />
      </div>
      <Caption text={COPY.start} from={b.morph} />
    </SceneFade>
  );
};

export const LidClosed: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.lid;
  return (
    <SceneFade len={SCENES.lid.len}>
      <AbsoluteFill style={{...center, paddingBottom: 160}}>
        <Laptop closed={ease(f, b.close, b.shut)} lock={ease(f, b.lock, b.lock + 10)} />
      </AbsoluteFill>
      <Caption text={COPY.lid} from={8} />
    </SceneFade>
  );
};

export const WorkEnds: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.end;
  const first = ease(f, 66, 76, [1, 0]);
  const second = ease(f, 70, 80);
  return (
    <SceneFade len={SCENES.end.len}>
      <div style={{opacity: first}}>
        <div style={{position: 'absolute', left: 100, top: 150}}>
          <Terminal typeCommand={-100} typePrompt={-80} enter={-20} done={b.done} />
        </div>
        <TrayWithTip from="holding" swap={b.morph} tip={b.morph + 14} tipShot="tooltip-dark-idle" />
      </div>
      <AbsoluteFill style={{...center, paddingBottom: 220, opacity: second}}>
        <Laptop closed={1} lock={0} moon={ease(f, b.sleepChime, b.sleepChime + 14)} />
      </AbsoluteFill>
      <Caption text={COPY.end} from={b.morph} small={COPY.endSmall} smallFrom={76} />
    </SceneFade>
  );
};

export const SettingsBack: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.settings;
  const ticks = [b.tick1, b.tick2];
  return (
    <SceneFade len={SCENES.settings.len}>
      <AbsoluteFill style={{...center, paddingBottom: 160}}>
        <div
          style={{
            width: 1000,
            background: DARK.panel,
            border: `2px solid ${DARK.panelLine}`,
            borderRadius: 20,
            padding: '24px 48px',
            fontFamily: SANS,
          }}
        >
          {settingsData.rows.map((row, i) => {
            const t = ease(f, ticks[i], ticks[i] + 10);
            return (
              <div
                key={row.label}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  height: 120,
                  borderTop: i > 0 ? `2px solid ${DARK.panelLine}` : undefined,
                }}
              >
                <div style={{fontSize: 46, color: DARK.text}}>{row.label}</div>
                <div style={{position: 'relative', width: 420, height: 100, overflow: 'hidden', textAlign: 'right'}}>
                  {[
                    {text: row.held, note: COPY.settingsHeld, colour: DARK.accent, y: -t * 100},
                    {text: row.own, note: COPY.settingsOwn, colour: DARK.text, y: (1 - t) * 100},
                  ].map((v) => (
                    <div key={v.note} style={{position: 'absolute', right: 0, top: 6, transform: `translateY(${v.y}px)`}}>
                      <div style={{fontSize: 46, fontWeight: 600, color: v.colour}}>{v.text}</div>
                      <div style={{fontSize: 24, color: DARK.dim}}>{v.note}</div>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </AbsoluteFill>
      <Caption text={COPY.settings} from={b.tick1} />
    </SceneFade>
  );
};

export const KeepAwake: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.keepAwake;
  const sub = ease(f, b.click1, b.click1 + 8);
  const split = ease(f, b.swap, b.swap + 14, [100, 50]);
  const W = 1400;
  const H = (W / 1144) * 564;
  const menus = (theme: 'light' | 'dark') => (
    <div style={{position: 'relative', width: W, height: H}}>
      <Shot id={`menu-${theme}-main`} width={(W / 1144) * 904} style={{position: 'absolute', right: 0, bottom: 0, opacity: 1 - sub}} />
      <Shot id={`menu-${theme}-sub`} width={W} style={{position: 'absolute', left: 0, top: 0, opacity: sub}} />
    </div>
  );
  return (
    <SceneFade len={SCENES.keepAwake.len}>
      <div style={{position: 'absolute', left: (1920 - W) / 2, top: 70}}>
        {menus('light')}
        <div style={{position: 'absolute', inset: 0, clipPath: `inset(0 0 0 ${split}%)`}}>{menus('dark')}</div>
        {split < 100 ? (
          <div style={{position: 'absolute', top: -12, bottom: -12, left: `${split}%`, width: 4, background: DARK.accent, borderRadius: 2}} />
        ) : null}
      </div>
      <Caption text={COPY.keepAwake} from={10} />
    </SceneFade>
  );
};

export const Speed: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.speed;
  const draft = React.useContext(DraftContext);
  const s = speedData as Record<string, number | string | boolean | null>;
  const items: [number | null, string, number, number][] = [
    [s.idleRamMb as number | null, COPY.speedLabels.ram, b.tick1, 1],
    [s.idleCpuMsPer10Min as number | null, COPY.speedLabels.cpu, b.tick2, 0],
    [s.startDelayS as number | null, COPY.speedLabels.delay, b.tick3, 1],
  ];
  return (
    <SceneFade len={SCENES.speed.len}>
      <AbsoluteFill style={{...center, flexDirection: 'column', fontFamily: SANS, color: DARK.text}}>
        <div style={{fontSize: 64, fontWeight: 600, marginBottom: 70}}>{COPY.speedTitle}</div>
        <div style={{display: 'flex', gap: 60}}>
          {items.map(([value, label, beat, digits]) => {
            const t = ease(f, beat, beat + 18);
            return (
              <div key={label} style={{width: 520, textAlign: 'center', opacity: ease(f, beat - 4, beat + 4)}}>
                <div style={{fontSize: 128, fontWeight: 600, color: DARK.accent, fontVariantNumeric: 'tabular-nums'}}>
                  {value === null ? '?' : (value * t).toFixed(digits)}
                </div>
                <div style={{fontSize: 30, color: DARK.dim, marginTop: 8}}>{label}</div>
              </div>
            );
          })}
        </div>
        <div style={{fontSize: 26, color: DARK.dim, marginTop: 70, opacity: ease(f, 40, 52)}}>
          {s.measured ? `Measured on Wakeshot ${s.build}, ${s.date}, x64.` : draft ? COPY.speedNotMeasured : ''}
        </div>
      </AbsoluteFill>
    </SceneFade>
  );
};

const OUTRO_LINE = 38;
/** The install box's height: a 48 px line, 16 px padding and a 2 px border above and below. */
const INSTALL_BOX = 48 + 2 * 16 + 2 * 2;

export const Outro: React.FC = () => {
  const f = useCurrentFrame();
  const icon = 192;
  const side = plated(icon, true).side;
  const {iconTop, tops} = stack(icon, [
    nameOf(side),
    {gap: BELOW.line * side, height: CAP_EM * OUTRO_LINE, text: true},
    {gap: BELOW.box * side, height: INSTALL_BOX, text: false},
  ]);
  return (
    <AbsoluteFill style={{opacity: ease(f, 0, 10), fontFamily: SANS, color: DARK.text}}>
      <AppIcon size={icon} style={{position: 'absolute', left: (VIDEO.width - icon) / 2, top: iconTop}} />
      <Name top={tops[0]} side={side} />
      <Line top={tops[1]} size={OUTRO_LINE} style={{color: DARK.dim}}>
        {COPY.outroLine}
      </Line>
      <div style={{position: 'absolute', left: 0, right: 0, top: tops[2], display: 'flex', justifyContent: 'center'}}>
        <div
          style={{
            fontFamily: MONO,
            fontSize: 36,
            lineHeight: '48px',
            padding: '16px 32px',
            background: DARK.panel,
            border: `2px solid ${DARK.panelLine}`,
            borderRadius: 12,
            color: DARK.accent,
          }}
        >
          {COPY.install}
        </div>
      </div>
    </AbsoluteFill>
  );
};

