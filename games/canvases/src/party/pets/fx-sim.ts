import FX from './fx.json' with { type: 'json' };

/** Particle effects around a pet, from fx.json: bursts for moves and milestones, ambience for its stage, and a store effect it wears. The native game's pet_fx.rs reads the same file and simulates it the same way. Positions and sizes are in pet heights, feet at y = 0, facing +z. */
export type Glyph = 'heart' | 'star' | 'sparkle' | 'dot' | 'bang' | 'ring' | 'bubble';
export const GLYPHS: Glyph[] = ['heart', 'star', 'sparkle', 'dot', 'bang', 'ring', 'bubble'];

export type Emitter = {
  glyph: Glyph;
  colors: string[];
  /** A burst makes `count` at once; ambience makes `rate` a second. */
  count?: number;
  rate?: number;
  /** Made only while the pet walks; standing, a trail barely trickles. */
  move?: boolean;
  life: [number, number];
  at: [number, number, number];
  radius: number;
  /** Starts on the circle `radius` out, round the pet rather than inside it. */
  shell?: boolean;
  speed: [number, number];
  dir: 'up' | 'out' | 'ring' | 'back' | 'swirl' | 'orbit';
  spread?: number;
  gravity?: number;
  drag?: number;
  size: [number, number];
  /** How big it ends, against how big it starts. */
  grow?: number;
  alpha?: number;
  /** Lies on the ground, like a ring. */
  flat?: boolean;
  /** Turns round the pet, in radians a second, for swirls and orbits. */
  spin?: number;
  orbit?: number;
  delay?: number;
};

export type FxName = keyof typeof FX.bursts;
export const FX_MAX: number = FX.max;
/** Sprites drawn this much bigger than fx.json sizes, so they read in a small card; rings on the ground are not. */
export const SPRITE_SCALE: number = FX.spriteScale;
export const FX_BURSTS = FX.bursts as unknown as Record<FxName, Emitter[]>;
export const FX_AMBIENT = FX.ambient as unknown as Record<string, Emitter[]>;
export const FX_COSMETICS = FX.cosmetics as unknown as Record<string, Emitter[]>;
/** How much of a walking trail a pet that stands still still makes. */
export const STANDING_TRAIL = 0.35;
/** How far out a standing pet's trail floats. */
export const STANDING_RING = 0.55;

export type Particle = {
  x: number;
  y: number;
  z: number;
  vx: number;
  vy: number;
  vz: number;
  age: number;
  life: number;
  size: number;
  grow: number;
  alpha: number;
  color: string;
  glyph: Glyph;
  flat: boolean;
  spin: number;
  orbit: number;
  angle: number;
  gravity: number;
  drag: number;
  swirl: boolean;
};

/** The ambience a stage and a worn effect make: motes at Best friend, an aura at Legend, then the effect. */
export function ambienceFor(stage: number, effect: string | null | undefined): Emitter[] {
  const tiers = Object.keys(FX_AMBIENT).map(Number).filter((tier) => tier <= stage).sort((a, b) => b - a);
  return [...(tiers.length ? FX_AMBIENT[String(tiers[0])] : []), ...((effect && FX_COSMETICS[effect]) || [])];
}

const between = (rand: () => number, [low, high]: [number, number]) => low + (high - low) * rand();

export class FxSim {
  particles: Particle[] = [];
  private ambient: { emitter: Emitter; carry: number }[] = [];
  private waiting: { emitter: Emitter; left: number }[] = [];

  private rand: () => number;
  private max: number;

  constructor(rand: () => number = Math.random, max = FX_MAX) {
    this.rand = rand;
    this.max = max;
  }

  setAmbient(emitters: Emitter[]) {
    this.ambient = emitters.map((emitter) => ({ emitter, carry: 0 }));
  }

  burst(name: FxName) {
    for (const emitter of FX_BURSTS[name] ?? []) {
      if (emitter.delay) this.waiting.push({ emitter, left: emitter.delay });
      else this.spawn(emitter, emitter.count ?? 1);
    }
  }

