import { lerp, turn, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { BattleSetup, BattleSnap, GameEvent } from '../protocol';
import { BattleField } from '../battle/scene';
import { asMembers, rosterKeeper } from './common';

const OBJECTIVES: Record<string, string> = {
  tank: 'Bounce shells off the walls · most wrecks wins',
  titan: 'Dodge the wind-up · strike the core while it is winded',
};

const HINTS: Record<string, string> = {
  tank: 'Drive with WASD, aim the turret with the mouse, click to fire.',
  titan: 'Move with WASD. Dodge the wind-up, then click to strike the core.',
};

/** Tank Clash and Titan Siege share one yard. */
const battle: BoutModule<BattleSnap, BattleSetup, GameEvent> = {
  create(host): BoutRenderer<BattleSnap, BattleSetup, GameEvent> {
    const field = new BattleField(host.scene);
    const roster = rosterKeeper(host);
    return {
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats));
        field.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        field.effect(event, asMembers(view.seats));
        const { sounds } = host;
        if (event.kind === 'shell') sounds.shot();
        else if (event.kind === 'wreck') sounds.blast();
        else if (event.kind === 'ricochet' || event.kind === 'splinter') sounds.splat();
        else if (event.kind === 'strike') sounds.bump(1);
        else if (event.kind === 'wind_up') sounds.count(false);
        else if (event.kind === 'strike_core') sounds.splat();
        else if (event.kind === 'titan_down') sounds.win();
        else if (event.kind === 'floored') sounds.fall();
      },
      shot(view) {
        const setup = view.setup;
        const half = setup && 'half' in setup ? setup.half : (setup?.radius ?? 20);
        return { kind: 'frame', half: [half + 3, half + 3], pitch: 52, portraitPitch: 64 };
      },
      shake: () => field.shake,
      aim(point, view) {
        const me = view.snapshot?.players.find((p) => p.seat === view.mySeat);
        return me ? Math.atan2(-(point.x - me.x), -(point.z - me.z)) : null;
      },
      hud: (view) => ({ objective: OBJECTIVES[view.key], hint: HINTS[view.key] }),
      dispose() {
        field.reset();
        field.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      // Coming back out of a wreck is a jump, so there is nothing to blend.
      if (!prev || prev.out !== next.out) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        z: lerp(prev.z, next.z, t),
        hull: next.hull === undefined ? undefined : turn(prev.hull ?? next.hull, next.hull, t),
        turret: next.turret === undefined ? undefined : turn(prev.turret ?? next.turret, next.turret, t),
      };
    });
    const shells = (b.shells ?? []).map((next) => {
      const prev = (a.shells ?? []).find((shell) => shell.id === next.id);
      if (!prev) return next;
      return { ...next, x: lerp(prev.x, next.x, t), z: lerp(prev.z, next.z, t) };
    });
    const titan =
      a.titan && b.titan
        ? { ...b.titan, x: lerp(a.titan.x, b.titan.x, t), z: lerp(a.titan.z, b.titan.z, t), facing: turn(a.titan.facing, b.titan.facing, t) }
        : b.titan;
    return { ...b, time: lerp(a.time, b.time, t), players, shells, titan };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.out ? player : { ...player, x: player.x + (player.vx ?? 0) * dt, z: player.z + (player.vz ?? 0) * dt },
      ),
      shells: (snap.shells ?? []).map((shell) => ({ ...shell, x: shell.x + shell.vx * dt, z: shell.z + shell.vz * dt })),
    };
  },
};

export default battle;
