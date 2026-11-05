import React from 'react';
import {AbsoluteFill, Easing, staticFile, useCurrentFrame} from 'remotion';
import {Audio} from '@remotion/media';
import speedData from '../../data/speed.json';
import {AppIcon, TrayGlyph} from '../art/Glyph.tsx';
import {ON_DARK, plated} from '../art/heldLight.ts';
import {COPY} from '../copy.ts';
import {MONO, SANS} from '../fonts.ts';
import {At, World, toScreen, velocityAt, viewAt, type View} from '../motion/Camera.tsx';
import {Cursor, cursorAt, type CursorMove} from '../motion/Cursor.tsx';
import {DirectionalBlur} from '../motion/DirectionalBlur.tsx';
import {LAMP_OFF, LAMP_ON, SETTLE, progress} from '../motion/ease.ts';
import {Focus, focusAt, type Rect} from '../motion/Focus.tsx';
import {Pop} from '../motion/Pop.tsx';
import {Stage} from '../motion/Stage.tsx';
import {CAPTION_BASELINE, CAPTION_STEP, STYLE, Words} from '../motion/Words.tsx';
import {DARK, MOSS} from '../theme.ts';
import {CAMERA, CAPTIONS, FOCUS, LIGHT_OFF_FRAMES, LIGHT_ON_FRAMES, PLACES, SCENES, T, VIDEO, type PlaceName} from '../timeline.ts';
import {Capture, SETS, StandInTag} from './Capture.tsx';
import {DraftContext} from './draft.ts';
import {LaptopTilt} from './LaptopTilt.tsx';
import {SettingsCard} from './SettingsCard.tsx';
import {Terminal} from './Terminal.tsx';
import {ABOVE_LAPTOP, CARD_AT, HERO, HOOK_TERMINAL, LAPTOP_AT, LAPTOP_SCALE, SET_AT, TRAY_TERMINAL, cardRow, setBox} from './world.ts';

export type PromoProps = {draft: boolean};

const SIZE = {width: VIDEO.width, height: VIDEO.height};
const within = (f: number, from: number, to: number) => f >= from && f < to;
const centre = (b: Rect) => ({x: b.x + b.w / 2, y: b.y + b.h / 2});

/** Where the stage's soft lift sits in each shot, in world pixels: behind what the viewer should look at. */
function liftAt(f: number): {x: number; y: number} {
  if (f < SCENES.tray.from) return f < SCENES.light.from ? {x: HOOK_TERMINAL.x, y: HOOK_TERMINAL.y} : HERO.light;
  if (f < SCENES.lid.from) return {x: PLACES.tray.x + 120, y: PLACES.tray.y};
  if (f < SCENES.menu.from) return f >= T.settings.move && f < T.sleeps.move ? {x: CARD_AT.x + 430, y: PLACES.card.y} : {x: PLACES.lid.x, y: -60};
  if (f < SCENES.end.from) return centre(setBox('menu', 'menu'));
  return {x: PLACES.end.x, y: -80};
}

// ---- Small drawn parts --------------------------------------------------------------------------------------------

/** A small moon, drawn: the sleeping sign of the hook's chip and the sleeps shot. */
const Moon: React.FC<{size: number; colour: string}> = ({size, colour}) => (
  <svg width={size} height={size} viewBox="-50 -50 100 100" style={{display: 'block'}}>
    <path d="M 10 -44 A 46 46 0 1 0 44 18 A 38 38 0 1 1 10 -44 Z" fill={colour} />
  </svg>
);

/**
 * A capture set floats on the stage as a picture (`promo-video.md`, "Colour and stage"): a soft deep shadow and a
 * 1 px rim, square corners, so no capture pixel is clipped or rounded.
 */
const Float: React.FC<{w: number; h: number}> = ({w, h}) => (
  <div style={{position: 'absolute', left: 0, top: 0, width: w, height: h, boxShadow: '0 40px 120px rgba(0,0,0,0.55)', outline: `1px solid ${DARK.panelLine}`}} />
);

