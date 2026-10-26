import { lerp, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import type { GameEvent, RaceSetup, RaceSnap } from '../protocol';
import { RockerTrack } from '../race/track';
import { asMembers, rosterKeeper } from './common';

/** The lobby has no server setup yet, so selecting Rocker Rally shows this course. */
const PREVIEW: RaceSetup = {
  game: 'race',
  half: 11,
  start: -38,
  finish: 38,
  boosts: [[-5.5, -17], [5.5, -2], [-4, 15], [4.5, 29]],
  mud: [[0, -9, 3], [-5, 7, 2.6], [5, 23, 2.4]],
};

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

/** Rocker Rally and Red Light Rush share one course. */
const race: BoutModule<RaceSnap, RaceSetup, GameEvent> = {
  create(host): BoutRenderer<RaceSnap, RaceSetup, GameEvent> {
    const track = new RockerTrack(host.scene);
    const roster = rosterKeeper(host);
    let pumping = false;
    // Rocker Rally is a rhythm game, not a direction hold. Send the edge of
    // each A/D press immediately so a quick alternating tap is never lost
    // between the regular movement-input samples.
    const onPump = (event: KeyboardEvent) => {
      if (!pumping || event.repeat || isTyping(event.target)) return;
      if (event.code !== 'KeyA' && event.code !== 'KeyD') return;
      event.preventDefault();
      host.send({ type: 'input', x: event.code === 'KeyA' ? -1 : 1, z: 0 });
    };
    window.addEventListener('keydown', onPump);

    return {
      fog: [120, 340],
      frame(view) {
        pumping = view.key === 'race' && view.phase === 'playing' && view.playing;
        track.visible = true;
        track.sync(view.snapshot, view.setup ?? PREVIEW, asMembers(view.seats), roster(view.seats));
        track.update(view.dt);
      },
      shot: () => ({ kind: 'frame', half: [13, 43], pitch: 58, portraitPitch: 65 }),
      hud: (view) =>
        view.key === 'race'
          ? {
              objective: 'Pump to build speed · steer onto boosts',
              hint: 'Alternate A and D to pump. W and S steer for boost pads.',
              touch: 'pump',
            }
          : {
              objective: 'Reach the finish · freeze on red',
              hint: 'Run with WASD. Stop moving the moment the light turns red.',
            },
      dispose() {
        window.removeEventListener('keydown', onPump);
        track.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev || prev.finished !== next.finished) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        z: lerp(prev.z, next.z, t),
        speed: lerp(prev.speed, next.speed, t),
        bounce: lerp(prev.bounce, next.bounce, t),
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },

  extrapolate(snap, dt) {
    return {
      ...snap,
      players: snap.players.map((player) => (player.finished ? player : { ...player, z: player.z + player.speed * dt })),
    };
  },
};

export default race;
