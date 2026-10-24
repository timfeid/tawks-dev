import type * as THREE from 'three';

export type Character = {
  id: string;
  name: string;
  body: string;
  accent: string;
  belly: string;
};

export type Pose = 'idle' | 'ride' | 'cheer' | 'sad' | 'fall';

/** A party character. Feet rest at y = 0 and it faces +z. */
export type Rider = {
  group: THREE.Group;
  /** `speed` is ground speed in units per second, for the walk cycle. */
  animate(pose: Pose, time: number, speed: number): void;
  dispose(): void;
};

export type Seat = {
  seat: number;
  name: string;
  character: string;
  is_bot: boolean;
  connected: boolean;
  in_round: boolean;
};

export type BoutPhase = 'lobby' | 'countdown' | 'playing' | 'results';

export type Sounds = {
  bump(strength: number): void;
  fall(): void;
  splash(): void;
  shot(): void;
  splat(): void;
  blast(): void;
  down(mine: boolean): void;
  step(volume: number): void;
  beep(urgency: number): void;
  count(final: boolean): void;
  win(): void;
};

/** What a renderer may send back to the room besides the stick. */
export type BoutMessage =
  | { type: 'input'; x: number; z: number; yaw?: number; pitch?: number; jump?: boolean; crouch?: boolean }
  | { type: 'fire'; yaw: number; pitch: number };

export type BoutHost = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  canvas: HTMLCanvasElement;
  /** A phone or a small window: keep geometry and shadows cheap. */
  mobile: boolean;
  /** The y of the sea every bout floats in. */
  waterY: number;
  sounds: Sounds;
  character(id: string): Character | undefined;
  rider(character: Character): Rider;
  send(message: BoutMessage): void;
};

/** The look and buttons of a first-person player, read by first-person bouts. */
export type Look = { yaw: number; pitch: number; jump: boolean; crouch: boolean };

export type BoutFrame<Snapshot, Setup> = {
  /** The bout's key; one module can draw several related bouts. */
  key: string;
  snapshot: Snapshot | null;
  setup: Setup | null;
  phase: BoutPhase;
  seats: Seat[];
  /** The viewer's seat, or null for spectators. */
  mySeat: number | null;
  /** Seats in first place once the bout is over. */
  winners: number[];
  /** Whether the viewer is playing this bout, as opposed to watching. */
  playing: boolean;
  stick: { x: number; z: number };
  look: Look;
  time: number;
  dt: number;
};

/**
 * Where the camera goes. `frame` keeps a box of `half` (x by z) in view from
 * above at `pitch` degrees, centred on `target`. `eye` puts the camera at a
 * point looking along yaw and pitch, for first-person bouts.
 */
export type Shot =
  | { kind: 'frame'; half: [number, number]; pitch: number; portraitPitch?: number; target?: [number, number]; lookHeight?: number }
  | { kind: 'eye'; x: number; y: number; z: number; yaw: number; pitch: number };

export type CombatHud = {
  /** What the left panel tracks. */
  meter: 'health' | 'score' | 'ball';
  hp: number;
  hpMax: number;
  hits: number;
  downs: number;
  up: boolean;
  /** Seconds until the viewer is back in, if they come back. */
  back: number | null;
  holding: boolean;
  hitsLabel: string;
  outTitle: string;
  team?: number;
};

export type BoutHud = {
  /** One line on what to do, shown beside the bout's name. */
  objective?: string;
  /** How to play with a keyboard, shown at the bottom. */
  hint?: string;
  /** A warning under the clock, such as a closing arena. */
  alert?: string | null;
  combat?: CombatHud | null;
  /** Per-seat tallies shown in the roster; leave out `downs` when nobody goes down. */
  scores?: { seat: number; hits: number; downs?: number; up: boolean; team?: number }[];
  teams?: [string, string];
  meter?: { label: string; progress: number; text: string } | null;
  award?: { id: number; points: number; caption: string } | null;
  hitMarker?: boolean;
  /** Touch players get a joystick unless the bout asks for pump buttons. */
  touch?: 'stick' | 'pump';
  /** First-person bouts: the fire button's name, and whether there is a crouch button. */
  action?: string;
  crouch?: boolean;
};

export interface BoutRenderer<Snapshot = unknown, Setup = unknown, Event = unknown> {
  frame(view: BoutFrame<Snapshot, Setup>): void;
  /** Called once the view has caught up with the moment the event happened. */
  effect?(event: Event, view: BoutFrame<Snapshot, Setup>): void;
  shot(view: BoutFrame<Snapshot, Setup>): Shot;
  /** Camera shake in world units, decaying on its own. */
  shake?(): number;
  hud?(view: BoutFrame<Snapshot, Setup>): BoutHud;
  /** Where the viewer faces when they come back in, for first-person bouts. */
  facing?(view: BoutFrame<Snapshot, Setup>): number | null;
  /** Turns the ground point under the pointer into the yaw sent with input. */
  aim?(point: { x: number; z: number }, view: BoutFrame<Snapshot, Setup>): number | null;
  /** Mist distance, for bouts larger than the default arena. */
  fog?: [number, number];
  dispose(): void;
}

export type BoutModule<Snapshot = unknown, Setup = unknown, Event = unknown> = {
  create(host: BoutHost): BoutRenderer<Snapshot, Setup, Event>;
  /**
   * Blends two snapshots, `t` of the way from `a` to `b`. Snapshots arrive 20
   * times a second and are drawn about 100 ms late, so without this the bout
   * moves in steps.
   */
  blend?(a: Snapshot, b: Snapshot, t: number): Snapshot;
  /** Carries the newest snapshot forward when the next one is late. */
  extrapolate?(snapshot: Snapshot, dt: number): Snapshot;
};

export function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

/** The shortest way round the circle, so a spin never takes the long path. */
export function turn(from: number, to: number, t: number) {
  let delta = (to - from) % (Math.PI * 2);
  if (delta > Math.PI) delta -= Math.PI * 2;
  if (delta < -Math.PI) delta += Math.PI * 2;
  return from + delta * t;
}
