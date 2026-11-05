import type { Presence } from '@tawks/activity-sdk';
import { INSTRUMENT_LABELS, type RoomView } from './types';

/** What a player's status says while they are in Encore. */
export function presenceFor(room: RoomView | null, conn: number | null): Partial<Presence> | null {
  if (!room) return null;
  const me = room.players.find((p) => p.conn === conn);
  const song = room.song;
  const inSong = (room.phase === 'loading' || room.phase === 'playing') && !!song;
  if (!inSong || !me?.playing || !me.part) return { details: 'Picking a song' };
  const difficulty = me.part.difficulty[0].toUpperCase() + me.part.difficulty.slice(1);
  return {
    details: `${song.name} by ${song.artist}`,
    state: `${INSTRUMENT_LABELS[me.part.instrument]}, ${difficulty}`,
    started_at: room.start_at ?? null,
  };
}