  /** `walking` is 0 standing to 1 at a walk, for trails. */
  step(dt: number, walking = 0) {
    for (const wait of this.waiting) {
      wait.left -= dt;
      if (wait.left <= 0) this.spawn(wait.emitter, wait.emitter.count ?? 1);
    }
    this.waiting = this.waiting.filter((wait) => wait.left > 0);
    for (const source of this.ambient) {
      const rate = (source.emitter.rate ?? 0) * (source.emitter.move ? Math.max(walking, STANDING_TRAIL) : 1);
      source.carry += rate * dt;
      const whole = Math.floor(source.carry);
      source.carry -= whole;
      if (whole) this.spawn(source.emitter, whole, walking);
    }
    for (const p of this.particles) {
      p.age += dt;
      if (p.orbit) {
        p.angle += p.spin * dt;
        p.x = Math.sin(p.angle) * p.orbit;
        p.z = Math.cos(p.angle) * p.orbit;
        continue;
      }
      const slow = Math.exp(-p.drag * dt);
      p.vx *= slow;
      p.vy = p.vy * slow + p.gravity * dt;
      p.vz *= slow;
      if (p.swirl) {
        const turn = p.spin * dt;
        const [x, z] = [p.x, p.z];
        p.x = x * Math.cos(turn) + z * Math.sin(turn);
        p.z = z * Math.cos(turn) - x * Math.sin(turn);
      }
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.z += p.vz * dt;
    }
    this.particles = this.particles.filter((p) => p.age < p.life);
  }

  private spawn(emitter: Emitter, count: number, walking = 1) {
    // A trail with nowhere to go floats round a standing pet instead of hiding behind it.
    if (emitter.dir === 'back' && walking < 0.5)
      emitter = { ...emitter, dir: 'up', spread: 0.6, shell: true, radius: STANDING_RING, at: [0, emitter.at[1], 0] };
    for (let i = 0; i < count; i++) {
      if (this.particles.length >= this.max) this.particles.shift();
      const rand = this.rand;
      const azimuth = rand() * Math.PI * 2;
      const reach = emitter.radius * (emitter.shell ? 1 : Math.sqrt(rand()));
      const speed = between(rand, emitter.speed);
      let [dx, dy, dz] = [0, 1, 0];
      if (emitter.dir === 'up') {
        const tilt = (emitter.spread ?? 0) * rand();
        [dx, dy, dz] = [Math.sin(tilt) * Math.cos(azimuth), Math.cos(tilt), Math.sin(tilt) * Math.sin(azimuth)];
      } else if (emitter.dir === 'out') {
        const up = 0.2 + rand() * 0.8;
        const flat = Math.sqrt(1 - up * up);
        [dx, dy, dz] = [flat * Math.cos(azimuth), up, flat * Math.sin(azimuth)];
      } else if (emitter.dir === 'ring' || emitter.dir === 'swirl') {
        [dx, dy, dz] = [Math.cos(azimuth), emitter.dir === 'swirl' ? 0.3 : 0, Math.sin(azimuth)];
      } else if (emitter.dir === 'back') {
        [dx, dy, dz] = [(rand() - 0.5) * 0.6, 0.3, -1];
      }
      const ring = emitter.dir === 'swirl' ? emitter.radius : reach;
      this.particles.push({
        x: emitter.at[0] + Math.cos(azimuth) * ring,
        y: emitter.at[1] + (emitter.flat ? 0 : (rand() - 0.5) * emitter.radius * 0.5),
        z: emitter.at[2] + Math.sin(azimuth) * ring,
        vx: dx * speed,
        vy: dy * speed,
        vz: dz * speed,
        age: 0,
        life: between(rand, emitter.life),
        size: between(rand, emitter.size),
        grow: emitter.grow ?? 1,
        alpha: emitter.alpha ?? 1,
        color: emitter.colors[Math.floor(rand() * emitter.colors.length)] ?? '#ffffff',
        glyph: emitter.glyph,
        flat: !!emitter.flat,
        spin: emitter.spin ?? 0,
        orbit: emitter.dir === 'orbit' ? (emitter.orbit ?? 0.3) : 0,
        angle: azimuth,
        gravity: emitter.gravity ?? 0,
        drag: emitter.drag ?? 0,
        swirl: emitter.dir === 'swirl',
      });
      if (emitter.dir === 'orbit') this.particles[this.particles.length - 1].y = emitter.at[1];
    }
  }
}

/** How opaque a particle is through its life: a quick fade in, a long fade out. */
export function fade(p: Particle) {
  const t = p.age / p.life;
  return p.alpha * Math.min(1, t / 0.1) * Math.min(1, (1 - t) / 0.4);
}

export function sizeOf(p: Particle) {
  return p.size * (1 + (p.grow - 1) * (p.age / p.life));
}

/** The burst a move makes. */
export function moveBurst(move: string): FxName | null {
  return move in FX_BURSTS ? (move as FxName) : null;
}
