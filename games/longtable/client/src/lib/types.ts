export type Pass = { ws_url: string; token: string; lobby_id: string };

export type Visibility = 'public' | 'gm' | 'me';
export type Edge = 'advantage' | 'disadvantage';

export type Die = { value: number; kept: boolean };
export type Part =
  | { kind: 'dice'; negative: boolean; count: number; sides: number; dice: Die[]; subtotal: number }
  | { kind: 'flat'; negative: boolean; value: number };

export type Roll = {
  formula: string;
  label: string | null;
  edge: Edge | null;
  parts: Part[];
  total: number;
  natural: 'twenty' | 'one' | null;
};

export type RollView =
  | { shown: 'open'; id: number; at_ms: number; by: string; by_name: string; visibility: Visibility; roll: Roll }
  | { shown: 'secret'; id: number; at_ms: number; by: string; by_name: string; by_gm: boolean };

export type Slots = { max: number; used: number };

export type FullSheet = {
  access: 'full';
  id: string;
  owner: string;
  name: string;
  class_level: string;
  hp: number;
  hp_max: number;
  ac: number;
  abilities: number[];
  conditions: string[];
  exhaustion: number;
  slots: Slots[];
  notes: string;
};

export type StripSheet = {
  access: 'strip';
  id: string;
  owner: string;
  name: string;
  class_level: string;
  hp: number;
  hp_max: number;
  ac: number;
  conditions: string[];
  exhaustion: number;
};

export type SheetView = FullSheet | StripSheet;

export type SheetPatch = Partial<Omit<FullSheet, 'access' | 'id' | 'owner'>>;

export type Combatant = {
  id: string;
  name: string;
  init: number;
  bonus: number;
  sheet_id: string | null;
  owner: string | null;
  hidden: boolean;
};

export type Person = { user_id: string; name: string; avatar_url: string; online: boolean };

export type TableView = {
  me: string;
  gm: string | null;
  people: Person[];
  sheets: SheetView[];
  initiative: { entries: Combatant[]; current: string | null; round: number; hidden_turn: boolean };
  rolls: RollView[];
};

export type Condition = { name: string; summary: string };

export type Action =
  | { type: 'roll'; formula: string; visibility: Visibility }
  | { type: 'sheet_create'; name: string }
  | { type: 'sheet_update'; id: string; patch: SheetPatch }
  | { type: 'sheet_delete'; id: string }
  | { type: 'init_add_sheet'; sheet_id: string; init: number | null }
  | { type: 'init_add_npc'; name: string; init: number | null; bonus: number; hidden: boolean }
  | { type: 'init_remove'; id: string }
  | { type: 'init_set'; id: string; init: number }
  | { type: 'init_hide'; id: string; hidden: boolean }
  | { type: 'init_sort' }
  | { type: 'init_next' }
  | { type: 'init_prev' }
  | { type: 'init_clear' }
  | { type: 'set_gm'; user_id: string }
  | { type: 'take_gm' };

export type ServerMsg =
  | { type: 'welcome'; user_id: string; conditions: Condition[]; attribution: string; compatibility: string }
  | { type: 'table'; table: TableView; rolled: number[] }
  | { type: 'pong'; t: number }
  | { type: 'error'; message: string };

export type ClientMsg = { type: 'hello'; token: string } | { type: 'ping'; t: number } | { type: 'act'; action: Action };