/** A padlock, drawn. */
const Lock: React.FC<{size: number; colour: string}> = ({size, colour}) => (
  <svg width={size} height={size * 1.15} viewBox="-30 -36 60 70" style={{display: 'block'}}>
    <path d="M -15 -4 L -15 -16 A 15 15 0 0 1 15 -16 L 15 -4" fill="none" stroke={colour} strokeWidth={6.5} strokeLinecap="round" />
    <rect x={-23} y={-6} width={46} height={38} rx={9} fill={colour} />
    <rect x={-3} y={6} width={6} height={12} rx={3} fill={DARK.panel} />
  </svg>
);

/** The panel look shared by the chips, the badge and the card: a soft top light, a hairline rim, a deep shadow. */
export const PANEL: React.CSSProperties = {
  background: `linear-gradient(180deg, #232B1F 0%, ${DARK.panel} 100%)`,
  border: '1px solid rgba(255,255,255,0.09)',
  boxShadow: '0 24px 60px rgba(0,0,0,0.55), inset 0 1px 0 rgba(255,255,255,0.06)',
};

const Chip: React.FC<{children: React.ReactNode; style?: React.CSSProperties}> = ({children, style}) => (
  <div
    style={{
      ...PANEL,
      display: 'inline-flex',
      alignItems: 'center',
      gap: 16,
      height: 76,
      padding: '0 30px 0 22px',
      borderRadius: 38,
      fontFamily: SANS,
      fontSize: 36,
      color: DARK.text,
      whiteSpace: 'nowrap',
      ...style,
    }}
  >
    {children}
  </div>
);

const Badge: React.FC<{children: React.ReactNode}> = ({children}) => (
  <div style={{...PANEL, width: 108, height: 108, borderRadius: 54, display: 'flex', alignItems: 'center', justifyContent: 'center'}}>{children}</div>
);

// ---- Shot 1 and 2: the hook and the light -------------------------------------------------------------------------

/** The hook's terminal at work, then still; in the light shot the same terminal, faint, working again. */
const HookTerminal: React.FC<{f: number}> = ({f}) => {
  const light = f >= SCENES.light.from;
  // In the light shot the lines go on where they stopped, at half the pace, behind the glyph.
  const virtual = light ? T.hook.sleep + Math.max(0, f - T.light.faintLines) / 2 : f;
  const push = light ? 1 : 1 + 0.03 * Math.min(1, f / 60);
  return (
    <At x={HOOK_TERMINAL.x} y={HOOK_TERMINAL.y}>
      <div style={{perspective: 1600, opacity: light ? 0.1 : 1}}>
        <div style={{transform: `rotateY(8deg) scale(${push})`, transformOrigin: '50% 50%'}}>
          <Terminal f={virtual} streamFrom={T.hook.firstLine} every={T.hook.lineEvery} stopAt={light ? Infinity : T.hook.sleep} soft={light ? 3 : 0} />
        </div>
      </div>
    </At>
  );
};

/** The hero glyph of the light shot, in the world until the whip takes it. */
const HeroGlyph: React.FC<{f: number}> = ({f}) => {
  const on = progress(f, T.light.on, LIGHT_ON_FRAMES, LAMP_ON);
  const bloom = progress(f, T.light.on, 18, SETTLE);
  return (
    <>
      <div
        style={{
          position: 'absolute',
          left: HERO.light.x - 420,
          top: HERO.light.y - 420,
          width: 840,
          height: 840,
          borderRadius: '50%',
          opacity: 0.55 * bloom,
          transform: `scale(${0.3 + 0.7 * bloom})`,
          background: `radial-gradient(closest-side, ${MOSS.lightEdge}66 0%, ${MOSS.lightEdge}22 45%, ${MOSS.lightEdge}00 100%)`,
        }}
      />
      {f < T.tray.whip ? <TrayGlyph size={HERO.size} on={on} colours={ON_DARK} page={null} style={{position: 'absolute', left: HERO.left, top: HERO.top}} /> : null}
    </>
  );
};

// ---- Shot 3: Claude Code starts, the icon turns green; the work ends, it lets go ---------------------------------

/** Which picture of the tray set shows at frame `f`: the icon's state and its tooltip follow the terminal exactly. */
const trayShot = (f: number) => (f < T.tray.start ? 'S1' : f < T.tray.hover ? 'S2' : f < T.tray.done ? 'S3' : 'S4');

