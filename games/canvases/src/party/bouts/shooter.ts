import { lerp, turn, type BoutFrame, type BoutModule, type BoutRenderer, type CombatHud } from '@tawks/bout-sdk';
import type { GameEvent, ShooterSetup, ShooterSnap } from '../protocol';
import { ShooterArena } from '../shooter/arena';
import { asMembers, inArena, rosterKeeper } from './common';

type View = BoutFrame<ShooterSnap, ShooterSetup>;

const OBJECTIVES: Record<string, string> = {
  target: 'React first · shoot your lane light',
  popgun: 'One hit eliminates · be the last standing',
  ricochet: 'Use cover · score the most splats',
  shooter: 'Paint rivals · score the most splats',
  speedball: 'Hang the flag at their station or take everyone out',
};

const SIDES: [string, string] = ['RED', 'BLUE'];
const HOW: Record<string, string> = {
  flag: 'flag hung',
  elimination: 'everyone out',
  time: 'more left standing',
  trade: 'both sides out',
};

/** What the last speedball round came to, for the line under the clock. */
function roundLine(round: NonNullable<ShooterSnap['round']>) {
  const last = round.last;
  if (!last) return null;
  const how = HOW[last.how] ?? last.how;
  if (last.winner === null) return `Round ${round.number}: nobody takes it (${how})`;
  return `${SIDES[last.winner]} takes round ${round.number} · ${how}`;
}

