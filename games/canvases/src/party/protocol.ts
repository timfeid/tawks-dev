export type Feature = 'none' | 'cap' | 'leaf' | 'antenna' | 'long_ears' | 'round_ears' | 'horns' | 'beanie' | 'visor' | 'top_hat' | 'crown' | 'wizard';

export type Costume = {
  color: string;
  /** Cosmetic body width; head size and gameplay bounds stay the same. */
  body_width?: number | null;
  hat?: string | null; shirt?: string | null; pants?: string | null;
  shoes?: string | null; gloves?: string | null;
  /** Carried on the board in place of the plain sack. */
  bag?: string | null;
  /** Worn together, as many as you like. */
  extras?: string[] | null;
  model_id?: string | null;
  /** A free head swap; none is the round head. */
  head?: string | null;
  /** The worn PuckLab jersey's synced club design; none draws the default PuckLab jersey. Set by Tawks. */
  jersey?: JerseyDesign | null;
  /** The owned face reaction shown on hover and when celebrating. */
  reaction?: string | null;
  shuffle_reactions?: boolean;
  /** With shuffle on, every reaction owned, to pick from. Set by Tawks. */
  reaction_pool?: string[];
};

export type JerseyStripe = {
  pattern: 'single' | 'double' | 'triple' | 'bold' | 'thick_thin' | 'thin_thick_thin';
  color: string;
};
/** A Lab club jersey as design data: colours, stripes and PuckLab's own drawn emblem. */
export type JerseyDesign = {
  body: string;
  sleeves?: string | null;
  yoke?: string | null;
  collar?: string | null;
  trim?: string | null;
  sleeve_stripes?: JerseyStripe[];
  hem_stripes?: JerseyStripe[];
  crest?: { kind: 'emblem'; paths: string[]; view_box: number[]; color: string; back?: string | null } | null;
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

export type Phase =
  | { name: 'lobby' }
  | { name: 'board' }
  | {name:'map_finale'}
  | {name:'city_heist'}
  | { name: 'vote'; seconds: number; options: string[]; votes: BoutVote[] }
  | { name: 'table' }
  /** A free go at the bout before it counts: nothing is scored. */
  | { name: 'practice'; seconds: number }
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

export type SpaceKind =
  | 'start'
  | 'coin'
  | 'loss'
  | 'item'
  | 'event'
  | 'shop'
  | 'hazard'
  | 'teleport'
  | 'prize_spot'
  | 'cannon'
  | 'tide'
  | 'ice';
export type Item =
  | 'bat'
  | 'bomb'
  | 'drive_by'
  | 'slapshot'
  | 'slash_rush'
  | 'drive_car'
  | 'grapple'
  | 'trap'
  | 'double_dice'
  | 'golden_die'
  | 'swap'
  | 'shield'
  | 'medkit'
  | 'compass'
  | 'curse'
  | 'pickpocket'
  | 'heist'
  /** Picked up by suction and thrown as a barricade; never in the bag. */
  | 'crate'
  /** Packed from Frozen Peak's snowman and thrown to freeze someone; never in the bag. */
  | 'snowball';

export type BoardSpace = {
  id: number;
  x: number;
  z: number;
  kind: SpaceKind;
  next: number[];
  link: number | null;
  /** Shut until the last rounds, like the summit on Frozen Peak. */
  gated: boolean;
  /** Underground, in a tunnel; the camera follows a token down into it. */
  under?: boolean;
  /** Which of the map file's paths it was laid along, 0 for the lap. */
  path?: number;
};

export type BoardToken = {
  seat: number;
  space: number;
  previous?: number | null;
  facing?: number | null;
  /** Null when fog hides someone else's purse from this viewer. */
  coins: number | null;
  health: number;
  items: Item[];
  prizes: number;
  resting: number;
  shield: boolean;
  cursed: boolean;
  /** Who crossed them last, and owes them for it. */
  grudge: number | null;
  /** Holding a crate picked up by the vacuum, and a snowball from the snowman. */
  carrying?: boolean;
  snowball?: boolean;
  /** Vacuum upgrades bought at the shop, kept for the game. */
  upgrades?: Upgrade[];
};

export type BranchOption = {
  node: number;
  kind: SpaceKind;
  shortcut: boolean;
  saves: number;
};

export type ShopOffer = { item: Item; price: number };

/** The vacuum's upgrades, bought at the shop and kept on the token. */
export type Upgrade = 'wide_nozzle' | 'turbo' | 'big_battery' | 'filter';
export type UpgradeOffer = { upgrade: Upgrade; price: number };
export const UPGRADES: Record<Upgrade, { label: string; help: string }> = {
  turbo: { label: 'Turbo', help: 'Your vacuum spins up in half the time. Kept for the game.' },
  wide_nozzle: { label: 'Wide nozzle', help: 'A cone 1.4 times as wide. Every upgrade also adds 15% suction speed. Kept for the game.' },
  big_battery: { label: 'Big battery', help: '32 seconds of suction instead of 20. Kept for the game.' },
  filter: { label: 'Filter', help: "Rivals' vacuums cannot drain your coins, though they can still blow you back. Kept for the game." },
};

export type Taunt = 'laugh' | 'clap' | 'wave' | 'cry' | 'point' | 'hurry';

export type TurnStep =
  | { step: 'items' }
  | { step: 'position'; item: Item }
  | { step: 'roll' }
  | { step: 'walking' }
  | { step: 'driving' }
  | { step: 'raiding' }
  | { step: 'branch'; options: BranchOption[]; left: number }
  | { step: 'shop'; offers: ShopOffer[]; upgrades?: UpgradeOffer[] }
  | { step: 'cannon'; max: number }
  /** Something thrown in first person, a bomb or a grapple, hex, swap orb or sticky hand;
   * `wind` is in units a second, `fuse` in seconds. */
  | { step: 'lob'; item: Item; wind: [number, number]; fuse: number }
  /** A bat swung in first person. */
  | { step: 'swing'; fuse: number }
  /** The vacuum out, aimed in first person, with `seconds` of sucking. */
  | { step: 'vacuum'; seconds: number }
  /** An item used on yourself, or the heist, and its skill check: a needle
   * sweeping every `period` seconds, best stopped in the middle. */
  | { step: 'check'; item: Item; period: number; fuse: number; number: number | null }
  | { step: 'done' };

export type RoundEvent =
  | 'coin_shower'
  | 'windfall'
  | 'item_rain'
  | 'prize_moves'
  | 'underdog'
  | 'fog'
  | 'toll_road'
  | 'market_day'
  | 'earthquake'
  | 'last_round'
  /** Mature games only. */
  | 'bounty'
  | 'plague'
  | 'shakedown';

export type Bonus = 'champion' | 'explorer' | 'troublemaker';

export type BonusAward = {
  bonus: Bonus;
  seats: number[];
  count: number;
};

export type Highlight = 'biggest_swing' | 'longest_walk' | 'most_knockouts' | 'worst_dice';

export type HighlightAward = {
  highlight: Highlight;
  seat: number;
  value: number;
};

export type BoardLogEntry = Record<string, any> & { kind: string };
export type AirCoinView={id:number;space:number;x:number;z:number;height:number;coins:number;available:boolean;collected_by:number|null};

export type MapKey = 'island' | 'volcano' | 'shipwreck' | 'frozen';

export type WalkView = {
  id:number;seat:number;from:number;path:number[];waiting?:number;elapsed:number;delay:number;hop_seconds:number;
  eye_height:number;x:number;z:number;hop:number;yaw:number;pitch:number;vacuum_on:boolean;
  bonus:number;earned:number;max_bonus:number;
  cone:number;range:number;rate?:number;
  pulling?:[number,number][];power?:number;out?:number;blowing?:boolean;
  coins:{id:number;x:number;z:number;height:number;active:boolean;progress:number;collected:boolean}[];
};

export type PositionView = {
  seat:number;item:Item;x:number;z:number;home_space:number;home_x:number;home_z:number;
  remaining:number;active:boolean;speed:number;radius:number;
};

export type RaidView = {seat:number;item:'slash_rush'|'drive_by';ammo:number;shot_cooldown:number;hits:[number,number][];x:number;z:number;yaw:number;home_space:number;home_x:number;home_z:number;remaining:number;speed:number;radius:number;cooldown:number;slashing:number};

export type DriverView={seat:number;x:number;z:number;yaw:number;speed:number;remaining:number;arriving:number;home_x:number;home_z:number;home_space:number};

export type TrafficView={time:number;roads:{points:[number,number][];width:number}[];cars:{id:number;x:number;z:number;yaw:number;speed:number;braking:boolean}[];crossings:{id:number;x:number;z:number;axis:"x"|"z";pedestrian_green:boolean;remaining:number}[]};

export type CrawlerView = {controller:number|null;controller_round:number|null;pulled_by:number|null;blowing:boolean;strength:number;x:number;z:number;yaw:number;target:number|null;speed:number;contact_radius:number;grace:number;cooldowns:[number,number][]};
export type PedestrianView = {id:number;x:number;z:number;yaw:number;remaining_cash:number;robbed:boolean;walking:boolean;fleeing:boolean;knocked_down:boolean};
export type FinaleCheckpoint = {x:number;z:number;radius:number;blocked:boolean;via?:[number,number][]};
export type FinaleObstacle = {kind:'barrier'|'ice';x:number;z:number;yaw:number;half_width:number;half_depth:number};
export type FinaleRacer = {seat:number;x:number;z:number;yaw:number;next_checkpoint:number;waypoint?:number;finished:boolean;dnf:boolean;finish_time:number|null;penalties:number;stun:number;gauge:number;waiting:boolean;place:number;slipping?:boolean};
export type MapFinaleView = {map:MapKey;title:string;rules:string;elapsed:number;remaining:number;duration:number;ready:number;speed:number;checkpoints:FinaleCheckpoint[];racers:FinaleRacer[];tide_high:boolean;obstacles?:FinaleObstacle[]};

export type CandyVanView={x:number;z:number;yaw:number;price:number;range:number;phase:'offer'|'visit';joined_seats:number[];bought_seats:number[];can_join:boolean;can_buy:boolean;remaining:number};
export type CandyRunView={seat:number;x:number;z:number;home_x:number;home_z:number;home_space:number;remaining:number;speed:number};

export type BoardView = {
  candy_van?:CandyVanView|null;
  candy_runs?:CandyRunView[];
  finale?:MapFinaleView|null;
  pedestrians?:PedestrianView[];
  crawler?:CrawlerView|null;
  traffic?:TrafficView|null;
  driving?:DriverView[];
  raids?:RaidView[];
  positioning?:PositionView|null;
  walk?: WalkView | null;
  air_coins?:AirCoinView[];
  air_remaining?:number;
  /** How much bigger than its drawing the board is; scenery thickens to match. */
  world_scale?: number;
  map: MapKey;
  spaces: BoardSpace[];
  tokens: BoardToken[];
  standings?:number[][];
  /** Only the traps you know about: your own, and ones you have walked past. */
  traps: { space: number; mine: boolean }[];
  /** Coins lying on the board, for anyone who walks over them. */
  loose: { space: number; coins: number }[];
  prize_at: number;
  prize_cost: number;
  /** Every spot a prize can be bought at right now. */
  open_prizes: number[];
  order: number[];
  turn_seat: number;
  turn: TurnStep;
  round: number;
  rounds: number;
  last_roll: number | null;
  dice: number;
  fixed_roll: number | null;
  event: RoundEvent | null;
  /** Who buys the prize cheaply this round, and what it costs them. */
  underdog: number | null;
  underdog_cost: number;
  /** Who is alone in last place and rolls an extra die of up to 3. */
  handicap: number | null;
  /** What everyone has staked on the coming bout, and the most anyone may. */
  bets: { seat: number; coins: number }[];
  max_bet: number;
  bonuses: BonusAward[];
  /** What the party will be remembered for. */
  highlights: HighlightAward[];
  /** Where the storm is, and where it moves next, so it can be seen coming. */
  storm: number | null;
  storm_to: number | null;
  /** Whether the tide is in, on a board with tide spaces; it turns every round. */
  tide: boolean | null;
  /** The round gated ways such as the summit open in, on a board that has them, and whether they are open now. */
  gates_open_from: number | null;
  gates_open: boolean;
  log: BoardLogEntry[];
  /** Crates lying about for suction, and barricades across the way, by space. */
  crates?: number[];
  blockades?: number[];
  /** Whether the player taking their turn has used their vacuum yet, and the vacuum while it is out. */
  sucked?: boolean;
  vacuum?: VacuumView | null;
  /** The board's own things for the vacuum: coconuts on each palm, each
   * snowman built or not, boxes taken this round, lava rocks by space, and the
   * flotsam points with something bobbing at them. */
  coconuts?: number[];
  snowmen?: boolean[];
  boxes_taken?: number[];
  lava_rocks?: number[];
  flotsam?: number[];
  /** In a mature game, who has a price on their head, and who has the plague. */
  bounty?: number | null;
  plague?: number | null;
};

export type Member = {
  costume?: Costume | null;
  user_id: string;
  name: string;
  avatar_url: string;
  character: string;
  seat: number;
  is_bot: boolean;
  connected: boolean;
  wins: number;
  in_round: boolean;
  /** Done practising, ready for the bout to count. */
  ready?: boolean;
};

export type CitySide='police'|'crew';
export type CityView={round:number;jobs:number;remaining:number;briefing:{remaining:number;ready:number;total:number;you_ready:boolean}|null;night_score:[number,number];job_wait:number;patrol_route:[number,number][];dispatch:string|null;draw:boolean;ready:number;scores:[number,number];actors:{seat:number;side:CitySide;x:number|null;z:number|null;yaw:number;jailed:boolean;stunned:number;carrying:boolean;cuffed:number;acting:boolean;suction:boolean;automated:boolean;driving:boolean;car:[number,number]|null;jail_seconds:number;height:number;velocity:[number,number]}[];objective:string;targets:{label:string;x:number;z:number;progress:number}[];bank:[number,number];jail:[number,number];escape:[number,number]|null;alarm:boolean;alert:[number,number]|null;loot:[number,number]|null;loot_carrier:number|null;vault_progress:number|null;power_seconds:number|null;smokes:[number,number,number][];tools:[number,number];scanner:string|null;scanner_bearing:number|null;resolved:CitySide|null;reason:string|null;resolved_for:number};
export type RoomView = {
  city_mode?:boolean;
  city_config?:{police:number;jobs:number;host_police:boolean};
  city?:CityView|null;
  opening_minigame?:boolean;
  minigame_rewards?:{seat:number;place:number;coins:number;tier_one_chance:number}[];
  rounds?:number;
  match_id: string;
  /** Set up for a whole Tawkathon rather than a single bout; then the lobby
   * previews the board it would be played on. */
  party_mode?: boolean;
  preview_board?: BoardView | null;
  /** A dev stack, where the host can open the sandbox; and whether this is it. */
  dev_tools?: boolean;
  sandbox?: boolean;
  /** Cartoon gore and the dark round events, on the host's say. */
  mature?: boolean;
  /** Played in pairs, and each seat's pair (by `team`) once the Tawkathon starts. */
  pairs_on?: boolean;
  pairs?: { seat: number; team: number }[];
  /** The bout you can still rate, if you played the last one. */
  /** The last bout, for you to rate; `rated` is the stars you tapped, while a note can still follow. */
  feedback?: { number: number; key: string; name: string; rated?: number | null } | null;
  minigame: MinigameKey | string;
  minigame_name: string;
  bouts: BoutSpec[];
  table_games: [string, string][];
  bot_skill: number;
  bot_skill_name: string;
  skills: [string, number][];
  /** The board the next Tawkathon is played on, and the boards to pick from. */
  map: MapKey;
  maps: [MapKey, string][];
  host_id: string | null;
  phase: Phase;
  members: Member[];
  max_seats: number;
  board: BoardView | null;
  setup: GameSetup;
  /** Sides for the bout being played, when it is played in teams. */
  teams: { seat: number; team: number }[];
  recording?: RoomRecording | null;
};

export type RoomRecording = { by: string; by_user_id: string; since: number; live: boolean };

export type MinigameKey = 'bumper' | 'shooter' | 'dodgeball' | 'race' | 'redlight' | 'crumble' | 'hotpotato' | 'balloon' | 'sumo' | 'ledge' | 'coins' | 'target' | 'popgun' | 'ricochet' | 'retake';

export type ShooterBlock = { x: number; z: number; hx: number; hz: number; top: number; bottom?: number; kind?: string };

export type ShooterSetup = {
  game: 'shooter' | 'target' | 'popgun' | 'ricochet' | 'retake' | 'speedball';
  half: number;
  half_z?: number;
  /** How much the map for four was spread out for a bigger crowd; 1 when not. */
  scale?: number;
  hp: number;
  eye: number;
  height: number;
  radius: number;
  blocks: ShooterBlock[];
  /** Speedball: where each team hangs the flag. */
  stations?: { team: number; x: number; z: number; half_z: number }[];
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

export type ArenaGame = 'crumble' | 'hotpotato' | 'balloon' | 'sumo' | 'ledge' | 'coins' | 'tug';

export type ArenaSetup =
  | { game: 'crumble'; rule: string; tile: number; radius: number; tiles: { x: number; z: number }[] }
  | { game: 'hotpotato'; rule: string; radius: number }
  | { game: 'balloon'; rule: string; half: number; balloons: number }
  | { game: 'sumo'; rule: string; radius: number; min_radius: number }
  | {
      game: 'ledge';
      rule: string;
      radius: number;
      /** The middle stone, before it wears down to `min_core` at the end. */
      core: number;
      min_core: number;
      /** Innermost first; each ring's sections are numbered on from the last. */
      rings: LedgeRing[];
      reach: number;
      /** The cosine of the shove's half-angle. */
      cone: number;
      windup: number;
    }
  | { game: 'coins'; rule: string; radius: number }
  | { game: 'tug'; rule: string; half: number; half_z: number; pit: number; brink: number };

export type LedgeRing = { inner: number; outer: number; count: number; turn: number };

/** A ledge section: 0 solid, 1 cracking, 2 falling, 3 gone. */
export type LedgeFooting = 0 | 1 | 2 | 3;

export type ArenaCoin = { id: number; x: number; y: number; z: number; value: number };

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
  skidding?: boolean;
  safe?: boolean;
  ready?: boolean;
  purse?: number;
  team?: number;
  heaves?: number;
  /** Tug of War: how recently they hauled, for the lean. */
  pull?: number;
  /** Ledge Shuffle: which way a shove goes, and what they are doing. */
  yaw?: number;
  winding?: boolean;
  braced?: boolean;
  knockoffs?: number;
};

export type ArenaSnap = {
  game: ArenaGame;
  time: number;
  limit: number;
  players: ArenaPlayer[];
  /** Sumo: the platform as it stands, shrinking all round. */
  radius?: number;
  tiles?: { x: number; z: number; state: 'solid' | 'cracking' | 'falling' | 'gone' }[];
  /** Ledge Shuffle: every section's footing, and the core as it stands. */
  sections?: LedgeFooting[];
  core?: number;
  holder?: number | null;
  fuse?: number;
  coins?: ArenaCoin[];
  /** Seconds until the next gold coin falls, or null once they have all gone. */
  next_gold?: number | null;
  /** Tug of War: where the knot sits, negative being red's way. */
  rope?: number;
  brink?: number;
  dunked?: number | null;
};

export const ARENA_GAMES: ArenaGame[] = ['crumble', 'hotpotato', 'balloon', 'sumo', 'ledge', 'coins', 'tug'];

export type BattleGame = 'tank' | 'titan';

export type BattleSetup =
  | {
      game: 'tank';
      reload?:number;
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
  game?: 'bumper' | 'crumble' | 'hotpotato' | 'balloon' | 'sumo' | 'ledge';
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
  game: 'shooter' | 'target' | 'popgun' | 'ricochet' | 'retake' | 'speedball';
  time: number;
  limit: number;
  players: ShooterPlayer[];
  targets?: { id: number; owner: number; x: number; y: number; z: number; up: boolean }[];
  scores?: Record<string, number>;
  flag?: { x: number; z: number; holder: number | null };
  round?: {
    number: number;
    wins: [number, number];
    to_win: number;
    between: number;
    last: { winner: number | null; how: string } | null;
  };
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
  !!setup && ['shooter', 'target', 'popgun', 'ricochet', 'retake', 'speedball'].includes(setup.game);

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
  | { kind: 'flag_taken'; seat: number; team: number | null }
  | { kind: 'flag_dropped'; seat: number; x: number; z: number }
  | { kind: 'flag_hung'; seat: number; team: number }
  | { kind: 'round_end'; round: number; winner: number | null; how: string; wins: [number, number] }
  | { kind: 'round_start'; round: number }
  | { kind: 'retake_failed' }
  | { kind: 'crack'; seat: number; x: number; z: number }
  | { kind: 'tile_fall'; x: number; z: number }
  | { kind: 'drop'; seat: number }
  | { kind: 'pass'; from: number; to: number; fuse: number }
  | { kind: 'boom'; seat: number; x: number; z: number }
  | { kind: 'light'; seat: number }
  | { kind: 'pop'; seat: number; by: number; left: number }
  | { kind: 'burst'; seat: number; by: number }
  | { kind: 'heave'; seat: number; team: number }
  | { kind: 'dunked'; team: number }
  | { kind: 'over'; seat: number; by?: number | null }
  | { kind: 'ledge_windup'; seat: number; yaw: number }
  | { kind: 'ledge_shove'; seat: number; x: number; z: number; yaw: number; landed: boolean; blocked: boolean }
  | { kind: 'ledge_hit'; seat: number; by: number; x: number; z: number }
  | { kind: 'ledge_block'; seat: number; by: number; x: number; z: number }
  | { kind: 'ledge_crack'; section: number }
  | { kind: 'ledge_fall'; section: number }
  | { kind: 'grab'; seat: number; value: number; purse: number; x: number; z: number }
  | { kind: 'spill'; seat: number; by: number; coins: number }
  | { kind: 'gold'; x: number; z: number; value: number }
  | { kind: 'shell'; seat: number; id: number; x: number; z: number }
  | { kind: 'ricochet'; id: number; x: number; z: number }
  | { kind: 'splinter'; x: number; z: number }
  | { kind: 'wreck'; seat: number; by: number; x: number; z: number }
  | { kind: 'rollout'; seat: number }
  | { kind: 'off'; seat: number }
  | { kind: 'shove'; by: number; at: number }
  | { kind: 'jump'; seat: number }
  | { kind: 'crumble'; pad: number }
  | { kind: 'regrow'; pad: number }
  | { kind: 'dunked'; seat: number }
  | { kind: 'wind_up'; attack: string }
  | { kind: 'strike'; attack: string; x: number; z: number }
  | { kind: 'winded' }
  | { kind: 'recovered' }
  | { kind: 'floored'; seat: number; by: string }
  | { kind: 'swing'; seat: number }
  | { kind: 'strike_core'; seat: number; x: number; z: number; left: number }
  | { kind: 'titan_down' };

export type FeedbackResult={number:number;with_note:boolean;saved:boolean;rewarded:boolean;message?:string};

export type ServerMsg =
  | { type: 'welcome'; user_id: string; match_id: string; roster: CharacterDef[]; tick_hz: number; snapshot_hz: number; guest?: boolean }
  | { type: 'room'; room: RoomView; spectating: boolean }
  | { type: 'snap'; tick: number; game: GameSnap; events: GameEvent[] }
  | {type:'feedback_result'}&FeedbackResult
  | { type: 'pong'; t: number }
  | { type: 'table'; game: string; view: unknown }
  | { type: 'error'; message: string; code?: string };

export type ClientMsg =
  | { type: 'walk_vacuum'; yaw:number; pitch:number; on:boolean; blow?:boolean }
  | { type: 'refresh_costume' }
  | { type: 'hello'; token: string }
  | { type: 'input'; x: number; z: number; yaw?: number; pitch?: number; jump?: boolean; crouch?: boolean }
  | { type: 'fire'; yaw: number; pitch: number }
  | { type: 'press'; hand: 'left' | 'right' | 'both' }
  | { type: 'choose_game'; key: string }
  | { type: 'vote'; key: string }
  | { type: 'feedback'; rating: number; note: string }
  | { type: 'face'; yaw: number }
  | {type:'drive_input';throttle:number;steer:number;horn?:boolean}
  | {type:'raid_input';x:number;z:number;yaw:number}
  | {type:'raid_slash'}
  | {type:'raid_fire';on:boolean;yaw:number}
  | {type:'crawler_input';x:number;z:number}
  | {type:'join_candy_visit'}
  | {type:'candy_visit_input';x:number;z:number}
  | {type:'buy_candy'}
  | { type: 'position_input'; x:number; z:number }
  | { type: 'activate_item' }
  | { type: 'cancel_item' }
  | { type: 'choose_mode'; party: boolean }
  | {type:'choose_city';police:number;jobs:number;host_police?:boolean}
  | {type:'start_city';watch?:boolean}
  | {type:'city_action';held:boolean;suction?:boolean}
  | {type:'city_tool';tool:'smoke'|'scanner'}
  | {type:'city_automate';on:boolean}
  | {type:'city_ready'}
  | {type:'city_vehicle'}
  | { type: 'choose_skill'; level: string }
  | { type: 'choose_map'; map: MapKey }
  | { type: 'pick'; character: string }
  | { type: 'start' }
  | { type: 'start_party' }
  | { type: 'start_sandbox'; costume?: Costume }
  | { type: 'choose_mature'; on: boolean }
  | {type:'choose_rounds';rounds:number}
  | { type: 'choose_pairs'; on: boolean }
  | { type: 'ready' }
  | { type: 'dev'; tool: DevTool }
  | { type: 'use_item'; item: Item; target: number | null; number?: number }
  | { type: 'shop'; item: Item | null; upgrade?: Upgrade | null }
  | { type: 'taunt'; taunt: Taunt }
  | { type: 'bet'; coins: number }
  | { type: 'fire_cannon'; power: number }
  /** Throw what is in hand: `yaw` the way (0 is +z, a quarter turn +x), `pitch` in radians, `power` 0 to 1. */
  | { type: 'lob'; yaw: number; pitch: number; power: number }
  /** Stop the skill check of the item in hand: 0 to 1, best at 0.5. */
  | { type: 'check'; stop: number }
  | { type: 'suck' }
  | { type: 'vacuum'; yaw: number; on: boolean; blow?: boolean }
  | { type: 'stow_vacuum' }
  | { type: 'brace'; on: boolean }
  | { type: 'throw_crate' }
  | { type: 'throw_snowball' }
  /** Swing the bat toward `yaw`, the timing needle stopped at `stop`: 0 to 1, best at 0.5. */
  | { type: 'swing'; yaw: number; stop: number }
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

/** The sandbox's tools, on a dev stack. */
export type DevTool =
  | { action: 'give'; item: Item }
  | { action: 'coins'; coins: number }
  | { action: 'heal' }
  /** A prize for you, or with `seat` for anyone. */
  | { action: 'prize'; seat?: number }
  | { action: 'zombie_kill'; seat?: number }
  /** Your own token, or with `seat` anyone's, like a bot dragged there. */
  | { action: 'teleport'; space: number; seat?: number }
  | {action:'finale'}
  | {action:'crawler'}
  | {action:'candy_van'}
  | { action: 'exit' };

/** The bomb's flight, as the board works it out (see `Board::bomb_landing`). */
export const BOMB = { speed: 26, gravity: 20, hand: 1.4, directRadius: 1.7, blastRadius: 4.6 };
/** How far the bat reaches and how wide its arc is either side, as the board has it. */
export const BAT = { range: 5, arc: 0.75 };

/** Where a bomb thrown this way is after `t` seconds, from where it left the hand. */
export function bombAt(yaw: number, pitch: number, power: number, wind: [number, number], t: number) {
  const speed = Math.min(1, Math.max(0, power)) * BOMB.speed;
  const along = speed * Math.cos(pitch);
  return {
    x: (Math.sin(yaw) * along + wind[0]) * t,
    y: BOMB.hand + speed * Math.sin(pitch) * t - (BOMB.gravity * t * t) / 2,
    z: (Math.cos(yaw) * along + wind[1]) * t,
  };
}

/** How long it is in the air before it comes back down to the height it was thrown from. */
export function bombFlight(pitch: number, power: number) {
  const up = Math.min(1, Math.max(0, power)) * BOMB.speed * Math.sin(pitch);
  return (up + Math.sqrt(up * up + 2 * BOMB.gravity * BOMB.hand)) / BOMB.gravity;
}

/** The vacuum while it is out: where it points, whether it is sucking, the
 * seconds of sucking left, how long it has been out (where the chests have
 * drifted to), what it is drawing in and how far along, and who it is draining.
 * Each thing drawn in is keyed by its kind (see `PULL`) times 256 plus its
 * space or its index among the board's specials. */
/** The vacuum while it is out. `power` is the motor spinning up, 0 to 1;
 * `braced` are the drained rivals holding on against it. While `blowing`,
 * `pulling` is how far along each thing is to being blown off. `cone` is
 * its half width in radians, wider with the wide nozzle. */
export type VacuumView = {
  yaw: number;
  on: boolean;
  blowing?: boolean;
  cone?: number;
  range?:number;
  rate?:number;
  power: number;
  left: number;
  out: number;
  pulling: [number, number][];
  siphoning: number[];
  braced: number[];
};

/** What the vacuum can pull, as the board keys it. */
export const PULL = { loose: 0, crate: 1, chest: 2, box: 3, storm: 4, palm: 5, lava: 6, flotsam: 7, snowman: 8, air:9, pedestrian:11 } as const;
export const pullKey = (kind: number, index: number) => kind * 256 + index;

/** How far the vacuum reaches and how wide its cone is, as the board has it. */
export const VACUUM = {
  range: 14,
  cone: 0.22,
  seconds: 20,
  edge: 0.05,
  far: 0.4,
  spinUp: 0.45,
  knockAfter: 0.5,
  wideCone: 1.4,
  wideStrength: 0.85,
  turbo: 0.5,
  bigBattery: 7,
};

/** Where a chest hangs while the vacuum is out, off its spot (see `Board::chest_drift`). */
export function chestDrift(out: number): [number, number] {
  return [Math.sin(out * 1.1) * 3.2, Math.cos(out * 0.8) * 2.2];
}

/** Each pair's colour, by its index, so partners read as one side everywhere. */
export const PAIR_COLORS = ['#f97316', '#22d3ee', '#a3e635', '#e879f9'];
