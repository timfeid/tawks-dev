import type { BoutHost, Seat } from '@tawks/bout-sdk';
import type { CharacterDef, Member } from '../protocol';

/**
 * The first-party scenes predate the bout contract and still take the room's
 * members and a roster map; this keeps the map current as seats change.
 */
export function rosterKeeper(host: BoutHost) {
  let key = '';
  let roster = new Map<string, CharacterDef>();
  return (seats: Seat[]) => {
    const next = seats.map((seat) => seat.character).join();
    if (next !== key) {
      key = next;
      roster = new Map();
      for (const seat of seats) {
        const character = host.character(seat.character);
        if (character) roster.set(character.id, character as CharacterDef);
      }
    }
    return roster;
  };
}

export const asMembers = (seats: Seat[]) => seats as Member[];

export const inArena = (phase: string) => phase === 'playing' || phase === 'countdown';