const shooter: BoutModule<ShooterSnap, ShooterSetup, GameEvent> = {
  create(host): BoutRenderer<ShooterSnap, ShooterSetup, GameEvent> {
    const arena = new ShooterArena(host.scene);
    arena.onStep = (volume) => host.sounds.step(volume);
    const roster = rosterKeeper(host);
    let award: { id: number; points: number; caption: string } | null = null;
    let awardUntil = 0;
    let nextBeep = 0;
    const standing = (view: View) => view.snapshot?.players.find((p) => p.seat === view.mySeat) ?? null;

    return {
      fog: [100, 260],
      frame(view) {
        const snap = view.snapshot;
        const me = standing(view);
        const members = asMembers(view.seats);
        // Out of a speedball round, you watch the field from above until the next one.
        const watching = view.key === 'speedball' && !!me && !me.alive;
        arena.firstPerson = view.playing && !!me && inArena(view.phase) && !watching;
        arena.visible = true;
        arena.sync(snap, view.setup, members, roster(view.seats), view.mySeat);
        arena.control({ x: view.stick.x, z: view.stick.z, ...view.look }, view.dt);
        const mine = view.seats.find((seat) => seat.seat === view.mySeat);
        arena.viewModel(host.camera, mine ? roster(view.seats).get(mine.character) : undefined);
        arena.update(snap, view.time, view.dt);
        const bomb = snap?.game === 'retake' ? snap.objective : null;
        const now = performance.now();
        if (bomb && bomb.state === 'ticking' && view.phase === 'playing') {
          const left = Math.max(0, bomb.fuse / Math.max(1, bomb.fuse_max));
          if (now >= nextBeep) {
            nextBeep = now + 250 + left * 1150;
            host.sounds.beep(1 - left);
          }
        } else {
          nextBeep = 0;
        }
        if (award && now > awardUntil) award = null;
      },
      effect(event, view) {
        arena.effect(event, view.mySeat, asMembers(view.seats));
        const { sounds } = host;
        if (event.kind === 'target_hit') {
          sounds.splat();
          if (event.seat === view.mySeat) {
            award = { id: performance.now(), points: event.points, caption: `${(event.reaction * 1000).toFixed(0)} ms reaction` };
            awardUntil = performance.now() + 1250;
          }
        } else if (event.kind === 'exploded') sounds.blast();
        else if (event.kind === 'defused') sounds.win();
        else if (event.kind === 'defuse_start') sounds.count(false);
        else if (event.kind === 'flag_taken' || event.kind === 'flag_dropped') sounds.count(false);
        else if (event.kind === 'round_end') {
          const mine = view.snapshot?.players.find((p) => p.seat === view.mySeat)?.team;
          if (event.winner !== null && event.winner === mine) sounds.win();
          else sounds.down(false);
        } else if (event.kind === 'shot') sounds.shot();
        else if (event.kind === 'splat') sounds.splat();
        else if (event.kind === 'down') sounds.down(event.seat === view.mySeat);
      },
      shot(view) {
        const eye = arena.firstPerson ? arena.eye(view.look.yaw, view.look.pitch) : null;
        if (eye) return { kind: 'eye', ...eye };
        const halfX = view.setup?.half ?? 21;
        const halfZ = view.setup?.half_z ?? view.setup?.half ?? 18;
        if (view.key === 'target' && view.phase === 'lobby') {
          return { kind: 'frame', half: [Math.max(12, halfX + 4), 23], pitch: 50, portraitPitch: 62, target: [0, -9], lookHeight: 0 };
        }
        return { kind: 'frame', half: [halfX + 2, halfZ + 2], pitch: 52, portraitPitch: 64, lookHeight: 0 };
      },
      shake: () => arena.shake,
      facing(view) {
        const me = standing(view);
        return me?.alive ? me.yaw : null;
      },
      hud(view) {
        const snap = view.snapshot;
        const me = standing(view);
        const retake = view.key === 'retake';
        const speedball = view.key === 'speedball';
        const round = speedball ? snap?.round : undefined;
        const hits = (seat: number, kills: number) => snap?.scores?.[String(seat)] ?? kills;
        const combat: CombatHud | null = me
          ? {
              meter: view.key === 'target' ? 'score' : 'health',
              hp: me.hp,
              hpMax: view.setup?.hp ?? 100,
              hits: hits(me.seat, me.kills),
              downs: me.deaths,
              up: me.alive,
              back: retake || speedball || view.key === 'popgun' ? null : me.respawn,
              holding: false,
              hitsLabel: 'splats',
              outTitle: retake ? 'ELIMINATED' : speedball ? 'OUT' : 'Splatted',
              team: me.team,
            }
          : null;
        const objective = snap?.game === 'retake' ? snap.objective : null;
        const attacking = me?.team === 0;
        return {
          objective: retake
            ? attacking
              ? 'Attackers · defend the planted core'
              : 'Defenders · clear the site and defuse'
            : (OBJECTIVES[view.key] ?? OBJECTIVES.shooter),
          combat,
          scores: (snap?.players ?? []).map((p) => ({
            seat: p.seat,
            hits: hits(p.seat, p.kills),
            downs: view.key === 'target' ? undefined : p.deaths,
            up: p.alive,
            team: p.team,
          })),
          teams: retake ? ['ATTACK', 'RETAKE'] : speedball ? SIDES : undefined,
          alert: round && round.between > 0 ? roundLine(round) : null,
          meter: round
            ? {
                label: `ROUND ${round.number}`,
                progress: Math.min(1, (snap?.time ?? 0) / Math.max(1, snap?.limit ?? 1)),
                text: `RED ${round.wins[0]} - ${round.wins[1]} BLUE · first to ${round.to_win}`,
              }
            : retake && objective
              ? {
                  label: attacking ? 'CORE ARMED' : objective.progress > 0 ? 'DEFUSING' : 'RETAKE',
                  progress: Math.min(1, objective.progress / Math.max(0.01, objective.needed)),
                  text: attacking ? 'Defend the site' : `${Math.max(0, objective.needed - objective.progress).toFixed(1)}s to defuse`,
                }
              : null,
          award,
          hitMarker: arena.hitMarker,
          hint: 'Click to take aim. WASD moves, space jumps, C crouches, Escape lets go.',
          action: 'Fire',
          crouch: true,
        };
      },
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
      // A respawn teleports, so there is nothing sensible to blend through.
      if (!prev || prev.alive !== next.alive) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        y: lerp(prev.y, next.y, t),
        z: lerp(prev.z, next.z, t),
        yaw: turn(prev.yaw, next.yaw, t),
        pitch: lerp(prev.pitch, next.pitch, t),
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.alive ? { ...player, x: player.x + player.vx * dt, z: player.z + player.vz * dt } : player,
      ),
    };
  },
};

export default shooter;
