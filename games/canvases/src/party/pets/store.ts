import ITEMS from './store.json' with { type: 'json' };
import type { PetLook } from '../protocol';

export type PetSlot = 'pet' | 'pet-coat' | 'pet-hat' | 'pet-neck' | 'pet-effect';
/** Sold for pets as Tawkathon cosmetics. A `pet` item adopts that pet; seasonal ones are sold and put on from `from` to `until` (MM-DD, US Eastern) each year, like the seasonal heads. */
export type PetItem = {
  id: string;
  slot: PetSlot;
  name: string;
  price: number;
  icon: string;
  description: string;
  style?: string;
  colors?: string[];
  from?: string;
  until?: string;
};

export const PET_ITEMS = ITEMS as PetItem[];

const FIELD = { pet: 'id', 'pet-coat': 'coat', 'pet-hat': 'hat', 'pet-neck': 'neck' } as const;

export function petItem(id: string | null | undefined): PetItem | null {
  return PET_ITEMS.find((item) => item.id === id) ?? null;
}

/** The pet drawn for someone: the one they adopted and put on, or the one they hatched, with what it wears. */
export type ShownPet = { id: string; coat: PetItem | null; hat: PetItem | null; neck: PetItem | null; stage: number; effect: string | null };

export function shownPet(hatched: string | null | undefined, look?: PetLook | null): ShownPet | null {
  const id = look?.id ?? hatched;
  if (!id) return null;
  return { id, coat: petItem(look?.coat), hat: petItem(look?.hat), neck: petItem(look?.neck), stage: look?.stage ?? 0, effect: petItem(look?.effects?.[id])?.style ?? null };
}

/** A stable key for a shown pet, so a preview rebuilds only when the look changes. */
export function shownPetKey(pet: ShownPet | null) {
  return pet ? [pet.id, pet.coat?.id, pet.hat?.id, pet.neck?.id, pet.stage, pet.effect].join('|') : '';
}

/** The look with `id` on. Choosing the hatched pet goes back to it. An effect goes on the pet that is out, which is `hatched` while no other is; each pet keeps its own. */
export function withPetItem(look: PetLook | null | undefined, id: string, hatched?: string | null): PetLook {
  const item = petItem(id);
  if (!item) return { ...look };
  if (item.slot === 'pet-effect') {
    const out = look?.id ?? hatched;
    return out ? { ...look, effects: { ...look?.effects, [out]: id } } : { ...look };
  }
  const field = FIELD[item.slot];
  return { ...look, [field]: field === 'id' && id === hatched ? null : id };
}

/** The look with the effect taken off `pet`. */
export function withoutPetEffect(look: PetLook | null | undefined, pet: string): PetLook {
  const effects = { ...look?.effects };
  delete effects[pet];
  return { ...look, effects };
}

/** Whether `id` is on, counting the hatched pet as on while no other is chosen. */
export function wearsPetItem(look: PetLook | null | undefined, id: string, hatched?: string | null) {
  const item = petItem(id);
  if (!item) return false;
  if (item.slot === 'pet') return (look?.id ?? hatched) === id;
  if (item.slot === 'pet-effect') return !!(look?.id ?? hatched) && look?.effects?.[(look?.id ?? hatched)!] === id;
  return look?.[FIELD[item.slot]] === id;
}

/** The same windows as the seasonal heads in costumes/selection.ts and the server's wardrobe.rs, on US Eastern days. */
export function petItemInSeason(item: PetItem, now = new Date()) {
  if (!item.from || !item.until) return true;
  const parts = new Intl.DateTimeFormat('en-US', { timeZone: 'America/New_York', month: '2-digit', day: '2-digit' }).formatToParts(now);
  const part = (type: string) => parts.find((each) => each.type === type)!.value;
  const day = `${part('month')}-${part('day')}`;
  return item.from <= item.until ? item.from <= day && day <= item.until : day >= item.from || day <= item.until;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const day = (monthDay: string) => {
  const [month, date] = monthDay.split('-').map(Number);
  return `${MONTHS[month - 1]} ${date}`;
};

/** "Until Nov 30" in season, "Back Nov 15" out of it, nothing for a year-round item. */
export function petSeasonNote(item: PetItem, now = new Date()) {
  if (!item.from || !item.until) return '';
  return petItemInSeason(item, now) ? `Until ${day(item.until)}` : `Back ${day(item.from)}`;
}
