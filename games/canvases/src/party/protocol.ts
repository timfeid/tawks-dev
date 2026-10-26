export type Feature = 'cap' | 'leaf' | 'antenna' | 'long_ears' | 'round_ears' | 'horns' | 'beanie' | 'visor';

export type CharacterDef = {
  id: string;
  name: string;
  body: string;
  accent: string;
  belly: string;
  feature: Feature;
};

export type Phase =
  | { name: 'lobby' }
  | { name: 'board' }
  | { name: 'vote'; seconds: number; options: string[]; votes: BoutVote[] }
  | { name: 'table' }
  | { name: 'countdown'; seconds: number }
  | { name: 'playing' }
  | { name: 'results'; seconds: number; placements: number[][] }
  | { name: 'game_over'; seconds: number; standings: number[][] };

export type BoutVote = { seat: number; key: string };

export type BoutControls = 'move' | 'move_action' | 'move_aim' | 'first_person';

export type BoutSpec = {
  key: string;
  name: string;
  rules: string;
  controls: BoutControls;
  format: 'free_for_all' | 'teams';
  min_players: number;
  max_players: number;
};

export type SpaceKind = 'start' | 'coin' | 'loss' | 'item' | 'event' | 'shop' | 'hazard' | 'teleport' | 'prize_spot';
export type Item = 'bat' | 'bomb' | 'grapple' | 'trap';

export type BoardSpace = {
  id: number;
  x: number;
  z: number;
  kind: SpaceKind;
  next: number[];
  link: number | null;
};

export type BoardToken = {
  seat: number;
  space: number;
  coins: number;
  health: number;
  items: Item[];
  prizes: number;
  resting: number;
};

export type BranchOption = {
  node: number;
  kind: SpaceKind;
  shortcut: boolean;
  saves: number;
};

export type TurnStep =
  | { step: 'items' }
  | { step: 'roll' }
  | { step: 'branch'; options: BranchOption[]; left: number }
  | { step: 'done' };

export type BoardLogEntry = Record<string, any> & { kind: string };

export type AimView = {
  seat: number;
  spaces: number;
};

export type ReachView = {
  item: Item;
  spaces: number;
};

export type BoardView = {
  spaces: BoardSpace[];
  tokens: BoardToken[];
  traps: number[];
  prize_at: number;
  prize_cost: number;
  order: number[];
  turn_seat: number;
  turn: TurnStep;
  round: number;
  rounds: number;
  last_roll: number | null;
  log: BoardLogEntry[];
  aim: AimView[];
  reach: ReachView[];
};

export type Member = {
  user_id: string;
  name: string;
  avatar_url: string;
  character: string;
  seat: number;
  is_bot: boolean;
  connected: boolean;
  wins: number;
  in_round: boolean;
};

export type RoomView = {
  match_id: string;
  minigame: MinigameKey | string;
  minigame_name: string;
  bouts: BoutSpec[];
  table_games: [string, string][];
  bot_skill: number;
  bot_skill_name: string;
  skills: [string, number][];
  host_id: string | null;
  phase: Phase;
  members: Member[];
  max_seats: number;
  board: BoardView | null;
  setup: GameSetup;
  recording?: RoomRecording | null;
};

export type RoomRecording = { by: string; by_user_id: string; since: number; live: boolean };

export type MinigameKey = 'bumper' | 'shooter' | 'dodgeball' | 'race' | 'redlight' | 'crumble' | 'hotpotato' | 'balloon' | 'ledge' | 'target' | 'popgun' | 'ricochet' | 'retake';

export type ShooterBlock = { x: number; z: number; hx: number; hz: number; top: number; bottom?: number; kind?: string };

export type ShooterSetup = {
  game: 'shooter' | 'target' | 'popgun' | 'ricochet' | 'retake';
  half: number;
  half_z?: number;
  hp: number;
  eye: number;
  height: number;
  radius: number;
  blocks: ShooterBlock[];
};

export type DodgeballSetup = {
  game: 'dodgeball';
  half_x: number;
  half_z: number;
  eye: number;
  height: number;
  radius: number;
  ball_radius: number;
  blocks: ShooterBlock[];
};

export type RaceSetup = {
  game: 'race' | 'redlight';
  half: number;
  start: number;
  finish: number;
  boosts: [number, number][];
  mud: [number, number, number][];
};

export type ArenaGame = 'crumble' | 'hotpotato' | 'balloon' | 'ledge';

export type ArenaSetup =
  | { game: 'crumble'; rule: string; tile: number; radius: number; tiles: { x: number; z: number }[] }
  | { game: 'hotpotato'; rule: string; radius: number }
  | { game: 'balloon'; rule: string; half: number; balloons: number }
  | { game: 'ledge'; rule: string; radius: number; ledge: number };