const TrayShot: React.FC<{f: number}> = ({f}) => {
  const at = SET_AT.tray;
  const shot = trayShot(f);
  const glyph = setBox('tray', 'icon');
  // A ring of the holding colour spreads from the icon the moment it turns, so the eye goes from the Enter to the icon.
  const pulse = within(f, T.tray.start, T.tray.start + 14) ? (f - T.tray.start) / 14 : null;
  return (
    <>
      <div style={{position: 'absolute', left: at.x, top: at.y}}>
        <StandInTag set="tray" shots={Object.keys(SETS.tray.shots)} style={{top: SETS.tray.h * at.zoom + 12}} />
        <Float w={SETS.tray.w * at.zoom} h={SETS.tray.h * at.zoom} />
        {shot === 'S3' ? (
          <>
            {/* The hover look of the button comes at once, as under a hand; the tooltip rises in with its shadow. */}
            <Capture set="tray" shot="S2" zoom={at.zoom} />
            <Capture set="tray" shot="S3" zoom={at.zoom} clip="button" />
            <Pop at={T.tray.hover} rise>
              <Capture set="tray" shot="S3" zoom={at.zoom} clip="tipPop" />
            </Pop>
          </>
        ) : (
          <Capture set="tray" shot={shot} zoom={at.zoom} />
        )}
      </div>
      {pulse !== null ? (
        <div
          style={{
            position: 'absolute',
            left: glyph.x + glyph.w / 2 - 40 - 50 * pulse,
            top: glyph.y + glyph.h / 2 - 40 - 50 * pulse,
            width: 80 + 100 * pulse,
            height: 80 + 100 * pulse,
            borderRadius: '50%',
            border: `4px solid ${MOSS.holdingOnDark}`,
            opacity: 1 - pulse,
          }}
        />
      ) : null}
      <div style={{position: 'absolute', left: TRAY_TERMINAL.x, top: TRAY_TERMINAL.y, transform: `scale(${TRAY_TERMINAL.scale})`, transformOrigin: '0 0'}}>
        <Terminal
          f={f}
          preset={0}
          waitUntil={T.tray.start}
          streamFrom={T.tray.start + T.tray.lineEvery}
          every={T.tray.lineEvery}
          stopAt={T.tray.done}
          doneAt={T.tray.done}
          rows={5}
        />
      </div>
    </>
  );
};

/** The glyph flying from the light shot into the tray, going out as it flies, so it lands as the idle icon. */
const FlyingGlyph: React.FC<{f: number}> = ({f}) => {
  if (!within(f, T.tray.whip, T.tray.whip + T.tray.whipLen)) return null;
  const end = setBox('tray', 'icon');
  // Both ends are where the camera rests in each shot, so the glyph leaves the hero's spot and lands on the capture's.
  const from = toScreen(PLACES.hook, SIZE, HERO.left, HERO.top);
  const to = toScreen(PLACES.tray, SIZE, end.x, end.y);
  const at = (g: number) => {
    const t = progress(g, T.tray.whip, T.tray.whipLen, SETTLE);
    return {x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t, s: 1 + (end.w / HERO.size - 1) * t, t};
  };
  const p = at(f);
  const a = at(f - 0.5);
  const b = at(f + 0.5);
  return (
    <DirectionalBlur vx={b.x - a.x} vy={b.y - a.y} id="glyph-blur">
      <TrayGlyph
        size={HERO.size}
        on={1 - p.t}
        colours={ON_DARK}
        page={null}
        style={{position: 'absolute', left: 0, top: 0, transformOrigin: '0 0', transform: `translate(${p.x}px, ${p.y}px) scale(${p.s})`}}
      />
    </DirectionalBlur>
  );
};

/** The cursor in the tray shot: it comes to rest on the icon, so the tooltip shows as it does under a hand. */
function trayCursor(): CursorMove[] {
  const g = centre(setBox('tray', 'icon'));
  const at = SET_AT.tray;
  return [
    {at: T.tray.whip, x: at.x + SETS.tray.w * at.zoom + 40, y: g.y + 60, len: 0},
    {at: T.tray.hover - 2, x: g.x + 14, y: g.y + 14, len: 14},
  ];
}

// ---- Shots 4 to 6: the lid, the settings, the sleep ---------------------------------------------------------------

