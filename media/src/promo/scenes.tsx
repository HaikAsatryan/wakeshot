import React from 'react';
import {AbsoluteFill, Easing, interpolate, useCurrentFrame} from 'remotion';
import settingsData from '../../data/settings.json';
import speedData from '../../data/speed.json';
import {AppIcon} from '../art/Glyph.tsx';
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
 * The app icon over lines of text, centred as a group. The icon is drawn from the port's pixels, so it sits on whole
 * pixels at rest: its top is set, not left to the text's line heights, which are fractional.
 */
const Lockup: React.FC<{icon: number; height: number; iconStyle?: React.CSSProperties; on?: number; children: React.ReactNode}> = ({
  icon,
  height,
  iconStyle,
  on,
  children,
}) => {
  const top = Math.round((VIDEO.height - height) / 2);
  return (
    <AbsoluteFill style={{fontFamily: SANS, color: DARK.text}}>
      <AppIcon size={icon} on={on} style={{position: 'absolute', left: (VIDEO.width - icon) / 2, top, ...iconStyle}} />
      <div style={{position: 'absolute', left: 0, right: 0, top: top + icon, display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
        {children}
      </div>
    </AbsoluteFill>
  );
};

export const Title: React.FC = () => {
  const f = useCurrentFrame();
  const b = BEATS.title;
  // Icon 256, then 28 + 140 (name) + 8 + 56 (tagline).
  return (
    <SceneFade len={SCENES.title.len}>
      <Lockup icon={256} height={256 + 28 + 140 + 8 + 56} on={lightOn(f, b.lightOn)} iconStyle={{transform: `translateY(${ease(f, 0, 18, [30, 0])}px)`}}>
        <div style={{fontSize: 120, lineHeight: '140px', fontWeight: 600, marginTop: 28, opacity: ease(f, 30, 44)}}>{COPY.name}</div>
        <div style={{fontSize: 44, lineHeight: '56px', color: DARK.dim, marginTop: 8, opacity: ease(f, 42, 56)}}>{COPY.tagline}</div>
      </Lockup>
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

export const Outro: React.FC = () => {
  const f = useCurrentFrame();
  // Icon 192, then 24 + 112 (name) + 8 + 48 (line) + 44 + 84 (the install box: 48 + 2 x 16 padding + 2 x 2 border).
  return (
    <AbsoluteFill style={{opacity: ease(f, 0, 10)}}>
      <Lockup icon={192} height={192 + 24 + 112 + 8 + 48 + 44 + 84}>
        <div style={{fontSize: 96, lineHeight: '112px', fontWeight: 600, marginTop: 24}}>{COPY.name}</div>
        <div style={{fontSize: 38, lineHeight: '48px', color: DARK.dim, marginTop: 8}}>{COPY.outroLine}</div>
        <div
          style={{
            fontFamily: MONO,
            fontSize: 36,
            lineHeight: '48px',
            marginTop: 44,
            padding: '16px 32px',
            background: DARK.panel,
            border: `2px solid ${DARK.panelLine}`,
            borderRadius: 12,
            color: DARK.accent,
          }}
        >
          {COPY.install}
        </div>
      </Lockup>
    </AbsoluteFill>
  );
};