export type ArenaPlayer = {
  seat: number;
  x: number;
  z: number;
  y?: number;
  vx?: number;
  vz?: number;
  out: boolean;
  holding?: boolean;
  balloons?: number;
  dashing?: boolean;
  safe?: boolean;
  ready?: boolean;
};

export type ArenaSnap = {
  game: ArenaGame;
  time: number;
  limit: number;
  players: ArenaPlayer[];
  tiles?: { x: number; z: number; state: 'solid' | 'cracking' | 'falling' | 'gone' }[];
  ledges?: { x: number; z: number; taken: number | null }[];
  holder?: number | null;
  fuse?: number;
  phase?: 'roam' | 'scramble' | 'settle';
  timer?: number;
  round?: number;
};

export const ARENA_GAMES: ArenaGame[] = ['crumble', 'hotpotato', 'balloon', 'ledge'];

export type BattleGame = 'tank' | 'titan';

export type BattleSetup =
  | {
      game: 'tank';
      rule: string;
      half: number;
      radius: number;
      crates: { x: number; z: number; half: number; breakable: boolean }[];
    }
  | { game: 'titan'; rule: string; radius: number; titan: number; health: number; core: number };

export type BattlePlayer = {
  seat: number;
  x: number;
  z: number;
  vx?: number;
  vz?: number;
  hull?: number;
  turret?: number;
  kills?: number;
  deaths?: number;
  hits?: number;
  downs?: number;
  out: boolean;
  back_in?: number;
  reload?: number;
  ready?: boolean;
};

export type BattleSnap = {
  game: BattleGame;
  time: number;
  limit: number;
  players: BattlePlayer[];
  shells?: { id: number; x: number; z: number; vx: number; vz: number; by: number }[];
  crates?: { x: number; z: number; half: number; solid: boolean }[];
  titan?: {
    x: number;
    z: number;
    facing: number;
    health: number;
    max_health: number;
    state: 'stalk' | 'winding' | 'striking' | 'winded';
    attack: 'slam' | 'sweep' | 'charge';
    timer: number;
    radius: number;
  };
};

export const BATTLE_GAMES: BattleGame[] = ['tank', 'titan'];

export const isBattleSnap = (snap: GameSnap | null): snap is BattleSnap =>
  !!snap && BATTLE_GAMES.includes((snap.game ?? '') as BattleGame);

export const isBattleSetup = (setup: GameSetup): setup is BattleSetup =>
  !!setup && BATTLE_GAMES.includes(setup.game as BattleGame);

export const isArenaSnap = (snap: GameSnap | null): snap is ArenaSnap =>
  !!snap && ARENA_GAMES.includes((snap.game ?? '') as ArenaGame);

export const isArenaSetup = (setup: GameSetup): setup is ArenaSetup =>
  !!setup && ARENA_GAMES.includes(setup.game as ArenaGame);

export type GameSetup = ShooterSetup | DodgeballSetup | RaceSetup | ArenaSetup | BattleSetup | null;

export type BallState = 'rolling' | 'falling' | 'out';

export type BallSnap = {
  seat: number;
  x: number;
  z: number;
  y: number;
  vx: number;
  vz: number;
  state: BallState;
};

export type BumperSnap = {
  game?: 'bumper' | 'crumble' | 'hotpotato' | 'balloon' | 'ledge';
  time: number;
  limit: number;
  radius: number;
  ball_radius: number;
  balls: BallSnap[];
  beam?: number;
  hazard?: number;
};

export type ShooterPlayer = {
  seat: number;
  x: number;
  y: number;
  z: number;
  vx: number;
  vz: number;
  yaw: number;
  pitch: number;
  hp: number;
  kills: number;
  deaths: number;
  alive: boolean;
  respawn: number;
  ground: boolean;
  crouch?: boolean;
  team?: number;
};

export type ShooterSnap = {
  game: 'shooter' | 'target' | 'popgun' | 'ricochet' | 'retake';
  time: number;
  limit: number;
  players: ShooterPlayer[];
  targets?: { id: number; owner: number; x: number; y: number; z: number; up: boolean }[];
  scores?: Record<string, number>;
  objective?: {
    x: number;
    z: number;
    progress: number;
    needed: number;
    fuse: number;
    fuse_max: number;
    zone: number;
    defusing: boolean;
    state: 'ticking' | 'defused' | 'exploded';
    active: boolean;
  };
};