const lidShut = (f: number) => {
  const close = progress(f, T.lid.close, T.lid.shut - T.lid.close, Easing.in(Easing.cubic));
  // The lid meets the base and lifts 2 degrees before it rests.
  const bounce = within(f, T.lid.shut, T.lid.shut + 8) ? (2 / 105) * Math.sin((Math.PI * (f - T.lid.shut)) / 8) : 0;
  return close - bounce;
};

const LidPlace: React.FC<{f: number}> = ({f}) => {
  const breathe = 0.72 + 0.28 * Math.cos((2 * Math.PI * (f - T.lid.shut)) / 60);
  const edge = f < T.lid.shut ? progress(f, T.lid.close, T.lid.shut - T.lid.close) : breathe * (1 - progress(f, T.settings.off, LIGHT_OFF_FRAMES, LAMP_OFF));
  const led = 1 - progress(f, T.sleeps.sleep, LIGHT_OFF_FRAMES, LAMP_OFF);
  const yaw = -30 + 6 * Math.min(1, Math.max(0, (f - T.lid.orbit) / (SCENES.menu.from - T.lid.orbit)));
  const moon = progress(f, T.sleeps.moon, 15, SETTLE);
  const total = 300;
  const left = Math.max(0, total * (1 - Math.min(1, Math.max(0, (f - T.sleeps.chip) / T.sleeps.countLen))));
  const clock = `${Math.floor(Math.ceil(left) / 60)}:${String(Math.ceil(left) % 60).padStart(2, '0')}`;
  return (
    <>
      <div style={{position: 'absolute', left: LAPTOP_AT.x, top: LAPTOP_AT.y, transform: `scale(${LAPTOP_SCALE})`}}>
        <LaptopTilt f={f} shut={lidShut(f)} yaw={yaw} edge={edge} led={led} streamFrom={T.lid.cut} />
      </div>
      <At x={ABOVE_LAPTOP.x} y={ABOVE_LAPTOP.y}>
        <Pop at={T.lid.lock} out={T.sleeps.sleep}>
          <Badge>
            <Lock size={46} colour={DARK.text} />
          </Badge>
        </Pop>
      </At>
      {f >= T.sleeps.moon ? (
        <At x={ABOVE_LAPTOP.x} y={ABOVE_LAPTOP.y + 20 * (1 - moon)} style={{opacity: moon}}>
          <Moon size={104} colour={DARK.dim} />
        </At>
      ) : null}
      <At x={ABOVE_LAPTOP.x + 330} y={ABOVE_LAPTOP.y + 20}>
        <Pop at={T.sleeps.chip} out={SCENES.menu.from}>
          <Chip>
            <span style={{fontFamily: MONO, fontSize: 38, color: DARK.accent, width: '4ch', display: 'inline-block', textAlign: 'right'}}>{clock}</span>
            <span style={{color: DARK.dim}}>{COPY.idleChip}</span>
          </Chip>
        </Pop>
      </At>
    </>
  );
};

const CardPlace: React.FC<{f: number}> = ({f}) => (
  <div style={{position: 'absolute', left: CARD_AT.x, top: CARD_AT.y}}>
    <Pop at={T.settings.card} out={T.sleeps.move} origin="0% 50%">
      <SettingsCard f={f} rolls={[T.settings.row1, T.settings.row2]} />
    </Pop>
  </div>
);

// ---- Shot 7: the menu ---------------------------------------------------------------------------------------------

