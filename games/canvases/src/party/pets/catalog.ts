import PETS from './catalog.json';

export type Pet = { id: string; name: string; kind: string; body: string; accent: string; blurb: string };

export const PET_CATALOG = PETS as Pet[];

export function petById(id: string | null | undefined): Pet | null {
  return PET_CATALOG.find((pet) => pet.id === id) ?? null;
}
