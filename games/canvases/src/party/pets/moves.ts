import type { PetMove } from './bond';

/** How long each move plays, in seconds. */
export const MOVE_SECONDS: Record<PetMove, number> = { pet: 1.4, poke: 0.9, dance: 1.6, hop: 0.9, spin: 1.1, flip: 1.1, 'show-off': 2 };

/** Where a move puts the pet `t` seconds in: lifted by `lift` (in pet heights), turned by `spin` and tipped by `flip` (radians), squashed by `squash`. The native game plays pet and poke the same way. */
export function moveAt(move: PetMove, t: number) {
  const pose = { lift: 0, spin: 0, flip: 0, squash: 0 };
  const length = MOVE_SECONDS[move];
  if (t < 0 || t >= length) return pose;
  const ease = (x: number) => x * x * (3 - 2 * x);
  const hop = (from: number, to: number, height: number) => (t >= from && t < to ? Math.sin(((t - from) / (to - from)) * Math.PI) * height : 0);
  switch (move) {
    case 'pet':
      pose.lift = Math.abs(Math.sin(t * Math.PI * 2 * 1.6)) * 0.15 * (1 - t / length);
      break;
    case 'dance':
      pose.spin = Math.sin(t * Math.PI * 2 * 1.2) * 0.35 * (1 - t / length);
      pose.lift = Math.abs(Math.sin(t * Math.PI * 2.4)) * 0.08 * (1 - t / length);
      break;
    case 'poke':
    case 'hop':
      pose.lift = hop(0, 0.45, 0.45);
      pose.squash = Math.exp(-t * 7) * Math.cos(t * 22) * 0.22;
      break;
    case 'spin':
      pose.spin = ease(t / length) * Math.PI * 2;
      pose.lift = hop(0, length, 0.12);
      break;
    case 'flip':
      pose.lift = hop(0.1, 0.9, 0.6);
      pose.flip = t >= 0.1 && t < 0.9 ? -ease((t - 0.1) / 0.8) * Math.PI * 2 : 0;
      pose.squash = t >= 0.9 ? Math.exp(-(t - 0.9) * 9) * 0.2 : 0;
      break;
    case 'show-off':
      pose.spin = t < 1 ? ease(t) * Math.PI * 2 : 0;
      pose.lift = hop(0, 1, 0.3) + hop(1.1, 1.5, 0.2);
      break;
  }
  return pose;
}

/** The clip a move plays over its motion: a cheer, or a hop where the pet has one. */
export function moveClip(move: PetMove): 'Cheer' | 'Hop' | null {
  if (move === 'poke' || move === 'hop' || move === 'flip') return 'Hop';
  if (move === 'pet' || move === 'dance' || move === 'show-off') return 'Cheer';
  return null;
}
