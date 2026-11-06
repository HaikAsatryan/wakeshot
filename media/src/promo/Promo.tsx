import React from 'react';
import {AbsoluteFill, Easing, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {Audio} from '@remotion/media';
import speedData from '../../data/speed.json';
import {AppIcon, TrayGlyph} from '../art/Glyph.tsx';
import {ON_DARK, plated} from '../art/heldLight.ts';
import {previewTiles, tiles, type Tile} from '../agents.ts';
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
import {Check, Spinner, Terminal} from './Terminal.tsx';
import {
  ABOVE_LAPTOP,
  CARD_AT,
  HERO,
  HUB_SIZE,
  LAPTOP_AT,
  LAPTOP_SCALE,
  LID_ROW,
  PAIN_LAPTOP_AT,
  SET_AT,
  TILE,
  TRAY_TERMINAL,
  WORK_CHIP,
  cardRow,
  glyphAt,
  setBox,
  tileBoxes,
} from './world.ts';

/**
 * `preview` names agents to lay the tiles out for, as if they had passed: only for stills of a layout. The builds
 * never pass it, so the video shows exactly the agents `agents.ts` lists as passed.
 */
export type PromoProps = {draft: boolean; preview?: string[]};

const SIZE = {width: VIDEO.width, height: VIDEO.height};
const within = (f: number, from: number, to: number) => f >= from && f < to;
const centre = (b: Rect) => ({x: b.x + b.w / 2, y: b.y + b.h / 2});

/** Where the stage's soft lift sits in each shot, in world pixels: behind what the viewer should look at. */
function liftAt(f: number): {x: number; y: number} {
  if (f < SCENES.light.from) return {x: PLACES.hook.x, y: PLACES.hook.y + 120};
  if (f < SCENES.agents.from) return HERO.light;
  if (f < SCENES.tray.from) return {x: PLACES.agents.x, y: PLACES.agents.y};
  if (f < SCENES.lid.from) return {x: PLACES.tray.x + 120, y: PLACES.tray.y};
  if (f < SCENES.end.from) return f >= T.settings.move && f < T.sleeps.cut ? {x: CARD_AT.x + 430, y: PLACES.card.y} : {x: PLACES.lid.x, y: -60};
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

// ---- Shot 1: the pain ---------------------------------------------------------------------------------------------

/**
 * The laptop at work, then asleep under the agent: the screen dims, the work on it freezes mid-task, and the power
 * light breathes slowly, the way a sleeping laptop shows it. The same drawing as the lid shot, so the video ends on
 * this laptop sleeping again, this time once the work is done.
 */
const PainPlace: React.FC<{f: number}> = ({f}) => {
  const asleep = progress(f, T.hook.sleep, T.hook.dimLen, LAMP_OFF);
  const breathe = 0.2 + 0.8 * (0.5 + 0.5 * Math.cos((2 * Math.PI * Math.max(0, f - T.hook.sleep - T.hook.dimLen)) / 60));
  const led = f < T.hook.sleep + T.hook.dimLen ? 1 : breathe;
  return (
    <div style={{position: 'absolute', left: PAIN_LAPTOP_AT.x, top: PAIN_LAPTOP_AT.y}}>
      <LaptopTilt f={f} shut={0} yaw={-30} edge={0} led={led} streamFrom={T.hook.firstLine} stopAt={T.hook.sleep} screen={1 - 0.75 * asleep} />
    </div>
  );
};

// ---- Shot 2 and 3: the light, the name, the agents ----------------------------------------------------------------

/** The hero glyph's soft bloom, as large as the glyph allows. */
const Bloom: React.FC<{x: number; y: number; size: number; amount: number; grow: number}> = ({x, y, size, amount, grow}) => (
  <div
    style={{
      position: 'absolute',
      left: x - size / 2,
      top: y - size / 2,
      width: size,
      height: size,
      borderRadius: '50%',
      opacity: 0.55 * amount,
      transform: `scale(${0.3 + 0.7 * grow})`,
      background: `radial-gradient(closest-side, ${MOSS.lightEdge}66 0%, ${MOSS.lightEdge}22 45%, ${MOSS.lightEdge}00 100%)`,
    }}
  />
);

/** The hero glyph of the light shot, in the world, its light coming on. */
const HeroGlyph: React.FC<{f: number}> = ({f}) => {
  const on = progress(f, T.light.on, LIGHT_ON_FRAMES, LAMP_ON);
  const bloom = progress(f, T.light.on, 18, SETTLE);
  return (
    <>
      <Bloom x={HERO.light.x} y={HERO.light.y} size={840} amount={bloom} grow={bloom} />
      <TrayGlyph size={HERO.size} on={on} colours={ON_DARK} page={null} style={{position: 'absolute', left: HERO.left, top: HERO.top}} />
    </>
  );
};

/**
 * The hero glyph from the agents shot on, on screen: it rises with the camera to the top of the frame and shrinks to
 * `HUB_SIZE` as it goes, drawn from the port at each whole size, never a scaled bitmap (`media/README.md` decision
 * 7). It stays lit over the agents: Wakeshot holds while they work.
 */
function hubAt(f: number): {size: number; left: number; top: number; light: {x: number; y: number}} {
  const view = viewAt(CAMERA, PLACES as Record<PlaceName, View>, f);
  const t = progress(f, T.agents.move, 15, SETTLE);
  const size = Math.round((HERO.size + (HUB_SIZE - HERO.size) * t) * view.zoom);
  const mid = toScreen(view, SIZE, HERO.light.x, HERO.inkMid);
  return {size, ...glyphAt(size, mid.x, mid.y)};
}

const HubGlyph: React.FC<{f: number}> = ({f}) => {
  const hub = hubAt(f);
  // A ring of the holding colour spreads from the light once every tile is in: it holds for all of them.
  const pulse = within(f, T.agents.pulse, T.agents.pulse + 16) ? (f - T.agents.pulse) / 16 : null;
  const r = hub.size * 0.3;
  return (
    <>
      <Bloom x={hub.light.x} y={hub.light.y} size={840 * (hub.size / HERO.size)} amount={1} grow={1} />
      <TrayGlyph size={hub.size} on={1} colours={ON_DARK} page={null} style={{position: 'absolute', left: hub.left, top: hub.top}} />
      {pulse !== null ? (
        <div
          style={{
            position: 'absolute',
            left: hub.light.x - r - 70 * pulse,
            top: hub.light.y - r - 70 * pulse,
            width: 2 * r + 140 * pulse,
            height: 2 * r + 140 * pulse,
            borderRadius: '50%',
            border: `4px solid ${MOSS.holdingOnDark}`,
            opacity: 1 - pulse,
          }}
        />
      ) : null}
    </>
  );
};

/**
 * One agent tile: the name, an example task in the terminal's face, and the spinner of an agent at work. The video's
 * drawing in our colours, not a copy of any app, and no owner's logo (`agents.ts` says why).
 */
const AgentTile: React.FC<{f: number; tile: Tile; i: number}> = ({f, tile, i}) => {
  const task = COPY.tasks[tile.name] ?? COPY.taskFallback;
  // A long single name (one Copilot passed alone) steps down a size so it fits the tile.
  const nameSize = tile.name.length > 12 ? 40 : 46;
  return (
    <div
      style={{
        ...PANEL,
        boxSizing: 'border-box',
        width: TILE.w,
        height: TILE.h,
        borderRadius: 30,
        display: 'flex',
        alignItems: 'center',
        gap: 26,
        padding: '0 34px 0 40px',
      }}
    >
      <div style={{flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 12}}>
        <div style={{fontFamily: SANS, fontWeight: 600, fontSize: nameSize, letterSpacing: '-0.01em', color: DARK.text, whiteSpace: 'nowrap', lineHeight: 1}}>
          {tile.name}
        </div>
        <div style={{fontFamily: MONO, fontSize: 25, color: DARK.dim, whiteSpace: 'nowrap', lineHeight: 1}}>{task}</div>
      </div>
      <span style={{fontFamily: MONO, fontSize: 44, flex: 'none'}}>
        <Spinner f={f + 4 * i} colour={DARK.accent} turning />
      </span>
    </div>
  );
};

/** The agents shot: the tiles come in one after another, each rising from below, A to Z. */
const AgentsPlace: React.FC<{f: number; list: Tile[]}> = ({f, list}) => {
  const boxes = tileBoxes(list.length);
  return (
    <>
      {list.map((tile, i) => (
        <div key={tile.name} style={{position: 'absolute', left: boxes[i].x, top: boxes[i].y}}>
          <Pop at={T.agents.tiles + i * T.agents.tileStep} origin="50% 100%">
            <div style={{transform: `translateY(${Math.round(28 * (1 - progress(f, T.agents.tiles + i * T.agents.tileStep, 12, SETTLE)))}px)`}}>
              <AgentTile f={f} tile={tile} i={i} />
            </div>
          </Pop>
        </div>
      ))}
    </>
  );
};

// ---- Shot 4: the agent starts, the icon turns green; the work ends, it lets go ------------------------------------

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
          streamFrom={T.tray.done}
          every={1}
          stopAt={T.tray.done}
          doneAt={T.tray.done}
          rows={0}
          width={TRAY_TERMINAL.width}
          prompt={COPY.terminal.trayPrompt}
        />
      </div>
    </>
  );
};

