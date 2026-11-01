import { lerp, type BoutFrame, type BoutModule, type BoutRenderer, type Shot } from '@tawks/bout-sdk';
import type { GameEvent } from '../protocol';
import { RisingField, type RisingSetup, type RisingSnap } from '../rising/field';
import { asMembers, rosterKeeper } from './common';

/** How far back and above the camera sits from the height it is watching,
 * for the tank as built for four. A wider tank pulls the camera back with it. */
const BACK = 27;
const ABOVE = 13;
const RADIUS = 9.5;

/** The height worth watching: your own climber, or the highest one still dry. */
function focusHeight(view: BoutFrame<RisingSnap, RisingSetup>) {
  const snap = view.snapshot;
  if (!snap) return 0;
  const mine = snap.players.find((player) => player.seat === view.mySeat && !player.out);
  const up = snap.players.filter((player) => !player.out);
  const height = mine ? mine.y : up.length ? Math.max(...up.map((player) => player.y)) : snap.water;
  return Math.max(height, snap.water, 0) + 1;
}

/** Rising Water: a flooding tank with a spiral of crumbling platforms up the middle. */
const rising: BoutModule<RisingSnap, RisingSetup, GameEvent> = {
  create(host): BoutRenderer<RisingSnap, RisingSetup, GameEvent> {
    const field = new RisingField(host.scene);
    const roster = rosterKeeper(host);
    let watching: number | null = null;
    return {
      fog: [80, 220],
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats));
        field.update(view.snapshot, view.time);
      },
      effect(event, view) {
        if (event.kind === 'dunked') host.sounds.splash();
        else if (event.kind === 'crumble') host.sounds.step(0.5);
        else if (event.kind === 'jump' && event.seat === view.mySeat) host.sounds.bump(0.3);
      },
      shot(view): Shot {
        const want = focusHeight(view);
        watching = watching === null ? want : lerp(watching, want, 1 - Math.exp(-view.dt * 2.5));
        const away = (view.setup?.radius ?? RADIUS) / RADIUS;
        return { kind: 'eye', x: 0, y: watching + ABOVE * away, z: BACK * away, yaw: 0, pitch: -Math.atan2(ABOVE, BACK) };
      },
      hud: (view) => ({
        objective: 'Stay above the water',
        hint: 'Move with WASD. Press space or click to jump. Platforms crumble once you land, so keep climbing.',
        alert: (() => {
          const snap = view.snapshot;
          const me = snap?.players.find((player) => player.seat === view.mySeat);
          return snap && me && !me.out && me.y - snap.water < 1.5 ? 'The water is at your feet' : null;
        })(),
        scores: view.snapshot?.players.map((player) => ({
          seat: player.seat,
          hits: Math.max(0, Math.round(player.y)),
          up: !player.out,
        })),
      }),
      dispose() {
        field.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev || prev.out !== next.out) return next;
      return { ...next, x: lerp(prev.x, next.x, t), y: lerp(prev.y, next.y, t), z: lerp(prev.z, next.z, t) };
    });
    return { ...b, time: lerp(a.time, b.time, t), water: lerp(a.water, b.water, t), players };
  },
};

export default rising;
