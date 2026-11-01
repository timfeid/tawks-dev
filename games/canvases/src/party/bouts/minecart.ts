import { lerp, turn, type BoutModule, type BoutRenderer } from '@tawks/bout-sdk';
import { MinecartField, type MinecartSetup, type MinecartSnap } from '../minecart/field';

type MinecartEvent =
  | { kind: 'release'; seat: number }
  | { kind: 'junction'; seat: number; fast: boolean }
  | { kind: 'crash'; a: number; b: number }
  | { kind: 'finish'; seat: number; ride: number };

/** The lobby has no server setup yet, so picking it shows a line laid the same way. */
function preview(): MinecartSetup {
  const straights: MinecartSetup['straights'] = [];
  const branches: MinecartSetup['branches'] = [];
  const fast: number[] = [];
  let z = 80;
  for (let junction = 0; junction <= 5; junction++) {
    straights.push([[0, z], [0, z - 14]]);
    z -= 14;
    if (junction === 5) break;
    const quick = junction % 2;
    const branch = (side: number) =>
      Array.from({ length: 25 }, (_, i): [number, number] => {
        const t = i / 24;
        const bulge = side === quick ? 3 : 9.5;
        return [(side === 0 ? -1 : 1) * bulge * (1 - Math.cos(t * Math.PI * 2)) * 0.5, z - t * 20];
      });
    branches.push([branch(0), branch(1)]);
    fast.push(quick);
    z -= 20;
  }
  return { game: 'minecart', straights, branches, fast, gauge: 0.9 };
}

const PREVIEW = preview();

const minecart: BoutModule<MinecartSnap, MinecartSetup, MinecartEvent> = {
  create(host): BoutRenderer<MinecartSnap, MinecartSetup, MinecartEvent> {
    const field = new MinecartField(host);
    return {
      fog: [80, 220],
      frame(view) {
        field.visible = true;
        field.sync(view.snapshot, view.setup ?? PREVIEW, view.seats);
        field.update(view.snapshot, view.time, view.dt);
      },
      effect(event, view) {
        if (event.kind === 'crash') {
          host.sounds.bump(1);
          if (event.a === view.mySeat || event.b === view.mySeat) field.shake = 0.6;
        } else if (event.kind === 'junction' && event.seat === view.mySeat) {
          if (event.fast) host.sounds.step(0.6);
          else host.sounds.beep(0.4);
        } else if (event.kind === 'release' && event.seat === view.mySeat) {
          host.sounds.count(true);
        } else if (event.kind === 'finish' && event.seat === view.mySeat) {
          host.sounds.win();
        }
      },
      shake: () => field.shake,
      shot(view) {
        const cart = field.focus(view.snapshot, view.mySeat);
        // Look a little down the line, since that is where the junctions come from.
        const target: [number, number] = cart ? [cart.x * 0.6, cart.z - 7] : [field.start.x, field.start.z - 4];
        return { kind: 'frame', half: [15, 20], pitch: 52, portraitPitch: 62, target, lookHeight: 1 };
      },
      hud(view) {
        const mine = view.snapshot?.players.find((player) => player.seat === view.mySeat);
        return {
          objective: 'Take the short branch at every junction',
          hint: 'Hold A or D before a junction to pick a branch. Green flags mark the short way. Hold S to brake.',
          scores: view.snapshot?.players.map((player) => ({
            seat: player.seat,
            hits: Math.round(player.progress * 100),
            up: !player.stunned,
          })),
          meter: mine
            ? {
                label: mine.waiting ? 'Waiting at the gate' : mine.finished ? 'Finished' : 'Ride',
                progress: mine.progress,
                text: `${mine.ride.toFixed(1)} s`,
              }
            : null,
        };
      },
      dispose() {
        field.dispose();
      },
    };
  },

  blend(a, b, t) {
    if (a.game !== b.game) return b;
    const players = b.players.map((next) => {
      const prev = a.players.find((player) => player.seat === next.seat);
      if (!prev || prev.waiting !== next.waiting || prev.finished !== next.finished) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        z: lerp(prev.z, next.z, t),
        heading: turn(prev.heading, next.heading, t),
        speed: lerp(prev.speed, next.speed, t),
        progress: lerp(prev.progress, next.progress, t),
        ride: lerp(prev.ride, next.ride, t),
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), players };
  },
};

export default minecart;