/**
 * The glyph flying from the top of the agents shot into the tray, going out as it flies, so it lands as the idle
 * icon. It is drawn from the port at each whole size, never a scaled bitmap (`media/README.md` decision 7), and it
 * fades over its last frames into the capture's own icon, so two icons never show side by side (review m3 of round 4).
 */
const FlyingGlyph: React.FC<{f: number}> = ({f}) => {
  if (!within(f, T.tray.whip, T.tray.whip + T.tray.whipLen)) return null;
  const end = setBox('tray', 'icon');
  // It leaves the hub's spot on screen and flies to where the capture's icon is on screen at that frame, so it lands
  // on the icon while the camera still settles and then rides on it, the same size, until it has faded.
  const hub = hubAt(T.tray.whip);
  const from = {x: hub.left, y: hub.top};
  const at = (g: number) => {
    const view = viewAt(CAMERA, PLACES as Record<PlaceName, View>, g);
    const to = {x: Math.round(SIZE.width / 2 - view.x) + end.x, y: Math.round(SIZE.height / 2 - view.y) + end.y};
    const t = progress(g, T.tray.whip, FLY_LAND, SETTLE);
    return {x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t, s: 1 + (end.w / hub.size - 1) * t, t};
  };
  const p = at(f);
  const a = at(f - 0.5);
  const b = at(f + 0.5);
  const size = Math.max(8, Math.round(hub.size * p.s));
  const fade = 1 - progress(f, T.tray.whip + FLY_LAND, T.tray.whipLen - FLY_LAND, (t) => t);
  return (
    <DirectionalBlur vx={b.x - a.x} vy={b.y - a.y} id="glyph-blur">
      <TrayGlyph size={size} on={1 - p.t} colours={ON_DARK} page={null} style={{position: 'absolute', left: Math.round(p.x), top: Math.round(p.y), opacity: fade}} />
    </DirectionalBlur>
  );
};

