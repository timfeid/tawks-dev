import { defineGame, ring, body, steer, dash, collide, dist, nearest, seek, flee, rankOuts } from '@tawks/made-kit';

const START_R = 11;
const END_R = 3;
const SHRINK_S = 50;
const LIVES = 2;
const SAFE_S = 1.5;

const radiusAt = (t) => START_R - (START_R - END_R) * Math.min(1, t / SHRINK_S);

export default defineGame({
  meta: {
    name: 'Thin Ice',
    players: [2, 8],
    controls: 'move_action',
    seconds: 90,
    goal: 'Shove the others into the sea and stay on the ice',
    win: 'The last player on the ice wins',
    lose: 'Fall in twice and you are out',
    rules: 'Dash into people to knock them off the floe. It shrinks as the round goes on, and standing still for two seconds cracks the ice under you. You get one fall for free.',
  },

  setup(ctx) {
    const spots = ring(ctx.seats.length, 6);
    return {
      radius: START_R,
      cracks: [],
      fell: null,
      players: ctx.seats.map((s, i) => ({
        seat: s.seat,
        ...body(spots[i].x, spots[i].z, { r: 0.6 * s.body.radius, mass: s.body.mass }),
        lives: LIVES,
        safe: 0,
        cooldown: 0,
        still: 0,
        out: null,
      })),
    };
  },

  tick(state, ctx) {
    state.radius = radiusAt(ctx.time);
    for (const c of state.cracks) c.t += ctx.dt;
    const alive = state.players.filter((p) => p.out === null);
    for (const p of alive) {
      const input = ctx.input(p.seat);
      steer(p, input.move, ctx.dt, { speed: 7, accel: 18, friction: 0.6 });
      p.cooldown = Math.max(0, p.cooldown - ctx.dt);
      p.safe = Math.max(0, p.safe - ctx.dt);
      if (input.pressed('action') && p.cooldown === 0) {
        dash(p, input.move, 11);
        p.cooldown = 1.2;
        ctx.emit({ kind: 'dash' });
      }
      p.still = Math.hypot(p.vx, p.vz) < 0.5 ? p.still + ctx.dt : 0;
      if (p.still > 2 && state.cracks.length < 40) {
        state.cracks.push({ x: p.x, z: p.z, r: 1.2, t: 0 });
        p.still = 0;
      }
    }
    collide(alive, { bounce: 0.6 });
    for (const p of alive) {
      if (p.safe > 0) continue;
      const sunk = Math.hypot(p.x, p.z) > state.radius || state.cracks.some((c) => c.t > 1 && dist(p, c) < c.r);
      if (!sunk) continue;
      p.lives -= 1;
      state.fell = { seat: p.seat, t: ctx.time, out: p.lives === 0 };
      ctx.emit({ kind: 'splash', x: p.x, z: p.z, out: p.lives === 0 });
      if (p.lives === 0) {
        p.out = ctx.time;
      } else {
        const a = ctx.rand() * Math.PI * 2;
        p.x = Math.sin(a) * 1.5;
        p.z = Math.cos(a) * 1.5;
        p.vx = 0;
        p.vz = 0;
        p.safe = SAFE_S;
      }
    }
    if (alive.filter((p) => p.out === null).length <= 1 || ctx.time >= 90) ctx.end(rankOuts(state.players));
  },

  bot(state, seat, ctx) {
    const me = state.players.find((p) => p.seat === seat);
    if (me.out !== null) return {};
    const skill = ctx.skill(seat);
    const crack = nearest(me, state.cracks);
    if (crack && dist(me, crack) < crack.r + 0.8) return { move: flee(me, crack, skill) };
    const r = Math.hypot(me.x, me.z) || 1;
    const outSpeed = Math.max(0, (me.vx * me.x + me.vz * me.z) / r);
    if (r > state.radius - 2 - outSpeed * 0.4) return { move: seek(me, { x: 0, z: 0 }, skill) };
    const foe = nearest(me, state.players.filter((p) => p.out === null && p.seat !== seat && p.safe === 0));
    if (!foe) return { move: seek(me, { x: Math.sin(ctx.time) * 3, z: Math.cos(ctx.time) * 3 }, skill) };
    // Dash only when the foe is between me and the water, so the shove sends it off.
    const outward = (foe.x - me.x) * foe.x + (foe.z - me.z) * foe.z > 0;
    const close = dist(me, foe) < 2.6;
    return { move: seek(me, foe, skill), action: close && outward && ctx.rand() < 0.02 + 0.08 * skill };
  },

  view(state, v) {
    v.water();
    v.camera.frame(START_R + 1, START_R + 1, 55);
    v.disc('rim', 0, 0, state.radius + 0.25, { color: 'danger', height: 0.36 });
    v.disc('floe', 0, 0, state.radius, { color: 'ice', height: 0.4 });
    state.cracks.forEach((c, i) =>
      v.disc(`crack${i}`, c.x, c.z, c.r, { color: c.t > 1 ? 'water' : 'danger', y: 0.41, height: 0.02 }),
    );
    for (const p of state.players) {
      const pose = p.out !== null ? 'fall' : p.safe > 0 ? 'wave' : 'ride';
      v.player(p.seat, p, { pose, y: 0.4, hidden: p.out !== null && v.time - p.out > 1.5 });
    }
    // The app announces outs itself; the game names the first fall.
    const fell = state.fell && !state.fell.out && v.time - state.fell.t < 2 ? state.fell : null;
    const name = fell ? (v.seats.find((s) => s.seat === fell.seat) || { name: 'Someone' }).name : '';
    v.hud({
      objective: 'Shove them off. Keep moving or the ice cracks.',
      alert: fell ? `${name} fell in, one life left` : state.radius < 6 ? 'The ice is almost gone' : null,
      timer: Math.max(0, 90 - v.time),
      scores: state.players.map((p) => ({ seat: p.seat, up: p.out === null, label: p.out === null ? (p.lives === 1 ? '1 life' : `${p.lives} lives`) : null })),
    });
  },

  effect(e, v) {
    if (e.kind === 'splash') {
      v.sound(e.out ? 'lose' : 'splash');
      v.burst(e.x, e.z, 'water');
    } else v.sound('thud');
  },
});
