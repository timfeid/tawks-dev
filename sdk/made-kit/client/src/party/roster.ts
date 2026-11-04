import type { CharacterDef, Costume } from './types';
import ROSTER from './roster.json';

export const ROSTER_COLORS = ROSTER as Omit<CharacterDef, 'costume'>[];

/** Someone's Tawkathon character from their saved outfit; with no outfit, a colour by seat. */
export function characterFor(costume: Costume | null | undefined, seat = 1): CharacterDef {
  const fallback = ROSTER_COLORS[(Math.max(1, seat) - 1) % ROSTER_COLORS.length];
  const color = (costume && ROSTER_COLORS.find((entry) => entry.id === costume.color)) ?? fallback;
  return { ...color, costume: costume ?? null };
}