const MenuShot: React.FC<{f: number}> = ({f}) => {
  const at = SET_AT.menu;
  const m = T.menu;
  // Which picture of the set is the base at this frame; the parts that open pop over the one before.
  const base = f < m.click1 ? 'S1' : f < m.keep ? 'S5a' : f < m.hour ? 'S5b' : f < m.click2 ? 'S5c' : 'S5d';
  return (
    <div style={{position: 'absolute', left: at.x, top: at.y}}>
      <StandInTag set="menu" shots={Object.keys(SETS.menu.shots)} />
      <Float w={SETS.menu.w} h={SETS.menu.h} />
      {base === 'S5a' ? (
        <>
          <Capture set="menu" shot="S1" zoom={1} />
          <Capture set="menu" shot="S5a" zoom={1} clip="bar" />
          <Pop at={m.click1} rise>
            <Capture set="menu" shot="S5a" zoom={1} clip="menuPop" />
          </Pop>
        </>
      ) : base === 'S5b' ? (
        <>
          <Capture set="menu" shot="S5a" zoom={1} />
          <Capture set="menu" shot="S5b" zoom={1} clip="menuPop" />
          <Capture set="menu" shot="S5b" zoom={1} clip="bar" />
          <Pop at={m.keep} rise>
            <Capture set="menu" shot="S5b" zoom={1} clip="subPop" />
          </Pop>
        </>
      ) : base === 'S5d' ? (
        <>
          <Capture set="menu" shot="S5d" zoom={1} />
          <Capture set="menu" shot="S5c" zoom={1} style={{opacity: 1 - progress(f, m.click2, 4, (t) => t)}} />
        </>
      ) : (
        <Capture set="menu" shot={base} zoom={1} />
      )}
      {/* The battery figure moved from 96% to 97% while the set was taken: one value throughout, so nothing jumps. */}
      <Capture set="menu" shot="S5a" zoom={1} clip="battery" />
    </div>
  );
};

/**
 * After the click the menu is gone and the tray is small: the subject is the icon that now holds, so a round lens
 * rises above it with the tray glyph at 128 px, the port's own pixels, its light coming on as the real icon's did.
 */
const Lens: React.FC<{f: number}> = ({f}) => {
  const b = setBox('menu', 'button');
  const size = 196;
  const art = 128;
  const at = {x: b.x + b.w / 2, y: b.y - 190};
  return (
    <>
      <Pop at={T.menu.ring} origin="50% 100%" style={{position: 'absolute', left: at.x - size / 2, top: at.y - size / 2, width: size, height: size}}>
        <div style={{...PANEL, width: size, height: size, borderRadius: size / 2, display: 'flex', alignItems: 'center', justifyContent: 'center'}}>
          <TrayGlyph size={art} on={progress(f, T.menu.ring, LIGHT_ON_FRAMES, LAMP_ON)} colours={ON_DARK} page={null} />
        </div>
      </Pop>
      <div
        style={{
          position: 'absolute',
          left: at.x - 1,
          top: at.y + size / 2 + 6,
          width: 2,
          height: b.y - (at.y + size / 2) - 18,
          background: MOSS.lightEdge,
          opacity: 0.7 * progress(f, T.menu.ring + 4, 8, (t) => t),
          transformOrigin: '50% 100%',
          transform: `scaleY(${progress(f, T.menu.ring + 4, 8, SETTLE)})`,
        }}
      />
    </>
  );
};

/** The cursor's path in the menu shot, in world pixels. */
function menuCursor(): CursorMove[] {
  const at = SET_AT.menu;
  const icon = centre(setBox('menu', 'button'));
  const keep = centre(setBox('menu', 'keepAwake'));
  const hour = centre(setBox('menu', 'oneHour'));
  return [
    {at: T.menu.cut, x: at.x + SETS.menu.w + 60, y: at.y + SETS.menu.h - 10, len: 0},
    {at: T.menu.click1 - 3, ...icon, len: 12},
    {at: T.menu.keep, x: keep.x + 40, y: keep.y, len: 15},
    {at: T.menu.hour, x: hour.x - 10, y: hour.y, len: 12},
  ];
}

// ---- Shot 8: the end card -----------------------------------------------------------------------------------------

/** The stacked lockup (`docs/brand.md`): the name's cap height 0.36 of the plate's side, 0.26 of it below the plate. */
const STACKED = {cap: 0.36, gap: 0.26};
const END_ICON = 256;

/** The end card's layout: the icon's top, the name's baseline, and the three lines' baselines (screen pixels). */
const END = (() => {
  const p = plated(END_ICON, true);
  const cap = STACKED.cap * p.side;
  const line = STYLE.line.size * STYLE.line.cap;
  const small = STYLE.small.size * STYLE.small.cap;
  const mono = STYLE.mono.size * STYLE.mono.cap;
  const total = p.side + STACKED.gap * p.side + cap + 0.26 * p.side + line + 0.18 * p.side + small + 0.4 * p.side + mono;
  const iconTop = Math.round(VIDEO.height / 2 - total / 2 - p.margin - 10);
  const nameBase = Math.round(iconTop + p.margin + p.side + STACKED.gap * p.side + cap);
  const lineBase = Math.round(nameBase + 0.26 * p.side + line);
  const smallBase = Math.round(lineBase + 0.18 * p.side + small);
  const monoBase = Math.round(smallBase + 0.4 * p.side + mono);
  return {iconTop, nameBase, nameSize: cap / STYLE.name.cap, lineBase, smallBase, monoBase};
})();

