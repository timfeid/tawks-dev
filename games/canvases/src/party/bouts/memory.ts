import { lerp, type BoutFrame, type BoutHud, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import { MemoryField, type MemoryEvent, type MemorySetup, type MemorySnap } from '../memory/field';
import { asMembers, rosterKeeper } from './common';

/** The lobby has no server setup yet, so picking it shows the grid as it plays. */
const PREVIEW: MemorySetup = { game: 'memory', side: 3, spacing: 5, tile: 2, half: 9.5 };

function objective(snap: MemorySnap | null) {
  if (!snap) return 'Remember the tiles';
  if (snap.phase === 'show') return `Round ${snap.round} · watch ${snap.length} tiles`;
  if (snap.phase === 'recall') return 'Walk them back in order';
  return `Round ${snap.round} cleared`;
}

function hud(view: BoutFrame<MemorySnap, MemorySetup>): BoutHud {
  const snap = view.snapshot;
  const me = snap?.players.find((player) => player.seat === view.mySeat);
  const walking = snap?.phase === 'recall' && me && !me.out;
  return {
    objective: objective(snap),
    hint: 'Move with WASD. Click or press space to stomp the tile you are on.',
    scores: snap?.players.map((player) => ({ seat: player.seat, hits: player.cleared, up: !player.out })),
    meter:
      walking && snap
        ? {
            label: 'STEPS',
            progress: snap.length ? me.progress / snap.length : 0,
            text: me.done ? 'All done' : `${me.progress} of ${snap.length}`,
          }
        : null,
    alert: me?.out && snap?.phase !== 'break' ? 'You are out' : null,
  };
}

/** Memory Tiles: watch a grid light up, then walk the sequence back. */
const memory: BoutModule<MemorySnap, MemorySetup, MemoryEvent> = {
  create(host): BoutRenderer<MemorySnap, MemorySetup, MemoryEvent> {
    const field = new MemoryField(host.scene);
    const roster = rosterKeeper(host);
    return {
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup ?? PREVIEW, asMembers(view.seats), roster(view.seats));
        field.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        const mine = 'seat' in event && event.seat === view.mySeat;
        if (event.kind === 'light') host.sounds.beep(event.tile * 0.35);
        else if (event.kind === 'go') host.sounds.count(true);
        else if (event.kind === 'stomp') {
          field.stomp(event.tile, true);
          if (mine) host.sounds.step(0.8);
        } else if (event.kind === 'wrong') {
          field.stomp(event.tile, false);
          host.sounds.down(mine);
        } else if (event.kind === 'timeout') host.sounds.down(mine);
        else if (event.kind === 'done' && mine) host.sounds.beep(3);
      },
      shot(view) {
        // A bigger field plays on a bigger board, so the shot widens with it.
        const half = (view.setup ?? PREVIEW).half + 1.5;
        return { kind: 'frame', half: [half, half], pitch: 62, portraitPitch: 72 };
      },
      hud,
      dispose() {
        field.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game || a.phase !== b.phase) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev) return next;
      return { ...next, x: lerp(prev.x, next.x, t), z: lerp(prev.z, next.z, t) };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },
};

export default memory;