export type DodgeballPlayer = {
  seat: number;
  x: number;
  y: number;
  z: number;
  vx: number;
  vz: number;
  yaw: number;
  pitch: number;
  out: boolean;
  back_in: number;
  holding: boolean;
  hits: number;
  outs: number;
  ground: boolean;
};

export type DodgeballBall = {
  id: number;
  x: number;
  y: number;
  z: number;
  state: 'idle' | 'held' | 'live';
  by: number | null;
};

export type DodgeballSnap = {
  game: 'dodgeball';
  time: number;
  limit: number;
  players: DodgeballPlayer[];
  balls: DodgeballBall[];
};

export type RacePlayer = { seat: number; x: number; z: number; speed: number; bounce: number; finished: boolean };
export type RaceSnap = { game: 'race' | 'redlight'; time: number; limit: number; players: RacePlayer[] };

export type GameSnap = BumperSnap | ShooterSnap | DodgeballSnap | RaceSnap | ArenaSnap | BattleSnap;

/** Narrows the room's setup to the one the first person arenas use. */
export const isShooterSetup = (setup: GameSetup): setup is ShooterSetup =>
  !!setup && ['shooter', 'target', 'popgun', 'ricochet', 'retake'].includes(setup.game);

export const isShooterSnap = (snap: GameSnap | null): snap is ShooterSnap =>
  !!snap && ['shooter', 'target', 'popgun', 'ricochet', 'retake'].includes(snap.game ?? 'bumper');
export const isDodgeballSnap = (snap: GameSnap | null): snap is DodgeballSnap => snap?.game === 'dodgeball';
export const isRaceSnap = (snap: GameSnap | null): snap is RaceSnap => snap?.game === 'race' || snap?.game === 'redlight';
export const isBumperSnap = (snap: GameSnap | null): snap is BumperSnap =>
  !!snap &&
  !['shooter', 'target', 'popgun', 'ricochet', 'retake', 'dodgeball', 'race', 'redlight', ...ARENA_GAMES].includes(
    snap.game ?? 'bumper',
  );

export type GameEvent =
  | { kind: 'bump'; a: number; b: number; strength: number; x: number; z: number }
  | { kind: 'fall'; seat: number }
  | { kind: 'splash'; seat: number; x: number; z: number }
  | { kind: 'shot'; seat: number; from: [number, number, number]; to: [number, number, number]; hit: boolean }
  | { kind: 'splat'; seat: number; by: number; x: number; y: number; z: number; down: boolean }
  | { kind: 'down'; seat: number; by: number }
  | { kind: 'respawn'; seat: number }
  | { kind: 'throw'; seat: number; id: number }
  | { kind: 'catch'; seat: number; off: number; id: number }
  | { kind: 'pickup'; seat: number; id: number }
  | { kind: 'bounce'; id: number; x: number; z: number }
  | { kind: 'out'; seat: number; by: number; ball: number }
  | { kind: 'back'; seat: number }
  | { kind: 'pump'; seat: number; side: number }
  | { kind: 'boost'; seat: number; x: number; z: number }
  | { kind: 'finish'; seat: number; place: number }
  | { kind: 'target_hit'; seat: number; target: number; x: number; y: number; z: number; points: number; reaction: number }
  | { kind: 'defuse'; progress: number }
  | { kind: 'defuse_start'; seat: number | null }
  | { kind: 'defuse_stop' }
  | { kind: 'defused'; seat: number | null; fuse: number }
  | { kind: 'exploded'; x: number; z: number; caught: number[] }
  | { kind: 'retake_failed' }
  | { kind: 'crack'; seat: number; x: number; z: number }
  | { kind: 'tile_fall'; x: number; z: number }
  | { kind: 'drop'; seat: number }
  | { kind: 'pass'; from: number; to: number; fuse: number }
  | { kind: 'boom'; seat: number; x: number; z: number }
  | { kind: 'light'; seat: number }
  | { kind: 'pop'; seat: number; by: number; left: number }
  | { kind: 'burst'; seat: number; by: number }
  | { kind: 'horn'; ledges: number }
  | { kind: 'claim'; seat: number; x: number; z: number }
  | { kind: 'over'; seat: number }
  | { kind: 'music'; round: number; ledges: number }
  | { kind: 'shell'; seat: number; id: number; x: number; z: number }
  | { kind: 'ricochet'; id: number; x: number; z: number }
  | { kind: 'splinter'; x: number; z: number }
  | { kind: 'wreck'; seat: number; by: number; x: number; z: number }
  | { kind: 'rollout'; seat: number }
  | { kind: 'wind_up'; attack: string }
  | { kind: 'strike'; attack: string; x: number; z: number }
  | { kind: 'winded' }
  | { kind: 'recovered' }
  | { kind: 'floored'; seat: number; by: string }
  | { kind: 'swing'; seat: number }
  | { kind: 'strike_core'; seat: number; x: number; z: number; left: number }
  | { kind: 'titan_down' };

