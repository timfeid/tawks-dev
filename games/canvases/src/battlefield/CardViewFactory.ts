import type { CardView, ManaCost } from './types';

export const EMPTY_MANA: ManaCost = {
  neon: 0,
  glitch: 0,
  chrome: 0,
  slag: 0,
  bio: 0,
  colorless: 0,
};

const CARD_DEFAULTS = {
  description: '',
  cost: EMPTY_MANA,
  kind_tag: 'card',
  kind_label: '',
  frame_style: 'card',
  armor: 0,
  shield: 0,
  keywords: [] as string[],
  tapped: false,
  can_cast: false,
  can_play_land: false,
  playable: false,
  can_attack: false,
  can_tap: false,
  can_block: false,
  can_discard: false,
  summoning_sickness: false,
  haste: false,
  burnout: false,
  abilities: [] as any[],
  colors: [] as string[],
};

export function createPackInventoryCard(
  typeId: string,
  name: string,
  description: string,
  pileCount: number,
): any {
  return {
    ...CARD_DEFAULTS,
    id: `pack-type-${typeId}`,
    name,
    description,
    kind_tag: 'pack',
    kind_label: 'Booster Pack',
    playable: true,
    colors: ['#f59e0b'],
    pile_count: pileCount,
  };
}

export function createOpeningPackCard(
  packInstanceId: string | null,
  typeId: string | null,
  packTypes: any[],
  userPacks: any[],
): any {
  if (!packInstanceId && !typeId) return null;
  const pack = userPacks.find((p) => p.pack_instance_id === packInstanceId);
  const packType = pack
    ? packTypes.find((t) => t.pack_type_id === pack.pack_type_id)
    : typeId
      ? packTypes.find((t) => t.pack_type_id === typeId)
      : null;
  return {
    ...CARD_DEFAULTS,
    id: `pack-type-${pack?.pack_type_id || typeId || 'active'}`,
    name: packType?.name || 'Pack',
    description: packType?.description || '',
    kind_tag: 'pack',
    kind_label: 'Booster Pack',
    colors: ['#f59e0b'],
  };
}

export function createPackResultCard(raw: any, index: number): any {
  return {
    ...CARD_DEFAULTS,
    ...raw,
    id: `pack-result-${index}`,
    name: raw.card_name || raw.name || raw.card_id || 'Unknown',
    description: raw.description || raw.effect_text || null,
    image_url: raw.card_image_url || raw.image_url || null,
    cost: raw.cost || EMPTY_MANA,
    kind_tag: raw.kind_tag && raw.kind_tag !== 'pack_pull' ? raw.kind_tag : 'card',
    kind_label:
      raw.kind_label && raw.kind_label !== 'Pack Pull' && raw.kind_label !== 'pack_pull'
        ? raw.kind_label
        : '',
    categories: (raw.categories || []).filter((cat: string) => cat !== 'pack_pull'),
    frame_style: raw.frame_style || 'card',
    power: raw.power ?? null,
    toughness: raw.toughness ?? null,
    armor: raw.armor ?? 0,
    shield: raw.shield ?? 0,
    keywords: raw.keywords || [],
    abilities: raw.abilities || [],
    colors: raw.colors?.length ? raw.colors : ['#38bdf8'],
    _zone: 'pack_opening',
  };
}

export function createDeckSelectCard(deck: {
  id: string;
  name: string;
  description: string;
  colors: string[];
  image_url?: string | null;
}): CardView {
  return {
    ...CARD_DEFAULTS,
    id: deck.id,
    name: deck.name,
    description: deck.description ?? '',
    kind_tag: 'deck',
    kind_label: 'Deck',
    colors: deck.colors.length > 0 ? deck.colors : ['#64748b'],
    unit_type: null,
    image_url: deck.image_url ?? null,
    effect_text: null,
    target_spec: null,
    attached_to: null,
    attached_to_name: null,
    loyalty: null,
  } as unknown as CardView;
}

export function createDeployCard(node: {
  value: string;
  label: string;
  color: string;
  image_url?: string | null;
}): CardView {
  return {
    ...CARD_DEFAULTS,
    id: `deploy-${node.value}`,
    name: node.label,
    description: `Add ${node.label} mana`,
    kind_tag: 'infrastructure',
    kind_label: 'Mana',
    frame_style: 'infra_hex',
    can_play_land: true,
    playable: true,
    colors: [node.color],
    image_url: node.image_url ?? null,
  } as unknown as CardView;
}

export function createFallbackOperator(): CardView {
  return {
    ...CARD_DEFAULTS,
    id: 'operator-placeholder',
    name: 'Select Operator',
    kind_tag: 'unit',
    kind_label: 'Apex',
    supertype: 'apex',
    colors: ['#22d3ee'],
    unit_type: null,
    image_url: null,
    effect_text: null,
    target_spec: null,
    attached_to: null,
    attached_to_name: null,
    loyalty: null,
  } as unknown as CardView;
}
