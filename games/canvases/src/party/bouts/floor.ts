import { lerp, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { ArenaSetup, ArenaSnap, GameEvent } from '../protocol';
import { PartyArena } from '../arena/scene';
import { asMembers, rosterKeeper } from './common';

const HINTS: Record<string, string> = {
  crumble: 'Move with WASD. Standing still cracks the tile under you.',
  hotpotato: 'Move with WASD. Run into someone to hand the bomb on.',
  balloon: 'Move with WASD. Click or press space to dash and burst a balloon.',
  ledge: 'Move with WASD. When the horn sounds, stand on a ledge.',
};

/** Crumble Run, Hot Potato Panic, Balloon Brawl and Ledge Shuffle share one floor. */
const floor: BoutModule<ArenaSnap, ArenaSetup, GameEvent> = {
  create(host): BoutRenderer<ArenaSnap, ArenaSetup, GameEvent> {
    const arena = new PartyArena(host.scene);
    const roster = rosterKeeper(host);
    return {
      frame(view) {
        arena.visible = true;
        arena.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats));
        arena.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        arena.effect(event, asMembers(view.seats));
        const { sounds } = host;
        if (event.kind === 'boom') sounds.blast();
        else if (event.kind === 'pop') sounds.splat();
        else if (event.kind === 'horn') sounds.count(true);
        else if (event.kind === 'over' || event.kind === 'drop') sounds.fall();
        else if (event.kind === 'pass') sounds.bump(0.5);
        else if (event.kind === 'tile_fall') sounds.step(0.5);
      },
      shot(view) {
        const setup = view.setup;
        const half = setup && 'radius' in setup ? setup.radius : setup && 'half' in setup ? setup.half : 14;
        return { kind: 'frame', half: [half + 2, half + 2], pitch: 54, portraitPitch: 64 };
      },
      shake: () => arena.shake,
      hud: (view) => ({ objective: 'Outlast the field', hint: HINTS[view.key] }),
      dispose() {
        arena.reset();
        arena.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev || prev.out !== next.out) return next;
      return { ...next, x: lerp(prev.x, next.x, t), y: lerp(prev.y ?? 0, next.y ?? 0, t), z: lerp(prev.z, next.z, t) };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.out ? player : { ...player, x: player.x + (player.vx ?? 0) * dt, z: player.z + (player.vz ?? 0) * dt },
      ),
    };
  },
};

export default floor;
