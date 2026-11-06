/** Port Run messages, as the server sends and takes them. */

export type Role = 'thief' | 'cop';
export type Stance = 'walk' | 'run' | 'crouch';
export type Class = 'sedan' | 'suv' | 'hatch' | 'pickup' | 'van' | 'cruiser';
export type Colour = 'black' | 'navy' | 'green' | 'grey' | 'white' | 'silver' | 'red';
export type Kind = 'crew' | 'parked' | 'stolen' | 'cruiser';
export type Tyre = 'ok' | 'flat' | 'rim';
export type Entry = 'handle' | 'lock' | 'window';
export type PingKind = 'mark' | 'pickup' | 'danger' | 'radio' | 'witness' | 'reader' | 'report' | 'flagged' | 'seen' | 'alarm';
export type Difficulty = 'easy' | 'normal' | 'hard';

export type Place = { at: 'foot' } | { at: 'seat'; vehicle: number; driver: boolean };

export type Damage = { panels: [number, number, number, number]; engine: number; lights: [boolean, boolean]; tyres: [Tyre, Tyre, Tyre, Tyre]; window_broken: boolean };
export type Handling = { grip: number; pull: number; top_speed: number; sparks: boolean; smoke: boolean };

export type Settings = { max_thieves: number; max_cops: number; difficulty: Difficulty; minutes: number };

export type VehicleView = {
  id: number;
  kind: Kind;
  class: Class;
  colour: Colour;
  x: number;
  z: number;
  h: number;
  damage: Damage;
  handling: Handling;
  driver: string | null;
  riders: string[];
  owner: string | null;
  siren: boolean;
  spotlight: boolean;
};

export type Ring = { speed: number; target: number; window: number };
export type Meter = { phase: number; freq: number; band: number; need: number; needle: number; progress: number };
export type TakeStage =
  | { stage: 'door'; tried_handle: boolean }
  | { stage: 'lock'; started: number; ring: Ring; hits: number; misses: number }
  | { stage: 'ignition'; started: number; meter: Meter }
  | { stage: 'started' };

export type Results = {
  outcome: 'delivered' | 'arrested' | 'timeout';
  payout: number;
  condition: number;
  seconds: number;
  stats: { driven: number; pursuits: number; spikes_hit: number; spin_outs: number; witnesses: number; plate_hits: number; foot_arrests: number; peak_heat: number };
  thieves: string[];
  cops: string[];
  car: string | null;
};

export type Ping = { id: number; kind: PingKind; x: number; z: number; text: string; age: number; by: string | null };

export type HeistView = {
  phase: { phase: 'setup' } | { phase: 'loading' } | { phase: 'running'; left: number } | { phase: 'ended'; results: Results };
  settings: Settings;
  me: { role: Role | null; place: Place; arrested: boolean } | null;
  players: { id: string; name: string; role: Role | null; place: Place; arrested: boolean; online: boolean }[];
  vehicles: VehicleView[];
  pings: Ping[];
  pursuit: 'calm' | 'pursuit' | 'search' | 'cooldown';
  spikes: { id: number; a: [number, number]; b: [number, number]; until: number }[];
  world: number;
  gate: { x: number; z: number; r: number };
  dev: boolean;
  contract?: { class: Class; shade: 'dark' | 'light' | 'red'; text: string; towns: { name: string; county: string; x: number; z: number; r: number }[] };
  heat?: number;
  reported?: boolean;
  take?: { car: number; thief: string; stage: TakeStage; alarm: { start: number; until: number } | null; mine: boolean; elapsed: number | null };
  wanted?: string | null;
};

export type HeistWorld = {
  version: number;
  parked: { id: number; class: Class; colour: Colour; x: number; z: number; h: number }[];
  readers: { x: number; z: number }[];
};

export type CarPose = { id: number; x: number; z: number; h: number; s: number; lights: boolean; siren: boolean; spot: boolean; driver: string | null };
export type AiPose = { id: string; x: number; z: number; h: number; s: number; siren: boolean };
export type FootPose = { id: string; x: number; z: number; h: number; s: number; stance: Stance; flash: boolean; role: Role; arrested: boolean };
export type HeistPoses = { cars: CarPose[]; ai: AiPose[]; feet: FootPose[] };

export type HeistEvent =
  | { to: string; event: 'notice'; text: string }
  | { to: string; event: 'alarm'; vehicle: number; until: number }
  | { to: string; event: 'porch_light'; x: number; z: number }
  | { to: string; event: 'bark'; x: number; z: number }
  | { to: string; event: 'take'; result: string }
  | { to: string; event: 'pursuit'; started: boolean }
  | { to: string; event: 'spin'; vehicle: number; side: number }
  | { to: string; event: 'spiked'; vehicle: number }
  | { to: string; event: 'arrested'; user_id: string }
  | { to: string; event: 'correction'; vehicle: number | null; x: number; z: number; h: number }
  | { to: string; event: 'ended'; outcome: Results['outcome'] }
  | { to: string; event: 'started' };

export type HeistMsg =
  | { op: 'role'; role: Role | null }
  | { op: 'settings'; settings: Settings }
  | { op: 'start' }
  | { op: 'rematch' }
  | { op: 'vehicle'; id: number; pose: { x: number; z: number; h: number; s: number; lights: boolean; horn: boolean } }
  | { op: 'foot'; foot: { x: number; z: number; h: number; s: number; stance: Stance; flash: boolean } }
  | { op: 'enter'; vehicle: number; driver: boolean }
  | { op: 'exit' }
  | { op: 'take_start'; vehicle: number }
  | { op: 'take_choose'; entry: Entry }
  | { op: 'take_press'; t: number }
  | { op: 'take_hold'; t: number; hold: boolean }
  | { op: 'take_abandon' }
  | { op: 'impact'; vehicle: number; x: number; z: number; speed: number }
  | { op: 'siren'; on: boolean }
  | { op: 'spotlight'; on: boolean }
  | { op: 'spike' }
  | { op: 'ping'; kind: PingKind; x: number; z: number; text: string }
  | { op: 'dev'; cmd: { do: 'vehicle'; id: number; x: number; z: number; h: number } | { do: 'me'; x: number; z: number } | { do: 'report_now' } | { do: 'end_in'; seconds: number } };

export const COLOURS: Record<Colour, string> = {
  black: '#1d1f24',
  navy: '#1f2f57',
  green: '#22402e',
  grey: '#4b4f56',
  white: '#e6e6e2',
  silver: '#aeb3b8',
  red: '#9c1d1d',
};

export const SHADE: Record<Colour, 'dark' | 'light' | 'red'> = {
  black: 'dark',
  navy: 'dark',
  green: 'dark',
  grey: 'dark',
  white: 'light',
  silver: 'light',
  red: 'red',
};

export const CLASS_LABEL: Record<Class, string> = { sedan: 'sedan', suv: 'SUV', hatch: 'hatchback', pickup: 'pickup', van: 'van', cruiser: 'cruiser' };
