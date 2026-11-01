import type { CharacterDef, Costume } from './protocol';
import ROSTER from './roster.json';

/** The player colours, as the party server's roster has them, for drawing a character outside a room. */
export const ROSTER_COLORS = ROSTER as Omit<CharacterDef, 'costume'>[];

/** Someone's Tawkathon character from their saved outfit. */
export function characterFor(costume: Costume): CharacterDef {
  const color = ROSTER_COLORS.find((entry) => entry.id === costume.color) ?? ROSTER_COLORS[0];
  return { ...color, costume };
}
