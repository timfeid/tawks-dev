import { lerp, type BoutModule } from '@tawks/bout-sdk';
import { BumperArena } from './arena';
import type { BallSnap, BumperEvent, BumperSnap } from './snapshot';

export type { BumperEvent, BumperSnap } from './snapshot';

const bumperBalls: BoutModule<BumperSnap, null, BumperEvent> = {
  create: (host) => new BumperArena(host),

  blend(a, b, t) {
    const balls = b.balls.map((next): BallSnap => {
      const prev = a.balls.find((ball) => ball.seat === next.seat);
      if (!prev || (prev.state === 'out') !== (next.state === 'out')) return next;
      return {
        ...next,
        x: lerp(prev.x, next.x, t),
        z: lerp(prev.z, next.z, t),
        y: lerp(prev.y, next.y, t),
        vx: lerp(prev.vx, next.vx, t),
        vz: lerp(prev.vz, next.vz, t),
        state: t < 0.5 ? prev.state : next.state,
      };
    });
    return { ...b, time: lerp(a.time, b.time, t), radius: lerp(a.radius, b.radius, t), balls };
  },

  extrapolate(snapshot, dt) {
    return {
      ...snapshot,
      balls: snapshot.balls.map((ball) =>
        ball.state === 'rolling' ? { ...ball, x: ball.x + ball.vx * dt, z: ball.z + ball.vz * dt } : ball,
      ),
    };
  },
};

export default bumperBalls;
