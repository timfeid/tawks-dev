import PETS from './catalog.json' with { type: 'json' };

export type Pet = { id: string; name: string; kind: string; body: string; accent: string; blurb: string };

export const PET_CATALOG = PETS as Pet[];

export function petById(id: string | null | undefined): Pet | null {
  return PET_CATALOG.find((pet) => pet.id === id) ?? null;
}

/** The pet an account hatched, worked out from its id exactly as the server does (libs/services/store/pets.rs). */
export function hatchedBy(user: string): Pet {
  let hash = 2166136261;
  for (const byte of new TextEncoder().encode('pet:' + user)) hash = Math.imul(hash ^ byte, 16777619) >>> 0;
  return PET_CATALOG[hash % PET_CATALOG.length];
}
