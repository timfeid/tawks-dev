import catalog from './catalog.json';
import { isFaceReaction, type FaceReaction } from './face';
import type { Costume } from '../types';

/** Sold like other Tawkathon cosmetics, as `tawkathon-<id>`. Shaped like the native game's emote catalog. */
export type ReactionItem = { id: string; name: string; price: number; icon: string; animation: FaceReaction; description: string };
export const REACTIONS = catalog.filter((item): item is ReactionItem => isFaceReaction(item.animation));

export const isReaction = (id: string | null | undefined) => !!id && REACTIONS.some((item) => item.id === id);
export const faceOf = (id: string | null | undefined): FaceReaction | null => REACTIONS.find((item) => item.id === id)?.animation ?? null;

/** The face to make for this look: its chosen reaction, or with shuffle on, any it owns. */
export function chooseReaction(costume: Costume | null | undefined, random = Math.random): FaceReaction | null {
  const pool = costume?.shuffle_reactions ? (costume.reaction_pool ?? []).filter(isReaction) : [];
  const id = pool.length ? pool[Math.floor(random() * pool.length)] : costume?.reaction;
  return faceOf(id);
}
