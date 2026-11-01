import { lerp, type BoutFrame, type BoutHud, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { ArenaSetup, ArenaSnap, GameEvent } from '../protocol';
import { PartyArena } from '../arena/scene';
import { LedgeFloor } from '../arena/ledge';
import { ledgeHint, ledgeReach } from '../arena/ledge-layout';
import { inputDevice, padButton } from '../input-device';
import { keyName, settings } from '../settings';
import { asMembers, rosterKeeper } from './common';

type View = BoutFrame<ArenaSnap, ArenaSetup>;

function hint() {
  const { keys, pad } = settings();
  const move = [keys.up[0], keys.left[0], keys.down[0], keys.right[0]].map((code) => (code ? keyName(code) : '—'));
  return ledgeHint(move, keyName(keys.action[0] ?? 'Space'), inputDevice() === 'pad' ? padButton(pad.action) : null);
}

function alert(view: View) {
  const snap = view.snapshot;
  const setup = view.setup?.game === 'ledge' ? view.setup : null;
  if (!snap || !setup) return null;
  if ((snap.core ?? setup.core) < setup.core - 0.01) return 'The last stone is wearing away';
  if ((snap.sections ?? []).some((state) => state === 1)) return 'The ledge is cracking';
  return null;
}

function hud(view: View): BoutHud {
  return {
    objective: 'Shove them off · last one up wins',
    hint: hint(),
    action: 'Shove',
    alert: alert(view),
    scores: (view.snapshot?.players ?? []).map((player) => ({
      seat: player.seat,
      hits: player.knockoffs ?? 0,
      up: !player.out,
    })),
  };
}

/** Ledge Shuffle: the floor games' riders on a crumbling ledge of its own. */
const ledge: BoutModule<ArenaSnap, ArenaSetup, GameEvent> = {
  create(host): BoutRenderer<ArenaSnap, ArenaSetup, GameEvent> {
    const arena = new PartyArena(host.scene);
    const floor = new LedgeFloor(arena.group);
    const roster = rosterKeeper(host);
    let half = 0;
    return {
      frame(view) {
        arena.visible = true;
        if (view.setup?.game === 'ledge') floor.build(view.setup);
        arena.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats));
        arena.update(view.snapshot, view.time, view.dt);
        floor.update(view.snapshot, view.mySeat, view.time, view.dt);
      },
      effect(event, view) {
        arena.effect(event, asMembers(view.seats));
        floor.effect(event, view.time);
        const { sounds } = host;
        if (event.kind === 'ledge_windup' && event.seat === view.mySeat) sounds.step(0.5);
        else if (event.kind === 'ledge_hit') sounds.bump(1.4);
        else if (event.kind === 'ledge_block') sounds.bump(0.45);
        else if (event.kind === 'ledge_shove' && !event.landed && !event.blocked && event.seat === view.mySeat) sounds.step(0.3);
        else if (event.kind === 'ledge_crack') sounds.beep(0.3);
        else if (event.kind === 'ledge_fall') sounds.step(0.9);
        else if (event.kind === 'over') sounds.fall();
      },
      shot(view) {
        const setup = view.setup?.game === 'ledge' ? view.setup : null;
        const reach = setup ? ledgeReach(setup.rings, setup.core, view.snapshot?.sections) : 12;
        // The shot closes in as the rings fall, without jumping when one goes.
        half = half === 0 ? reach : lerp(half, reach, Math.min(1, view.dt * 1.5));
        const framed = Math.max(half, 6);
        return { kind: 'frame', half: [framed + 2, framed + 2], pitch: 56, portraitPitch: 66 };
      },
      shake: () => Math.max(arena.shake, floor.shake),
      hud,
      dispose() {
        floor.dispose();
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
    const core = a.core === undefined || b.core === undefined ? b.core : lerp(a.core, b.core, t);
    return { ...b, time: lerp(a.time, b.time, t), core, players };
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

export default ledge;
