import type { CharacterDef } from './protocol';

/** Identity of the geometry/materials we build, independent of room packets,
 * display names, object identity, and costume property serialization order. */
export function characterAppearanceKey(def: CharacterDef): string {
  const costume = def.costume;
  return JSON.stringify([
    def.id, def.body, def.accent, def.belly, def.feature,
    costume?.color ?? def.id,
    costume?.hat ?? null, costume?.shirt ?? null, costume?.pants ?? null,
    costume?.shoes ?? null, costume?.gloves ?? null, [...(costume?.extras ?? [])].sort().join(','), costume?.bag ?? null,
    costume?.model_id ?? null, costume?.head ?? null,
    costume?.body_width ?? 1,
    costume?.shirt === 'shirt-pucklab' ? JSON.stringify(costume.jersey ?? null) : null,
    costume?.reaction ?? null, costume?.shuffle_reactions ? [...(costume.reaction_pool ?? [])].sort().join(',') : null,
  ]);
}