/** The flying glyph lands on the icon this many frames into the whip, then fades over the whip's last frames. */
const FLY_LAND = 8;

/** The cursor in the tray shot: it comes to rest on the icon, so the tooltip shows as it does under a hand. */
function trayCursor(): CursorMove[] {
  const g = centre(setBox('tray', 'icon'));
  const at = SET_AT.tray;
  return [
    {at: T.tray.whip, x: at.x + SETS.tray.w * at.zoom + 40, y: g.y + 60, len: 0},
    {at: T.tray.hover - 2, x: g.x + 14, y: g.y + 14, len: 14},
  ];
}

// ---- Shots 5 to 7: the lid, the settings, the sleep ---------------------------------------------------------------

const lidShut = (f: number) => {
  const close = progress(f, T.lid.close, T.lid.shut - T.lid.close, Easing.in(Easing.cubic));
  // The lid meets the base and lifts 2 degrees before it rests.
  const bounce = within(f, T.lid.shut, T.lid.shut + 8) ? (2 / 105) * Math.sin((Math.PI * (f - T.lid.shut)) / 8) : 0;
  return close - bounce;
};

/**
 * The work behind the shut lid: a chip with the agent's step under a turning spinner; at `done` it
 * says the work is done, with the terminal's own done line. The video's drawing, not a copy of any app's screen.
 */
