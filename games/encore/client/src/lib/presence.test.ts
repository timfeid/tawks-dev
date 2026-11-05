import { describe, expect, it } from 'vitest';
import { presenceFor } from './presence';
import type { Player, RoomView } from './types';

const player = (conn: number, playing: boolean): Player => ({
  conn,
  user_id: `u${conn}`,
  name: `P${conn}`,
  avatar_url: '',
  part: { instrument: 'drums', difficulty: 'expert' },
  ready: true,
  loaded: true,
  playing,
  done: false,
  away: false,
  progress: { score: 0, streak: 0, best_streak: 0, multiplier: 1, hit: 0, missed: 0, total: 0, star_meter: 0, star_active: false, muted: false },
});

const room = (phase: RoomView['phase'], start_at: number | null): RoomView => ({
  phase,
  song: { id: 's', name: 'Bully', artist: 'Shinedown' } as RoomView['song'],
  picked_by: null,
  host: 1,
  start_at,
  ends_at: null,
  players: [player(1, true), player(2, false)],
});

describe('presenceFor', () => {
  it('shows the song, part and start time while playing', () => {
    expect(presenceFor(room('playing', 1793875345000), 1)).toEqual({
      details: 'Bully by Shinedown',
      state: 'Drums, Expert',
      started_at: 1793875345000,
    });
  });

  it('says picking a song in the list, in the results and while watching', () => {
    expect(presenceFor(room('lobby', null), 1)).toEqual({ details: 'Picking a song' });
    expect(presenceFor(room('results', null), 1)).toEqual({ details: 'Picking a song' });
    expect(presenceFor(room('playing', 5), 2)).toEqual({ details: 'Picking a song' });
    expect(presenceFor(null, 1)).toBeNull();
  });
});
