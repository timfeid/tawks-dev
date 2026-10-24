export type UnitType = 'none' | 'elf' | 'human' | 'beast' | 'elemental';
export type CardKind =
  | 'infrastructure'
  | 'unit'
  | 'relic'
  | 'artifact' // legacy alias
  | 'instant'
  | 'ritual'
  | 'sorcery' // legacy alias
  | 'aura'
  | 'enchantment' // legacy alias
  | 'agent';

export type TargetSpec = {
  scope: 'self_player' | 'opponent' | 'any' | 'any_player_or_unit';
  kind: 'player' | 'unit' | 'player_or_unit';
  max_targets: number;
};

export type ManaCost = {
  neon: number;
  glitch: number;
  chrome: number;
  slag: number;
  bio: number;
  colorless: number;
};

export type ManaPool = {
  neon: number;
  glitch: number;
  chrome: number;
  slag: number;
  bio: number;
  colorless: number;
};

export type CardAbilityView = {
  id: string;
  name: string;
  cost: any; // Simplified for now, backend uses AbilityCost
  effect_text: string;
  target_spec: TargetSpec | null;
  once_per_turn: boolean;
  tap_targets: boolean;
  can_activate: boolean;
  inactive_reason?: string | null;
};

export type StatModifierSourceView = {
  source_card_id?: string | null;
  source_card_name?: string | null;
  power_mod: number;
  toughness_mod: number;
  label: string;
  granted_keywords?: string[];
};

export type CardView = {
  id: string;
  name: string;
  description: string;
  rarity: 'public' | 'private' | 'restricted' | 'classified' | 'singularity';
  categories: string[];
  visual_spec?: {
    skin_id: string;
    seed: number;
    wear: number;
    holo?: boolean;
  };
  cost: ManaCost;
  kind_tag: string;
  kind_label: string;
  frame_style?: string;
  supertype?: string;
  unit_type?: UnitType;
  power?: number;
  toughness?: number;
  base_power?: number;
  base_toughness?: number;
  stat_modifiers?: StatModifierSourceView[];
  armor: number;
  shield: number;
  keywords: string[];
  effect_text?: string;
  target_spec?: TargetSpec;
  tapped: boolean;
  can_cast: boolean;
  can_play_land: boolean;
  playable: boolean;
  can_attack: boolean;
  can_tap: boolean;
  can_block: boolean;
  can_discard: boolean;
  summoning_sickness: boolean;
  haste: boolean;
  burnout: boolean;
  loyalty?: number;
  counters?: number;
  damage?: number;
  image_url?: string | null;
  colors?: string[];
  abilities: CardAbilityView[];
  attached_to?: string | null;
  attached_to_name?: string | null;
  creator_id?: string | null;
  remembered_player_id?: string | null;
  remembered_player_name?: string | null;
  fade_out?: boolean;
  zone?: 'hand' | 'battlefield' | 'graveyard' | 'exile' | 'operator_zone' | 'library' | 'stack' | 'lobby';
  owner_name?: string | null;
  owned?: boolean;
  value?: number | null;
};

export type GridNodeView = {
  mana_type: string;
  color: string;
};

export type PlayerState = {
  id: string;
  name: string;
  life: number;
  mana_pool: ManaPool;
  /** Syndicate only: police attention, raided at 10. */
  heat?: number;
  infrastructures: number;
  is_active: boolean;
  deck_colors?: string[];
  battlefield_units: CardView[];
  battlefield_infrastructure: CardView[];
  operator_zone: CardView[];
  grid: GridNodeView[];
  hand_size: number;
  library_size: number;
  graveyard_size: number;
  operator_damage: Record<string, number>;
};

export type CombatPairView = {
  attacker: CardView;
  blockers: CardView[];
  target_kind: string;
  target_label: string;
  target_player_id: string | null;
  target_card_id: string | null;
  is_blocked: boolean;
  attacker_dies: boolean;
  blocker_dies: boolean[];
};

export type DraftAwardsView = {
  winner_id?: string | null;
  winner_name?: string | null;
  role: 'winner' | 'loser' | 'spectator';
  selection_mode: 'keep' | 'claim' | 'none';
  cards: CardView[];
  required_count: number;
  submitted: boolean;
  waiting_on: string[];
};

export type CardGameStage =
  | 'lobby'
  | 'draft'
  | 'draft_building'
  | 'awards'
  | 'draw'
  | 'main'
  | 'combat'
  | 'post_combat_main'
  | 'end'
  | 'sideboard_swap'
  | 'finished';

export type MatchFormat = 'bo1' | 'bo3';

export type MatchStateView = {
  format: MatchFormat;
  current_game: number;
  wins_required: number;
  wins: Record<string, number>;
  series_winner?: string | null;
  pending_sideboard: string[];
  /** Winner id per completed game, in order. */
  game_winners: string[];
};

export type CombatPhase = 'idle' | 'declare_attackers' | 'declare_blockers';

export type PackSlot = {
  slot_id: string;
  pack_type_id: string;
  slot_index: number;
  weight_public: number;
  weight_private: number;
  weight_restricted: number;
  weight_classified: number;
  weight_singularity: number;
};

export type PackType = {
  pack_type_id: string;
  name: string;
  description: string;
  active: boolean;
  slots: PackSlot[];
};

export type PackInstance = {
  pack_instance_id: string;
  pack_type_id: string;
  owner_id: string;
  source: string;
  source_ref?: string | null;
  opened: boolean;
  opened_at?: string | null;
  created_at: string;
};

export type PackOpeningResult = {
  result_id: string;
  pack_instance_id: string;
  slot_index: number;
  instance_id: string;
  card_id: string;
  rarity: 'public' | 'private' | 'restricted' | 'classified' | 'singularity';
  card_name: string;
  card_image_url?: string | null;
  description?: string;
  cost?: ManaCost;
  kind_tag?: string;
  kind_label?: string;
  power?: number | null;
  toughness?: number | null;
  colors?: string[];
  keywords?: unknown[];
  frame_style?: string;
  categories?: string[];
  loyalty?: number | null;
};

export type PackOpenResult = {
  pack_instance_id: string;
  cards: PackOpeningResult[];
};

export type Ruleset = 'mana_clash' | 'syndicate';

export type CardGameView = {
  stage: CardGameStage;
  ruleset?: Ruleset;
  active_player_id?: string | null;
  active_player_name?: string | null;
  round: number;
  players: PlayerState[];
  cards: CardView[];
  winner_player_id?: string | null;
  winner_player_name?: string | null;
  match_state?: MatchStateView | null;
  viewer_sideboard?: CardView[];
  viewer_main_deck?: CardView[];
  draft_awards?: DraftAwardsView | null;
  combat_phase?: CombatPhase;
  combat_blocker_id?: string | null;
  pack_types?: PackType[];
  user_packs?: PackInstance[];
  pack_open_result?: PackOpenResult | null;
  pack_opening_results?: PackOpeningResult[];
};