const WorkChip: React.FC<{f: number}> = ({f}) => {
  const step = COPY.terminal.behindLid;
  const done = f >= T.lid.done;
  return (
    <Chip style={{width: WORK_CHIP.w, height: WORK_CHIP.h, boxSizing: 'border-box', gap: 18, fontFamily: MONO, fontSize: 32}}>
      <span style={{width: 56, display: 'inline-flex', justifyContent: 'center', fontSize: 40}}>
        {done ? <Check size={34} colour={DARK.accent} /> : <Spinner f={f} colour={DARK.accent} turning />}
      </span>
      {done ? (
        <span style={{color: DARK.accent}}>{COPY.terminal.done}</span>
      ) : (
        <span style={{color: DARK.terminalText}}>
          <span style={{color: DARK.dim, display: 'inline-block', width: '5.5ch'}}>{step[0]}</span>
          {step[1]}
        </span>
      )}
    </Chip>
  );
};

const LidPlace: React.FC<{f: number}> = ({f}) => {
  const breathe = 0.72 + 0.28 * Math.cos((2 * Math.PI * (f - T.lid.shut)) / 60);
  // The edge light is the hold: it goes off at the frame the work is done.
  const edge = f < T.lid.shut ? progress(f, T.lid.close, T.lid.shut - T.lid.close) : breathe * (1 - progress(f, T.lid.done, LIGHT_OFF_FRAMES, LAMP_OFF));
  const led = 1 - progress(f, T.sleeps.sleep, LIGHT_OFF_FRAMES, LAMP_OFF);
  const yaw = -30 + 6 * Math.min(1, Math.max(0, (f - T.lid.orbit) / (SCENES.end.from - T.lid.orbit)));
  const moon = progress(f, T.sleeps.moon, 15, SETTLE);
  // After the time cut the badge sits alone, centred; it goes as the laptop sleeps and the moon takes its place.
  const lock = f < T.sleeps.cut ? LID_ROW.lock : ABOVE_LAPTOP;
  return (
    <>
      <div style={{position: 'absolute', left: LAPTOP_AT.x, top: LAPTOP_AT.y, transform: `scale(${LAPTOP_SCALE})`}}>
        <LaptopTilt f={f} shut={lidShut(f)} yaw={yaw} edge={edge} led={led} streamFrom={T.lid.cut} />
      </div>
      <At x={lock.x} y={lock.y}>
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
      {within(f, T.lid.work, T.sleeps.cut) ? (
        <div style={{position: 'absolute', left: LID_ROW.chip.x, top: LID_ROW.chip.y}}>
          <Pop at={T.lid.work} origin="50% 100%">
            <WorkChip f={f} />
          </Pop>
        </div>
      ) : null}
    </>
  );
};

