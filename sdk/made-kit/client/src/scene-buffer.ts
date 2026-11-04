import type { Effect, Entity, FrameDelta, SceneFrame } from './contract';

/** A whole frame with its entities keyed by id, the way the renderer reads it. */
export type SceneState = Omit<SceneFrame, 'entities'> & { entities: Map<string, Entity> };

/**
 * Rebuilds the frame a delta describes, as `FrameDelta::apply` does in
 * made-protocol. A delta whose base is not the previous frame returns null,
 * and the caller waits for the next whole frame.
 */
export function applyDelta(previous: SceneState | null, delta: FrameDelta): SceneState | null {
  if (delta.base !== null && (!previous || previous.n !== delta.base)) return null;
  const entities = new Map(delta.base === null ? [] : previous!.entities);
  for (const id of delta.del) entities.delete(id);
  for (const entity of delta.set) entities.set(entity.id, entity);
  return { n: delta.n, t: delta.t, stage: delta.stage, camera: delta.camera, hud: delta.hud, entities, effects: delta.effects };
}

export function toState(frame: SceneFrame): SceneState {
  return { ...frame, entities: new Map(frame.entities.map((e) => [e.id, e])) };
}

export function toFrame(state: SceneState): SceneFrame {
  return { ...state, entities: [...state.entities.values()] };
}

/** The delta from `from` to `to`, as `SceneFrame::diff` makes it; null `from` gives a whole frame. */
export function diffFrames(from: SceneFrame | null, to: SceneFrame): FrameDelta {
  const base = { n: to.n, t: to.t, stage: to.stage, camera: to.camera, hud: to.hud, effects: to.effects };
  if (!from) return { ...base, base: null, set: [...to.entities], del: [] };
  const before = new Map(from.entities.map((e) => [e.id, e]));
  const set: Entity[] = [];
  for (const entity of to.entities) {
    const old = before.get(entity.id);
    before.delete(entity.id);
    if (!old || !sameEntity(old, entity)) set.push(entity);
  }
  return { ...base, base: from.n, set, del: [...before.keys()].sort() };
}

function sameEntity(a: Entity, b: Entity) {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]) as Set<keyof Entity>;
  for (const key of keys) if (a[key] !== b[key]) return false;
  return true;
}

const LERPED = ['x', 'y', 'z', 'r', 'w', 'd', 'h'] as const;

const wrapAngle = (angle: number) => Math.atan2(Math.sin(angle), Math.cos(angle));

/** `a` moved toward `b` by `t`, field by field. Anything that is not a position, size or turn snaps to `b`. */
export function blendEntity(a: Entity, b: Entity, t: number): Entity {
  if (a.kind !== b.kind) return b;
  const out: Entity = { ...b };
  for (const field of LERPED) {
    const p = a[field];
    const q = b[field];
    if (typeof p === 'number' && typeof q === 'number') out[field] = p + (q - p) * t;
  }
  if (typeof a.yaw === 'number' && typeof b.yaw === 'number') {
    // A ball's yaw is how far it has rolled, which keeps growing; only a facing wraps.
    out.yaw = b.kind === 'ball' ? a.yaw + (b.yaw - a.yaw) * t : a.yaw + wrapAngle(b.yaw - a.yaw) * t;
  }
  return out;
}

export type Sample = {
  state: SceneState;
  entities: Entity[];
  /** Ground speed of each player, in metres a second, for the ride and walk animations. */
  speeds: Map<string, number>;
};

type Timed = { state: SceneState; played: boolean };

const KEEP = 24;
/** A jump bigger than this between two frames is a respawn, not movement, so it is not smeared. */
const TELEPORT = 6;

/**
 * Recent scene states in game time. `sample` draws a moment between two of
 * them, so 20 frames a second play back smoothly at the display's rate.
 */
export class SceneBuffer {
  private states: Timed[] = [];
  private latest: SceneState | null = null;
  private waiting = false;

  get current() {
    return this.latest;
  }

  /** A delta went missing and the scene is waiting for a whole frame. */
  get stalled() {
    return this.waiting;
  }

  reset() {
    this.states = [];
    this.latest = null;
    this.waiting = false;
  }

  /** Returns false when the delta could not be applied. */
  push(delta: FrameDelta): boolean {
    const next = applyDelta(this.latest, delta);
    if (!next) {
      this.waiting = true;
      return false;
    }
    this.waiting = false;
    // A new round starts its clock again; what came before it is not between anything.
    if (this.latest && next.t < this.latest.t) this.states = [];
    this.latest = next;
    this.states.push({ state: next, played: false });
    if (this.states.length > KEEP) this.states.shift();
    return true;
  }

  /** Effects of the frames the drawn moment has reached, each handed out once. */
  takeEffects(renderTime: number): Effect[] {
    const due: Effect[] = [];
    for (const timed of this.states) {
      if (timed.played || timed.state.t > renderTime) continue;
      timed.played = true;
      due.push(...timed.state.effects);
    }
    return due;
  }

  /** The scene at game time `renderTime`, between the frames on either side of it. */
  sample(renderTime: number): Sample | null {
    if (!this.states.length) return null;
    let before = this.states[0].state;
    let after = before;
    for (const { state } of this.states) {
      after = state;
      if (state.t >= renderTime) break;
      before = state;
    }
    if (renderTime < before.t) after = before;
    const span = after.t - before.t;
    const t = span > 0 ? Math.min(1, Math.max(0, (renderTime - before.t) / span)) : 1;
    const entities: Entity[] = [];
    const speeds = new Map<string, number>();
    for (const [id, next] of after.entities) {
      const previous = before.entities.get(id);
      if (!previous || previous.kind !== next.kind) {
        entities.push(next);
        continue;
      }
      const moved = Math.hypot(next.x - previous.x, next.z - previous.z);
      if (moved > TELEPORT) {
        entities.push(t < 0.5 ? previous : next);
        continue;
      }
      entities.push(blendEntity(previous, next, t));
      if (next.kind === 'player' && span > 0) speeds.set(id, moved / span);
    }
    return { state: t < 1 ? before : after, entities, speeds };
  }
}