export type ServerMsg =
  | { type: 'welcome'; user_id: string; match_id: string; roster: CharacterDef[]; tick_hz: number; snapshot_hz: number; guest?: boolean }
  | { type: 'room'; room: RoomView; spectating: boolean }
  | { type: 'snap'; tick: number; game: GameSnap; events: GameEvent[] }
  | { type: 'pong'; t: number }
  | { type: 'table'; game: string; view: unknown }
  | { type: 'error'; message: string; code?: string };

export type ClientMsg =
  | { type: 'hello'; token: string }
  | { type: 'input'; x: number; z: number; yaw?: number; pitch?: number; jump?: boolean; crouch?: boolean }
  | { type: 'fire'; yaw: number; pitch: number }
  | { type: 'choose_game'; key: string }
  | { type: 'vote'; key: string }
  | { type: 'choose_skill'; level: string }
  | { type: 'pick'; character: string }
  | { type: 'start' }
  | { type: 'start_party' }
  | { type: 'use_item'; item: Item; target: number | null }
  | { type: 'pass_items' }
  | { type: 'roll' }
  | { type: 'choose_branch'; node: number }
  | { type: 'end_turn' }
  | { type: 'add_bot' }
  | { type: 'remove_bot'; seat: number }
  | { type: 'table'; action: unknown }
  | { type: 'record'; on: boolean }
  | { type: 'ping'; t: number };

export type PartyPass = { ws_url: string; token: string; match_id: string };

export type CovenCard =
  | 'rumor'
  | 'testimony'
  | 'spectral_evidence'
  | 'vouching'
  | 'barn_fire'
  | 'hex'
  | 'pickpocket'
  | 'deflection'
  | 'sanctuary'
  | 'good_standing'
  | 'pillory'
  | 'betrothal'
  | 'familiar'
  | 'nightfall'
  | 'conspiracy';
export type CovenVerdict = 'innocent' | 'witch' | 'constable';
export type CovenCharacter =
  | 'minister'
  | 'magistrate'
  | 'farmer'
  | 'midwife'
  | 'innkeeper'
  | 'blacksmith'
  | 'widow'
  | 'servant'
  | 'trapper'
  | 'schoolmaster'
  | 'sea_captain'
  | 'physician';
export type CovenPlayer = {
  seat: number;
  character: CovenCharacter;
  character_title: string;
  alive: boolean;
  witch: boolean;
  verdicts: { revealed: boolean; card: CovenVerdict | null }[];
  table: CovenCard[];
  accusations: number;
  hand_size: number;
  ability_used: boolean;
};
export type CovenPhase =
  | { kind: 'dawn'; voted: Record<string, number> | null }
  | { kind: 'turn'; seat: number; played: boolean; drawing: boolean }
  | { kind: 'reveal'; chooser: number; accused: number }
  | { kind: 'conspiracy'; from: number | null; picked: boolean; waiting: number }
  | { kind: 'night'; votes: Record<string, number> | null; constable: boolean }
  | { kind: 'confess'; confessed: number[] }
  | { kind: 'over'; winner: 'town' | 'witches' };
export type CovenSeat = { user_id: string | null; name: string; avatar_url: string | null; character: string | null; connected: boolean };
export type CovenLog = { type: string; seat?: number; target?: number; second?: number | null; card?: string; index?: number; how?: string; witch?: boolean; winner?: string };
export type CovenView = {
  players: CovenPlayer[];
  hand: CovenCard[] | null;
  phase: CovenPhase;
  seconds_left: number;
  deck: number;
  discard_top: CovenCard | null;
  round: number;
  witches: number[];
  constable: boolean;
  familiar: number | null;
  log: CovenLog[];
  seats: CovenSeat[];
  me: number | null;
};
export type CovenAction =
  | { type: 'place_familiar'; target: number }
  | { type: 'draw' }
  | { type: 'play'; card: number; target?: number; second?: number }
  | { type: 'end_turn' }
  | { type: 'reveal'; index: number }
  | { type: 'conspiracy_pick'; index: number }
  | { type: 'kill'; target: number }
  | { type: 'guard'; target: number }
  | { type: 'confess'; index: number }
  | { type: 'peek'; target: number; index: number }
  | { type: 'tide' }
  | { type: 'cure'; target: number };
