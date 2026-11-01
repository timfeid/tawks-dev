import { lerp, turn, type BoutFrame, type BoutHud, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import { GhostHall, type GhostsEvent, type GhostsSetup, type GhostsSnap } from '../ghosts/hall';
import { asMembers, inArena, rosterKeeper } from './common';

type View = BoutFrame<GhostsSnap, GhostsSetup>;

const KIND_NAMES = { small: 'Ghost', gold: 'Golden ghost', big: 'Big ghost' } as const;

/** Ghost Busting: hold a vacuum on the ghosts drifting round a haunted hall. */
const ghosts: BoutModule<GhostsSnap, GhostsSetup, GhostsEvent> = {
  create(host): BoutRenderer<GhostsSnap, GhostsSetup, GhostsEvent> {
    const hall = new GhostHall(host.scene);
    const roster = rosterKeeper(host);
    const inHall = (view: View) => view.snapshot?.players.find((p) => p.seat === view.mySeat) ?? null;
    let award: { id: number; points: number; caption: string } | null = null;
    let awardCount = 0;

    function hud(view: View): BoutHud {
      const snap = view.snapshot;
      const me = inHall(view);
      const mine = view.mySeat;
      const pulling = snap && mine !== null ? snap.ghosts.filter((g) => g.pullers.includes(mine)) : [];
      const lone = pulling.find((g) => g.kind === 'big' && g.pullers.length < 2);
      const drawing = pulling.filter((g) => g.drawn > 0).sort((a, b) => b.drawn - a.drawn)[0];
      return {
        objective: 'Suck up ghosts · gold is worth 3 · team up on the big one',
        hint: 'Click to take aim. WASD moves, hold click to run the vacuum.',
        alert: lone && !drawing ? 'The big ghost needs a second vacuum on it' : null,
        meter: drawing ? { label: 'DRAWING IN', progress: Math.min(1, drawing.drawn), text: KIND_NAMES[drawing.kind] } : null,
        combat: me
          ? {
              meter: 'score',
              hp: 0,
              hpMax: 0,
              hits: me.points,
              downs: 0,
              up: true,
              back: null,
              holding: me.power > 0,
              hitsLabel: 'points',
              outTitle: '',
              scoreHint: `${me.ghosts} ${me.ghosts === 1 ? 'ghost' : 'ghosts'} caught`,
            }
          : null,
        scores: (snap?.players ?? []).map((p) => ({ seat: p.seat, hits: p.points, up: true })),
        award,
        action: 'Suck',
      };
    }

    return {
      fog: [40, 150],
      frame(view) {
        hall.firstPerson = view.playing && !!inHall(view) && inArena(view.phase);
        hall.visible = true;
        const members = asMembers(view.seats);
        hall.sync(view.snapshot, view.setup, members, roster(view.seats), view.mySeat);
        hall.control({ x: view.stick.x, z: view.stick.z, ...view.look }, view.dt);
        hall.viewModel(host.camera, view.mySeat !== null ? hall.colorOf(view.mySeat, members) : '#9aa4b5');
        hall.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        hall.effect(event, asMembers(view.seats));
        if (event.kind === 'caught') {
          const mine = view.mySeat !== null && event.seats.includes(view.mySeat);
          if (mine) {
            awardCount += 1;
            award = { id: awardCount, points: event.points, caption: event.seats.length > 1 ? `${KIND_NAMES[event.ghost]}, shared` : KIND_NAMES[event.ghost] };
            host.sounds.win();
          } else {
            host.sounds.splat();
          }
        } else if (event.kind === 'phase' || event.kind === 'escape') {
          host.sounds.beep(0.2);
        } else if (event.kind === 'spawn' && event.ghost === 'gold') {
          host.sounds.count(false);
        }
      },
      shot(view) {
        const eye = hall.firstPerson ? hall.eye(view.look.yaw, view.look.pitch) : null;
        if (eye) return { kind: 'eye', ...eye };
        const grow = (view.setup?.half_x ?? 13) / 13;
        return { kind: 'frame', half: [20 * grow, 18 * grow], pitch: 55, portraitPitch: 66, lookHeight: 0 };
      },
      shake: () => hall.shake,
      facing(view) {
        return inHall(view)?.yaw ?? null;
      },
      hud,
      dispose() {
        hall.dispose();
      },
    };
  },

  blend(a, b, t) {
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        y: lerp(prev.y, next.y, t),
        z: lerp(prev.z, next.z, t),
        yaw: turn(prev.yaw, next.yaw, t),
        pitch: lerp(prev.pitch, next.pitch, t),
        power: lerp(prev.power, next.power, t),
      };
    });
    const ghosts = b.ghosts.map((next) => {
      const prev = a.ghosts.find((ghost) => ghost.id === next.id);
      // Wrapping through a wall is a jump across the hall, not a glide.
      if (!prev || Math.abs(prev.x - next.x) + Math.abs(prev.z - next.z) > 4) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        y: lerp(prev.y, next.y, t),
        z: lerp(prev.z, next.z, t),
        drawn: lerp(prev.drawn, next.drawn, t),
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), players, ghosts };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) => ({ ...player, x: player.x + player.vx * dt, z: player.z + player.vz * dt })),
      ghosts: snap.ghosts.map((ghost) => ({ ...ghost, x: ghost.x + ghost.vx * dt, z: ghost.z + ghost.vz * dt })),
    };
  },
};

export default ghosts;
