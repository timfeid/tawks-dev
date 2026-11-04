/** The scene frame and relay messages, from the contract the made-protocol crate generates. */
export type * from '../../src/scene';
import type { GameMeta, RosterSeat } from '../../src/scene';

export type Seat = RosterSeat;
export type Meta = GameMeta;
/** Tied groups of seats, best first. */
export type Placements = number[][];
export type Button = 'action' | 'alt';