const CardPlace: React.FC<{f: number}> = ({f}) => (
  <div style={{position: 'absolute', left: CARD_AT.x, top: CARD_AT.y}}>
    <Pop at={T.settings.card} origin="0% 50%">
      <SettingsCard f={f} rolls={[T.settings.row1, T.settings.row2]} />
    </Pop>
  </div>
);

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
  const s = speedData as {measured: boolean; build: string | null; date: string | null; idleRamMb: number | null; idleCpuMsPer10Min: number | null};
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
  hook: [170, 318],
  question: [575],
  name: [690, 790],
  agents: [CAPTION_BASELINE],
  trayStart: [CAPTION_BASELINE],
  trayDone: [CAPTION_BASELINE],
  lid: [CAPTION_BASELINE],
  settings: [CAPTION_BASELINE],
  sleeps: [CAPTION_BASELINE],
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
export const Promo: React.FC<PromoProps> = ({draft, preview}) => {
  const f = useCurrentFrame();
  const list = preview ? previewTiles(preview) : tiles();
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
            : null;
    if (!box) return null;
    const p = toScreen(view, SIZE, box.x, box.y);
    return {x: p.x, y: p.y, w: box.w * view.zoom, h: box.h * view.zoom};
  };
  const focus = focusAt(FOCUS, f, resolve);
  // The stage darkens under the question, so it stands alone over the sleeping laptop.
  const dark = within(f, T.hook.question, SCENES.light.from) ? 0.86 * progress(f, T.hook.question, T.hook.darkLen, (t) => t) : 0;
  // The time cut before the sleep. Before the cut the black covers everything, the settings line too, so the shot
  // leaves as one; from the cut it sits under the words, so "Later" reads on the black as the picture comes back.
  const dip = interpolate(f, T.sleeps.dip, [0, 1, 1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp', easing: Easing.inOut(Easing.sin)});
  const black = dip > 0 ? <AbsoluteFill style={{background: '#000', opacity: dip}} /> : null;
  const stageLift = f < SCENES.light.from ? 1 - dark : f < SCENES.agents.from ? progress(f, T.light.on, 18, SETTLE) : 1;
  const inTray = within(f, T.tray.whip + T.tray.whipLen, SCENES.lid.from);
  const cursor = inTray ? cursorAt(trayCursor(), f) : null;
  const speed = f >= SCENES.end.from ? speedFill(draft) : null;
  return (
    <DraftContext.Provider value={draft}>
      {/*
        A new tree every frame (keyed by the frame), so Chrome paints each frame from nothing. Kept, it repaints only
        what changed since the frame the render tab drew before, and which frame that was depends on how the tabs
        share the work: soft shadows and gradients then kept a level of the old frame at their edges, and two renders
        of the same source differed (round 4).
      */}
      <AbsoluteFill key={f}>
        <Stage focus={liftScreen} lift={stageLift} width={VIDEO.width} height={VIDEO.height} />
        <World view={view} velocity={velocity} size={SIZE}>
          {f < SCENES.light.from ? <PainPlace f={f} /> : null}
          {within(f, SCENES.light.from, T.agents.move) ? <HeroGlyph f={f} /> : null}
          {within(f, T.agents.move, T.tray.whip + T.tray.whipLen) ? <AgentsPlace f={f} list={list} /> : null}
          {within(f, T.tray.whip, SCENES.lid.from) ? <TrayShot f={f} /> : null}
          {within(f, SCENES.lid.from, SCENES.end.from) ? <LidPlace f={f} /> : null}
          {within(f, T.settings.card, T.sleeps.cut) ? <CardPlace f={f} /> : null}
          {f >= SCENES.end.from ? <EndPlace f={f} /> : null}
        </World>
        {within(f, T.agents.move, T.tray.whip) ? <HubGlyph f={f} /> : null}
        <FlyingGlyph f={f} />
        <Focus rect={focus.rect} amount={focus.amount} dim={focus.dim} width={VIDEO.width} height={VIDEO.height} />
        {within(f, T.hook.sleep, SCENES.light.from) ? (
          // Beside the lid's top corner, where the eye already is.
          <div style={{position: 'absolute', left: 1430, top: 400}}>
            <Pop at={T.hook.sleep} origin="0% 50%">
              <Chip>
                <Moon size={32} colour={DARK.dim} />
                {COPY.sleepingChip}
              </Chip>
            </Pop>
          </div>
        ) : null}
        {dark > 0 ? <AbsoluteFill style={{background: '#000', opacity: dark}} /> : null}
        {cursor ? <Cursor tip={toScreen(view, SIZE, cursor.x, cursor.y)} f={f} clicks={[]} opacity={1} /> : null}
        {f >= T.sleeps.cut ? black : null}
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
              fontSize: 28,
              color: speed.measured ? DARK.dim : DARK.onDraft,
              opacity: progress(f, T.end.speed, 4, (t) => t),
            }}
          >
            <span style={speed.measured ? undefined : {background: DARK.draft, padding: '3px 10px', borderRadius: 4, fontWeight: 600}}>{speed.note}</span>
            {/* The agents shot names other companies' products: say plainly they are theirs and we are not with them. */}
            <span style={{color: DARK.dim}}> {COPY.trademarks}</span>
          </div>
        ) : null}
        {f < T.sleeps.cut ? black : null}
        {draft ? <DraftBanner f={f} /> : null}
      </AbsoluteFill>
      <Audio src={staticFile('sound/soundtrack.wav')} />
    </DraftContext.Provider>
  );
};
