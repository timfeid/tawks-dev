import { lerp, turn, type BoutFrame, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { DodgeballSetup, DodgeballSnap, GameEvent } from '../protocol';
import { DodgeballCourt } from '../dodgeball/court';
import { asMembers, inArena, rosterKeeper } from './common';

type View = BoutFrame<DodgeballSnap, DodgeballSetup>;

const dodgeball: BoutModule<DodgeballSnap, DodgeballSetup, GameEvent> = {
  create(host): BoutRenderer<DodgeballSnap, DodgeballSetup, GameEvent> {
    const court = new DodgeballCourt(host.scene);
    const roster = rosterKeeper(host);
    const onCourt = (view: View) => view.snapshot?.players.find((p) => p.seat === view.mySeat) ?? null;

    return {
      fog: [100, 260],
      frame(view) {
        court.firstPerson = view.playing && !!onCourt(view) && inArena(view.phase);
        court.visible = true;
        court.sync(view.snapshot, view.setup, asMembers(view.seats), roster(view.seats), view.mySeat);
        court.control({ x: view.stick.x, z: view.stick.z, ...view.look }, view.dt);
        court.viewModel(host.camera);
        court.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        court.effect(event, view.mySeat, asMembers(view.seats));
        const { sounds } = host;
        if (event.kind === 'throw') sounds.shot();
        else if (event.kind === 'bounce') sounds.splat();
        else if (event.kind === 'catch') sounds.win();
        else if (event.kind === 'out') sounds.down(event.seat === view.mySeat);
      },
      shot(view) {
        const eye = court.firstPerson ? court.eye(view.look.yaw, view.look.pitch) : null;
        if (eye) return { kind: 'eye', ...eye };
        return { kind: 'frame', half: [23, 20], pitch: 52, portraitPitch: 64, lookHeight: 0 };
      },
      shake: () => court.shake,
      facing(view) {
        const me = onCourt(view);
        return me && !me.out ? me.yaw : null;
      },
      hud(view) {
        const me = onCourt(view);
        return {
          objective: 'Catch, throw · be the last standing',
          combat: me
            ? {
                meter: 'ball',
                hp: 0,
                hpMax: 0,
                hits: me.hits,
                downs: me.outs,
                up: !me.out,
                back: me.back_in,
                holding: me.holding,
                hitsLabel: 'hits',
                outTitle: 'You are out',
              }
            : null,
          scores: (view.snapshot?.players ?? []).map((p) => ({ seat: p.seat, hits: p.hits, downs: p.outs, up: !p.out })),
          hint: 'Click to take aim. WASD moves, walk over a ball to pick it up, click to throw or catch.',
          action: 'Throw',
        };
      },
      dispose() {
        court.reset();
        court.dispose();
      },
    };
  },

  blend(a, b, t) {
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      // Coming back on court is a jump, so there is nothing to blend through.
      if (!prev || prev.out !== next.out) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        y: lerp(prev.y, next.y, t),
        z: lerp(prev.z, next.z, t),
        yaw: turn(prev.yaw, next.yaw, t),
        pitch: lerp(prev.pitch, next.pitch, t),
      };
    });
    const balls = b.balls.map((next) => {
      const prev = a.balls.find((ball) => ball.id === next.id);
      if (!prev || prev.state !== next.state) return next;
      return { ...next, x: lerp(prev.x, next.x, t), y: lerp(prev.y, next.y, t), z: lerp(prev.z, next.z, t) };
    });
    return { ...b, time: lerp(a.time, b.time, t), players, balls };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.out ? player : { ...player, x: player.x + player.vx * dt, z: player.z + player.vz * dt },
      ),
    };
  },
};

export default dodgeball;