const EndPlace: React.FC<{f: number}> = ({f}) => (
  <>
    <AppIcon
      size={END_ICON}
      on={progress(f, T.end.cut, LIGHT_ON_FRAMES, LAMP_ON)}
      style={{position: 'absolute', left: PLACES.end.x - END_ICON / 2, top: PLACES.end.y - VIDEO.height / 2 + END.iconTop}}
    />
    <div
      style={{
        position: 'absolute',
        left: PLACES.end.x - 960,
        width: 1920,
        top: PLACES.end.y - VIDEO.height / 2 + END.nameBase - STYLE.name.cap * END.nameSize,
        textAlign: 'center',
        fontFamily: SANS,
        fontWeight: 600,
        fontSize: END.nameSize,
        letterSpacing: '-0.01em',
        color: DARK.text,
        textBox: 'trim-both cap alphabetic',
      }}
    >
      {COPY.name}
    </div>
  </>
);

/** The speed numbers for the end card: measured, or tagged stand-ins in a draft (the final render refuses them). */
const SPEED_STAND_IN = {ram: '88.8', cpu: '888'};
function speedFill(draft: boolean): {fill: Record<string, string>; measured: boolean; note: string} {
  const s = speedData as Record<string, number | string | boolean | null>;
  if (s.measured === true) {
    return {
      fill: {ram: String(s.idleRamMb), cpu: String(s.idleCpuMsPer10Min)},
      measured: true,
      note: COPY.measured(String(s.build), String(s.date)),
    };
  }
  if (!draft) throw new Error('the speed numbers are not measured; the final video needs them');
  return {fill: SPEED_STAND_IN, measured: false, note: COPY.speedStandIn};
}

// ---- The video ----------------------------------------------------------------------------------------------------

const BLOCK = Object.fromEntries(CAPTIONS.map((b) => [b.id, b]));

/** Each caption block's baselines on screen. */
const BASELINES: Record<string, number[]> = {
  hook: [250, 410],
  name: [690, 790],
  trayStart: [CAPTION_BASELINE],
  trayDone: [CAPTION_BASELINE],
  lid: [CAPTION_BASELINE - CAPTION_STEP, CAPTION_BASELINE],
  settings: [CAPTION_BASELINE],
  sleeps: [CAPTION_BASELINE],
  menu: [CAPTION_BASELINE - CAPTION_STEP, CAPTION_BASELINE],
  end: [END.lineBase, END.smallBase, END.monoBase],
};

/** The draft mark, with the frame and the second, so notes on a draft can name the moment. */
const DraftBanner: React.FC<{f: number}> = ({f}) => (
  <div
    style={{
      position: 'absolute',
      top: 20,
      left: 20,
      background: DARK.draft,
      color: DARK.onDraft,
      fontFamily: SANS,
      fontWeight: 600,
      fontSize: 22,
      padding: '6px 14px',
      borderRadius: 8,
      fontVariantNumeric: 'tabular-nums',
    }}
  >
    {COPY.draft} {(f / VIDEO.fps).toFixed(1).padStart(4, '0')} s, frame {String(f).padStart(3, '0')}
  </div>
);

