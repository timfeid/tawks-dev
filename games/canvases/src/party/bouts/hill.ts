import { lerp, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { GameEvent } from '../protocol';
import { HillField, type HillSetup, type HillSnap } from '../hill/field';
import { asMembers, rosterKeeper } from './common';

/** The lobby has no server setup yet, so picking it shows the hill as it plays. */
const PREVIEW: HillSetup = { game: 'hill', radius: 20, peak: 7.5, summit: 3.6 };

/** King of the Hill, played on a peak cut from the island. */
const hill: BoutModule<HillSnap, HillSetup, GameEvent> = {
  create(host): BoutRenderer<HillSnap, HillSetup, GameEvent> {
    const field = new HillField(host.scene);
    const roster = rosterKeeper(host);
    return {
      fog: [90, 260],
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup ?? PREVIEW, asMembers(view.seats), roster(view.seats));
        field.update(view.snapshot, view.time, view.dt);
      },
      effect(event) {
        if (event.kind === 'off') host.sounds.fall();
        else if (event.kind === 'shove') host.sounds.splat();
      },
      shot(view) {
        // A bigger field plays on a bigger hill, so the shot widens with it.
        const half = (view.setup ?? PREVIEW).radius + 6;
        return { kind: 'frame', half: [half, half], pitch: 46, portraitPitch: 58, lookHeight: 3 };
      },
      hud: (view) => ({
        objective: 'Hold the summit',
        hint: 'Move with WASD. The slope pushes you off, so keep walking into it.',
        scores: view.snapshot?.players.map((player) => ({
          seat: player.seat,
          hits: Math.round(player.held),
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
      // Coming back on after sliding off is a jump, so there is nothing to blend.
      if (!prev || prev.out !== next.out) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        z: lerp(prev.z, next.z, t),
        y: lerp(prev.y, next.y, t),
        held: lerp(prev.held, next.held, t),
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },
};

export default hill;
