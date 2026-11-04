export type Feature = 'none' | 'cap' | 'leaf' | 'antenna' | 'long_ears' | 'round_ears' | 'horns' | 'beanie' | 'visor' | 'top_hat' | 'crown' | 'wizard';

export type TintSlot = 'hat' | 'shirt' | 'pants' | 'shoes' | 'gloves' | 'bag' | 'extra';

/** A Tawkathon look, as the party server's wardrobe sends it. Fields the kit does not draw are ignored. */
export type Costume = {
  color: string;
  body_width?: number | null;
  hat?: string | null;
  shirt?: string | null;
  pants?: string | null;
  shoes?: string | null;
  gloves?: string | null;
  bag?: string | null;
  extras?: string[] | null;
  model_id?: string | null;
  head?: string | null;
  reaction?: string | null;
  shuffle_reactions?: boolean;
  reaction_pool?: string[];
  tints?: Partial<Record<TintSlot, string>>;
};

export type CharacterDef = {
  id: string;
  name: string;
  body: string;
  accent: string;
  belly: string;
  feature: Feature;
  costume?: Costume | null;
};
