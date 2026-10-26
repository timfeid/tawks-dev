// These mirror games/party/src/fpmove.rs, so the view of your own movement
// keeps up with the keys instead of waiting for the next snapshot.
export const MAX_SPEED = 7;
export const CROUCH_SPEED = 0.42;
export const CROUCH_EYE = 0.95;
const ACCEL = 60;
const FRICTION = 8;
const AIR_CONTROL = 0.35;
const JUMP_SPEED = 5.4;
const GRAVITY = 16;
const STEP_FORGIVENESS = 0.2;
const PLAYER_HEIGHT = 1.7;
const CROUCH_HEIGHT = 1.15;

/** `bottom` above zero floats the block, like a bridge or a canopy you can walk under. */
export type Block = { x: number; z: number; hx: number; hz: number; top: number; bottom?: number };

/** The floor a player moves over: its blocks, bounds and their own size. */
export type Court = { blocks: Block[]; radius: number; halfX: number; halfZ: number };

export type LocalInput = { x: number; z: number; yaw: number; pitch: number; jump: boolean; crouch: boolean };

export type LocalState = {
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  onGround: boolean;
};

/** One step of the same movement the server runs, for your own player. */
export function stepLocal(state: LocalState, input: LocalInput, dt: number, court: Court) {
  const sy = Math.sin(input.yaw);
  const cy = Math.cos(input.yaw);
  let wishX = -sy * -input.z + cy * input.x;
  let wishZ = -cy * -input.z - sy * input.x;
  const wish = Math.hypot(wishX, wishZ);
  const control = state.onGround ? 1 : AIR_CONTROL;
  const topSpeed = input.crouch && state.onGround ? MAX_SPEED * CROUCH_SPEED : MAX_SPEED;
  if (wish > 1e-3) {
    wishX /= wish;
    wishZ /= wish;
    const along = state.vx * wishX + state.vz * wishZ;
    const add = Math.max(0, topSpeed * Math.min(1, wish) - along);
    const step = Math.min(ACCEL * control * dt, add);
    state.vx += wishX * step;
    state.vz += wishZ * step;
  }
  if (state.onGround) {
    const speed = Math.hypot(state.vx, state.vz);
    if (speed > 0) {
      const keep = Math.max(0, speed - speed * FRICTION * dt) / speed;
      state.vx *= keep;
      state.vz *= keep;
    }
    if (input.jump && !input.crouch) {
      state.vy = JUMP_SPEED;
      state.onGround = false;
    }
  }
  state.vy -= GRAVITY * dt;
  const feetBefore = state.y;
  const height = input.crouch ? CROUCH_HEIGHT : PLAYER_HEIGHT;
  state.x += state.vx * dt;
  state.z += state.vz * dt;
  state.y += state.vy * dt;

  let floor = 0;
  for (const block of court.blocks) {
    const inside =
      Math.abs(state.x - block.x) <= block.hx + court.radius && Math.abs(state.z - block.z) <= block.hz + court.radius;
    if (inside && feetBefore + 1e-3 >= block.top - STEP_FORGIVENESS && block.top > floor) floor = block.top;
  }
  if (state.y <= floor) {
    state.y = floor;
    state.vy = 0;
    state.onGround = true;
  } else {
    state.onGround = false;
  }

  for (const block of court.blocks) {
    const bottom = block.bottom ?? 0;
    if (
      bottom > 0 &&
      state.vy > 0 &&
      Math.abs(state.x - block.x) <= block.hx + court.radius * 0.5 &&
      Math.abs(state.z - block.z) <= block.hz + court.radius * 0.5 &&
      feetBefore + height <= bottom + 0.05 &&
      state.y + height > bottom
    ) {
      state.y = bottom - height;
      state.vy = 0;
    }
  }

  for (const block of court.blocks) {
    if (state.y >= block.top - 0.01 || state.y + height <= (block.bottom ?? 0)) continue;
    const nearestX = Math.min(Math.max(state.x, block.x - block.hx), block.x + block.hx);
    const nearestZ = Math.min(Math.max(state.z, block.z - block.hz), block.z + block.hz);
    const dx = state.x - nearestX;
    const dz = state.z - nearestZ;
    const distance = Math.hypot(dx, dz);
    if (distance > court.radius) continue;
    if (distance > 1e-4) {
      const push = court.radius - distance;
      state.x += (dx / distance) * push;
      state.z += (dz / distance) * push;
      const into = state.vx * (dx / distance) + state.vz * (dz / distance);
      if (into < 0) {
        state.vx -= (dx / distance) * into;
        state.vz -= (dz / distance) * into;
      }
    } else {
      state.x = block.x + block.hx + court.radius;
    }
  }

  const boundX = court.halfX - court.radius;
  const boundZ = court.halfZ - court.radius;
  if (Math.abs(state.x) > boundX) {
    state.x = Math.min(Math.max(state.x, -boundX), boundX);
    state.vx = 0;
  }
  if (Math.abs(state.z) > boundZ) {
    state.z = Math.min(Math.max(state.z, -boundZ), boundZ);
    state.vz = 0;
  }
}
