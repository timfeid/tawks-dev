import { lerp, type BoutFrame, type BoutHud, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { GameEvent } from '../protocol';
import { RelayField, type RelaySetup, type RelaySnap } from '../relay/field';
import { asMembers, rosterKeeper } from './common';

/** The lobby has no server setup yet, so picking it shows the lane as it plays. */
const PREVIEW: RelaySetup = {
  game: 'relay',
  rule: '',
  half_x: 5,
  end_z: 19,
  pen_z: 17.4,
  pen_radius: 1,
  gate_gap: 3.4,
  gate_thick: 0.6,
  gates: [-8.5, 0, 8.5],
  bench_x: 7.5,
};

function relayHud(view: BoutFrame<RelaySnap, RelaySetup>): BoutHud {
  const snap = view.snapshot;
  const mine = view.mySeat;
  const carrying = snap !== null && mine !== null && snap.holder === mine;
  const called = snap !== null && mine !== null && snap.receiver === mine;
  const objective = carrying
    ? 'Get the bomb through the gates to the lit pen'
    : called
      ? 'Wait in the pen. The bomb is coming to you'
      : 'Outlast the field';
  const lit = snap?.lit ?? 0;
  const fuse = snap?.fuse ?? 0;
  return {
    objective,
    hint: 'Move with WASD. Line up with the gap in each gate, then touch the runner in the pen.',
    alert: carrying && fuse < 3 ? 'The fuse is almost out' : null,
    meter:
      snap && snap.holder !== null && lit > 0
        ? { label: 'FUSE', progress: Math.min(1, Math.max(0, fuse / lit)), text: `${fuse.toFixed(1)}s` }
        : null,
    scores: snap?.players.map((player) => ({ seat: player.seat, hits: player.passes, up: !player.out })),
  };
}

/** Bomb Relay, run down a lane of sliding gates. */
const relay: BoutModule<RelaySnap, RelaySetup, GameEvent> = {
  create(host): BoutRenderer<RelaySnap, RelaySetup, GameEvent> {
    const field = new RelayField(host.scene);
    const roster = rosterKeeper(host);
    return {
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup ?? PREVIEW, asMembers(view.seats), roster(view.seats));
        field.update(view.snapshot, view.time, view.dt);
      },
      effect(event) {
        field.effect(event);
        if (event.kind === 'boom') host.sounds.blast();
        else if (event.kind === 'pass') host.sounds.bump(0.5);
        else if (event.kind === 'light') host.sounds.count(false);
      },
      shot: (view) => {
        const setup = view.setup ?? PREVIEW;
        return { kind: 'frame', half: [setup.bench_x + 2, setup.end_z + 1.5], pitch: 52, portraitPitch: 62 };
      },
      shake: () => field.shake,
      hud: relayHud,
      dispose() {
        field.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      // Stepping up to a pen or off to the bench is a jump, not a run.
      if (!prev || prev.out !== next.out || prev.waiting !== next.waiting || Math.hypot(next.x - prev.x, next.z - prev.z) > 3) {
        return next;
      }
      return { ...next, x: lerp(prev.x, next.x, t), z: lerp(prev.z, next.z, t) };
    });
    const gates = b.gates.map((gap, index) => lerp(a.gates[index] ?? gap, gap, t));
    return { ...b, time: lerp(a.time, b.time, t), fuse: lerp(a.fuse, b.fuse, t), gates, players };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) =>
        player.out || player.waiting ? player : { ...player, x: player.x + player.vx * dt, z: player.z + player.vz * dt },
      ),
    };
  },
};

export default relay;