/** The 30 second video. The soundtrack is one file that `scripts/make-sounds.mjs` mixes on the same timeline. */
export const Promo: React.FC<PromoProps> = ({draft}) => {
  const f = useCurrentFrame();
  const view = viewAt(CAMERA, PLACES as Record<PlaceName, View>, f);
  const velocity = velocityAt(CAMERA, PLACES as Record<PlaceName, View>, f);
  const lift = liftAt(f);
  const liftScreen = toScreen(view, SIZE, lift.x, lift.y);
  const resolve = (target: string): Rect | null => {
    const box =
      target === 'tip' || target === 'tipIdle'
        ? setBox('tray', target)
        : target === 'row1'
          ? cardRow(0)
          : target === 'row2'
            ? cardRow(1)
            : target === 'menuIcon'
              ? setBox('menu', 'button')
              : null;
    if (!box) return null;
    const p = toScreen(view, SIZE, box.x, box.y);
    return {x: p.x, y: p.y, w: box.w * view.zoom, h: box.h * view.zoom};
  };
  const focus = focusAt(FOCUS, f, resolve);
  const dim = within(f, T.hook.sleep, SCENES.light.from) ? 0.9 * progress(f, T.hook.sleep, T.hook.dimLen, (t) => t) : 0;
  const stageLift = f < SCENES.light.from ? 1 - dim : f < SCENES.tray.from ? progress(f, T.light.on, 18, SETTLE) : 1;
  const inTray = within(f, T.tray.whip + T.tray.whipLen, SCENES.lid.from);
  const inMenu = within(f, SCENES.menu.from, SCENES.end.from);
  const cursor = inTray ? cursorAt(trayCursor(), f) : inMenu ? cursorAt(menuCursor(), f) : null;
  const speed = f >= SCENES.end.from ? speedFill(draft) : null;
  return (
    <DraftContext.Provider value={draft}>
      <AbsoluteFill>
        <Stage focus={liftScreen} lift={stageLift} width={VIDEO.width} height={VIDEO.height} />
        <World view={view} velocity={velocity} size={SIZE}>
          {f < T.tray.whip + T.tray.whipLen ? <HookTerminal f={f} /> : null}
          {within(f, SCENES.light.from, T.tray.whip + T.tray.whipLen) ? <HeroGlyph f={f} /> : null}
          {within(f, T.tray.whip, SCENES.lid.from) ? <TrayShot f={f} /> : null}
          {within(f, SCENES.lid.from, SCENES.menu.from) ? <LidPlace f={f} /> : null}
          {within(f, T.settings.card, SCENES.menu.from) ? <CardPlace f={f} /> : null}
          {inMenu ? <MenuShot f={f} /> : null}
          {within(f, T.menu.ring, SCENES.end.from) ? <Lens f={f} /> : null}
          {f >= SCENES.end.from ? <EndPlace f={f} /> : null}
        </World>
        {dim > 0 ? <AbsoluteFill style={{background: '#000', opacity: dim}} /> : null}
        <FlyingGlyph f={f} />
        <Focus rect={focus.rect} amount={focus.amount} dim={focus.dim} width={VIDEO.width} height={VIDEO.height} />
        {within(f, T.hook.sleep, SCENES.light.from) ? (
          <div style={{position: 'absolute', right: 80, top: 70}}>
            <Pop at={T.hook.sleep} origin="100% 0%">
              <Chip>
                <Moon size={32} colour={DARK.dim} />
                {COPY.sleepingChip}
              </Chip>
            </Pop>
          </div>
        ) : null}
        {cursor ? (
          <Cursor
            tip={toScreen(view, SIZE, cursor.x, cursor.y)}
            f={f}
            clicks={inMenu ? [T.menu.click1, T.menu.click2] : []}
            opacity={inMenu ? 1 - progress(f, T.menu.click2 + 8, 8, (t) => t) : 1}
          />
        ) : null}
        {CAPTIONS.map((b) => {
          const first = Math.min(...b.lines.map((l) => l.at));
          return f >= first && (f < b.until + 6 || b.leave === 'stay') ? (
            <Words key={b.id} block={BLOCK[b.id]} baselines={BASELINES[b.id]} fill={speed?.fill} />
          ) : null;
        })}
        {speed ? (
          <div
            style={{
              position: 'absolute',
              left: 0,
              right: 0,
              bottom: 44,
              textAlign: 'center',
              fontFamily: SANS,
              fontSize: 24,
              color: speed.measured ? DARK.dim : DARK.onDraft,
              opacity: progress(f, T.end.speed, 4, (t) => t),
            }}
          >
            <span style={speed.measured ? undefined : {background: DARK.draft, padding: '3px 10px', borderRadius: 4, fontWeight: 600}}>{speed.note}</span>
          </div>
        ) : null}
        <Audio src={staticFile('sound/soundtrack.wav')} />
        {draft ? <DraftBanner f={f} /> : null}
      </AbsoluteFill>
    </DraftContext.Provider>
  );
};
