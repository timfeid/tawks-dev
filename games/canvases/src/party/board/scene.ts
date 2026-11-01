import * as THREE from 'three';
import { BoardCrawler } from './crawler';
import { BoardTraffic } from './traffic';
import {StaticProps}from'./static-props';
import { BoardRental } from './rental';
import { BoardCandyVan } from './candy-van';
import { buildCharacter, type CharacterRig } from '../characters';
import { characterAppearanceKey } from '../appearance';
import { vacuumPoint } from './vacuum-geometry';
import { Die } from './dice';
import { ITEMS } from './items';
import { grappleFlightSeconds, grappleTravelProgress } from './grapple';
import { impactSweep, spikeImpactClock, SPIKE_SWEEP_SECONDS, SPIKE_IMPACT_SECONDS } from './impact';
import { ITEM_ICONS } from './item-icons';
import { BAG_SHOULDER, buildBag, type CarriedBag } from '../bags';
import { iconPaths } from './glyphs';
import { SPACE_HELP } from './space-help';
import { Terrain } from './terrain';
import type { Ground, PartySfx } from '../sfx';
import { buildTunnels, findTunnels, tunnelHeights } from './tunnels';
import { DESIGNS } from './design';
import { dressDesign } from './dress-design';
import { BAT, PAIR_COLORS, PULL, UPGRADES, VACUUM, chestDrift, pullKey, type Upgrade, type VacuumView } from '../protocol';
import { WATER_Y } from '../stage';
import { place, placed } from './models';
import { BoardWeapons, type BoardWeapon } from './weapons';
import { PedestrianScene } from './pedestrians';
import { MapFinaleScene } from './finale';
import { disposeGlyphs, glyphTexture } from './glyphs';
import { DRESSINGS, groundPatch, iceSheet, lavaPool, onDeck, type Dresser } from './dressing';
import type { BoardLogEntry, BoardToken, BoardView, CharacterDef, Item, MapKey, Member, SpaceKind } from '../protocol';

/** The worn path laid between tiles, in each board's ground. */
const PATH_COLORS: Record<MapKey, string> = {
  island: '#e9dcb6',
  volcano: '#84776d',
  shipwreck: '#f1e2bd',
  frozen: '#8696a5',
};

const SPACE_COLORS: Record<SpaceKind, string> = {
  cannon: '#f97316',
  start: '#ffd23f',
  coin: '#4cc9f0',
  loss: '#ff5f6d',
  item: '#9b5de5',
  event: '#62c370',
  shop: '#ffa62b',
  hazard: '#7f1d3a',
  teleport: '#22d3ee',
  prize_spot: '#f8f0d0',
  tide: '#14b8a6',
  ice: '#bae6fd',
};

/** Whoever minds the shops: one look for every stall on every board. */
const SHOPKEEPER: CharacterDef = {
  id: 'shopkeeper',
  name: 'Shopkeeper',
  body: '#f6c47a',
  accent: '#b4532a',
  belly: '#fff4e0',
  feature: 'cap',
};

type Shop = { at: THREE.Vector3; facing: number; keeper: CharacterRig };

const HOP_SECONDS = 0.32;
const PRIZE_FLIGHT_SECONDS = 2.2;
/** How long a cannon shot stays in the air. */
const FLIGHT_SECONDS = 1.6;
/** How long the die spins and shows its number before the token sets off. */
const DIE_SECONDS = 2.6;
/** Sharing a space with the bat is in reach whichever way it faces, as in board.rs. */
const BAT_CLOSE = 0.8;

type TokenView = {
  seat: number;
  characterId: string;
  appearanceKey: string;
  rig: CharacterRig;
  /** The bag bought for them, or null for the plain sack. */
  bagStyle: string | null;
  bagTint: string | null;
  space: number;
  from: THREE.Vector3;
  to: THREE.Vector3;
  hop: number;
  yaw: number;
  tag: THREE.Sprite;
  tagKey: string;
  shake: number;
  /** Spaces still to hop through, one at a time. */
  path: THREE.Vector3[];
  /** Seconds to hold still first, while the die is rolled. */
  wait: number;
  /** Shot from a cannon, so the next hop arcs high and long. */
  flying: boolean;
  /** The character's colours, for the pieces it bursts into in a mature game. */
  colors: string[];
  /** Seconds left in pieces after a knockout, in a mature game. */
  burst: number;
  positioning: {
    at: THREE.Vector3; target: THREE.Vector3; from: THREE.Vector3;
    returning: boolean; elapsed: number; active: boolean;
  } | null;
};

/** A drop of cartoon gore or a piece of a burst character, flying until it lands. */
type Bit = { mesh: THREE.Mesh; velocity: THREE.Vector3; spin: THREE.Vector3; life: number; floor: number };
/** A pool or scorch left on the ground, fading. */
type Stain = { mesh: THREE.Mesh; life: number; max: number };

/** What the board wants the camera to do, and how strongly, from 0 to 1. */
export type BoardShot = {
  at: THREE.Vector3;
  wide: number;
  tall: number;
  pitch: number;
  yaw: number;
  weight: number;
};

type Floater = { sprite: THREE.Sprite; life: number; rise: number };
/** A flash, ring or puff that grows and fades; `rise` carries it upwards, as smoke. */
type Effect = { object: THREE.Mesh | THREE.Line; life: number; max: number; grow: number; rise?: number };

const itemLabel = (item: unknown) => ITEMS[item as Item]?.label ?? String(item);

/** How big the sack at a token's side is with nothing in it, then each item more: silly by the last. */
const BAG_SIZES = [0.6, 0.95, 1.3, 1.75, 2.3, 3];

/** A sack at a token's side, and the strap from it over the shoulder. */
type Bag = { pivot: THREE.Group; sack: THREE.Group; strap: THREE.Mesh; built: CarriedBag; style: string | null; tint: string | null; held: number; size: number; pop: number };
/** An item on its way into a sack, or flung out of a full one. */
type BagFlight = { sprite: THREE.Sprite; seat: number; start: THREE.Vector3; age: number; life: number; out: boolean };

/** An item as a round badge in its colour, its icon in dark ink, as on the tags. */
function badgeTexture(item: Item) {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 128;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = 'rgba(15,23,42,0.9)';
  ctx.beginPath();
  ctx.arc(64, 64, 62, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = ITEMS[item].color;
  ctx.beginPath();
  ctx.arc(64, 64, 54, 0, Math.PI * 2);
  ctx.fill();
  ctx.translate(64 - 12 * 2.7, 64 - 12 * 2.7);
  ctx.scale(2.7, 2.7);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.strokeStyle = '#0f172a';
  ctx.lineWidth = 2.4;
  for (const path of iconPaths(ITEM_ICONS[item])) ctx.stroke(path);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/** The status tag's canvas, and the round item badges along its bottom. */
const TAG_W = 560;
const TAG_H = 120;
const BADGE = 44;
const BADGE_TOP = 64;
const BADGE_GAP = 10;

/** Where each held item's badge sits on the tag, left to right. */
function badgeLayout(count: number) {
  const total = count * BADGE + (count - 1) * BADGE_GAP;
  const left = (TAG_W - total) / 2;
  return Array.from({ length: count }, (_, index) => ({ index, x: left + index * (BADGE + BADGE_GAP) }));
}

function heart(ctx: CanvasRenderingContext2D, x: number, y: number, size: number) {
  const s = size / 2;
  ctx.beginPath();
  ctx.moveTo(x, y + s * 0.9);
  ctx.bezierCurveTo(x - s * 1.6, y - s * 0.2, x - s * 0.6, y - s * 1.4, x, y - s * 0.45);
  ctx.bezierCurveTo(x + s * 0.6, y - s * 1.4, x + s * 1.6, y - s * 0.2, x, y + s * 0.9);
  ctx.closePath();
}

/// Hearts on top, ringed blue under a shield and purple under a curse,
/// then a round badge with its icon for each item held.
function statusTexture(token: BoardToken, maxHealth: number) {
  const canvas = document.createElement('canvas');
  canvas.width = TAG_W;
  canvas.height = TAG_H;
  const ctx = canvas.getContext('2d')!;
  ctx.lineJoin = 'round';
  const heartSize = 34;
  const heartsWidth = maxHealth * 42;
  for (const [on, color, pad] of [
    [token.shield, '#93c5fd', 6],
    [token.cursed, '#c084fc', 12],
  ] as const) {
    if (!on) continue;
    ctx.lineWidth = 5;
    ctx.strokeStyle = color;
    ctx.beginPath();
    ctx.roundRect((canvas.width - heartsWidth) / 2 - pad, 6 - pad / 2, heartsWidth + pad * 2, 50 + pad, 25);
    ctx.stroke();
  }
  for (let i = 0; i < maxHealth; i++) {
    const x = (canvas.width - heartsWidth) / 2 + 21 + i * 42;
    heart(ctx, x, 30, heartSize);
    ctx.lineWidth = 6;
    ctx.strokeStyle = 'rgba(15,23,42,0.9)';
    ctx.stroke();
    ctx.fillStyle = i < token.health ? '#f43f5e' : '#475569';
    ctx.fill();
  }
  ctx.font = '700 26px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  if (token.resting > 0) {
    const text = 'Resting';
    const w = ctx.measureText(text).width + 24;
    ctx.fillStyle = 'rgba(15,23,42,0.85)';
    ctx.beginPath();
    ctx.roundRect((canvas.width - w) / 2, 66, w, 40, 20);
    ctx.fill();
    ctx.fillStyle = '#cbd5e1';
    ctx.fillText(text, (canvas.width - w) / 2 + 12, 87);
  } else if (token.items.length) {
    for (const badge of badgeLayout(token.items.length)) {
      const item = token.items[badge.index];
      const cx = badge.x + BADGE / 2;
      const cy = BADGE_TOP + BADGE / 2;
      ctx.fillStyle = 'rgba(15,23,42,0.85)';
      ctx.beginPath();
      ctx.arc(cx, cy, BADGE / 2 + 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = ITEMS[item].color;
      ctx.beginPath();
      ctx.arc(cx, cy, BADGE / 2, 0, Math.PI * 2);
      ctx.fill();
      // The item's icon in dark ink, like a space's glyph on a light tile.
      ctx.save();
      const scale = (BADGE * 0.6) / 24;
      ctx.translate(cx - 12 * scale, cy - 12 * scale);
      ctx.scale(scale, scale);
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.strokeStyle = '#0f172a';
      ctx.lineWidth = 2.4;
      for (const path of iconPaths(ITEM_ICONS[item])) ctx.stroke(path);
      ctx.restore();
    }
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function tagKey(token: BoardToken) {
  return `${token.health}|${token.resting}|${token.shield}|${token.cursed}|${token.items.join(',')}`;
}

function labelTexture(text: string, color: string) {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = '700 44px system-ui, sans-serif';
  canvas.width = Math.ceil(ctx.measureText(text).width) + 36;
  canvas.height = 64;
  ctx.font = '700 44px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 8;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = 'rgba(15,23,42,0.85)';
  ctx.strokeText(text, 18, 34);
  ctx.fillStyle = color;
  ctx.fillText(text, 18, 34);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The party board: one tile per space on a floating plate, characters
 * standing on their tile, and pop-up numbers for coins, hits and traps.
 */
export class PartyBoard {
  readonly group = new THREE.Group();
  private crawler = new BoardCrawler();
  private traffic = new BoardTraffic();
  private rental = new BoardRental();
  private candyVan = new BoardCandyVan();
  /** Where the board's sounds play, once the page has one. */
  sounds: PartySfx | null = null;
  /** The spaces on a ship's deck, for footsteps on planks. */
  private deckSpaces = new Set<number>();
  private tiles = new Map<number, THREE.Mesh>();
  /** The coins, crates and dropped piles, hidden while something is being aimed. */
  private pickups: THREE.Object3D[] = [];
  private clear = false;
  /** Dots along the start of the arc while a bomb is aimed. */
  private arc: THREE.Points | null = null;
  /** A token being dragged in the sandbox, held over the space it would drop on. */
  private held: { seat: number; at: THREE.Vector3 } | null = null;
  private hiddenTag: number | null = null;
  private hiddenBody: number | null = null;
  private localFacing: {seat:number;yaw:number;at:number} | null = null;
  private lastView: BoardView | null = null;
  private coinGeometry: THREE.CylinderGeometry | null = null;
  private coconutMaterial: THREE.MeshStandardMaterial | null = null;
  private coinMaterial: THREE.MeshStandardMaterial | null = null;
  private walkClock: {id:number;elapsed:number;at:number} | null = null;
  /** Historical snapshots already carry their presentation clock. */
  replaying = false;
  private walkCoins: THREE.Mesh<THREE.CylinderGeometry, THREE.MeshStandardMaterial>[] = [];
  private stormFlights=new Map<number,{from:THREE.Vector3;to:THREE.Vector3;elapsed:number}>();
  private spikeHits=new Map<number,{at:THREE.Vector3;elapsed:number;impact:boolean;coins:number;hurt:BoardLogEntry[]}>();
  /** The vacuum while it is out: whose it is, the cone and the stream into it,
   * and when its view came, so the chest's drift runs on smoothly between views. */
  private vacuumSeat: number | null = null;
  private vacuumView: VacuumView | null = null;
  private vacuumAt = 0;
  private vacuumYaw = 0;
  private vacuumReticle: THREE.Group | null = null;
  private vacuumStream: THREE.Mesh[] = [];
  /** A ring under each token in its pair's colour, when playing in pairs. */
  private pairRings = new Map<number, THREE.Mesh>();
  pairs: { seat: number; team: number }[] = [];
  /** The board's own things for the vacuum, by their pull key: the palms'
   * coconuts, lava rocks, flotsam, the snowman, and the purple spaces' boxes. */
  private specialModels = new Map<number, THREE.Object3D>();
  private specials: { palms: [number, number][]; flotsam: [number, number][]; snowmen: [number, number][] } = { palms: [], flotsam: [], snowmen: [] };
  /** Crates lying about, and barricades thrown up, by the space they are on. */
  private crateModels = new Map<number, THREE.Object3D>();
  private barricadeModels = new Map<number, THREE.Object3D>();
  /** Coins on their way from one place to another, each on a path and a flip of its own. */
  /** Big words stamped over the action, popping in and then fading, as K.O.! */
  private stamps: { sprite: THREE.Sprite; age: number; life: number; size: number }[] = [];
  /** How far down each knocked-out token is lying, 0 standing to 1 flat, and the stars over them. */
  private downed = new Map<number, { amount: number; stars: THREE.Group }>();
  private knockedNow = new Set<unknown>();
  private bags = new Map<number, Bag>();
  private strapGeometry: THREE.BufferGeometry | null = null;
  private bagFlights: BagFlight[] = [];
  /** Items each seat has been given that are still in the air, so its sack waits for them. */
  private bagIncoming = new Map<number, number>();
  private badges = new Map<Item, THREE.CanvasTexture>();
  private coinsInFlight: {
    mesh: THREE.Mesh; from: THREE.Vector3; to: THREE.Vector3; t: number; seconds: number;
    side: THREE.Vector3; height: number; flip: number; phase: number; yaw: number; index: number; last: boolean;
  }[] = [];
  /** A mature game: hits splatter, knockouts burst, bombs scorch. */
  mature = false;
  private bits: Bit[] = [];
  private stains: Stain[] = [];
  // Made in the constructor, once the lists they are tracked in exist.
  private bitGeometry!: THREE.IcosahedronGeometry;
  private chunkGeometry!: THREE.BoxGeometry;
  private bloodMaterial!: THREE.MeshStandardMaterial;
  /** Over whoever has a price on their head, and fumes round whoever has the plague. */
  private bountyMark: THREE.Mesh | null = null;
  private fumes: THREE.Mesh[] = [];
  /** Whoever is aiming something, the way they face, and the ring and arrow marking them. */
  private aimer: { seat: number; yaw: number } | null = null;
  private aimerMark: THREE.Group | null = null;
  /** The bat in a token's hands: on its shoulder while aimed, then swung through the arc. */
  private bats = new Map<number, { pivot: THREE.Group; yaw: number; swing: number | null }>();
  private batHolder: number | null = null;
  /** A mark over each rival the bat would hit, while it is swung. */
  private batMarks = new Map<number, THREE.Mesh>();
  private batInReach = new Set<number>();
  /** Bombs in the air, each landing when its time is up. */
  private flights: {
    bomb: THREE.Object3D;
    from: THREE.Vector3;
    to: THREE.Vector3;
    t: number;
    seconds: number;
    /** A line back to the thrower's hand, for the grapple. */
    rope?: boolean;
    /** Normal-speed duration before the successful hook's local slow catch. */
    slowCatchFlight?: number;
    line?: THREE.Line;
    land: () => void;
  }[] = [];
  private weapons: BoardWeapons;
  private pedestrians: PedestrianScene;
  private finale: MapFinaleScene;
  private trapMarks = new Map<number, THREE.Mesh>();
  /** A token flying somewhere the camera should go along with. */
  private riding: number | null = null;
  /** A see-through bubble round each shielded token. */
  private bubbles = new Map<number, THREE.Mesh>();
  /** Seats left where they are until a throw that moves them comes down. */
  private stayPut = new Set<number>();
  /** Props hidden because they stand between a close camera and what it looks at. */
  private cleared = new Set<THREE.Object3D>();
  /** Where the last thrown thing came down, and when. */
  private landing: { at: THREE.Vector3; time: number } | null = null;
  /** Barricades walked into by a token not there yet, left standing until it is. */
  private breaking = new Set<number>();
  /** Traps sprung on a token still walking there, kept on show until it lands. */
  private springing = new Set<number>();
  /** Effects waiting for a seat's token to finish walking. */
  private arrivals: { seat: number; run: () => void; wait: number }[] = [];
  /** Steel jaws snapping shut, then fading. */
  private snaps: { group: THREE.Group; jaws: THREE.Object3D[]; t: number }[] = [];
  /** A pile of dropped coins on a space, until someone walks over it. */
  private spills = new Map<number, THREE.Object3D>();
  /** The other chests, open only in the last round. */
  private spares = new Map<number, THREE.Group>();
  /** The storm, and the marker on the space it moves to next. */
  private storm: THREE.Group | null = null;
  private stormAt = -1;
  private stormParts: {
    /** The height of the space it sits on, which its bob rides above. */
    base: number;
    rain: THREE.LineSegments<THREE.BufferGeometry, THREE.LineBasicMaterial>;
    drops: { x: number; z: number; phase: number }[];
    cloud: THREE.MeshStandardMaterial;
    ring: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
    nextFlash: number;
  } | null = null;
  /** A dashed ring on the space the storm walks to next, so it can be seen coming. */
  private stormNext: THREE.Group | null = null;
  /** The spaces each space leads on to, so the camera can look where a seat is heading. */
  private nextOf = new Map<number, number[]>();
  private pathIndexOf: { spaces: BoardView['spaces']; byId: Map<number, BoardView['spaces'][number]>; incoming: Map<number, number[]> } | null = null;
  /** The space before each one on the lap or its way, for the camera in a tunnel. */
  private prevOf = new Map<number, number>();
  private underground = new Set<number>();
  private stormNextAt = -1;
  private tokens = new Map<number, TokenView>();
  private floaters: Floater[] = [];
  private effects: Effect[] = [];
  private maxHealth = 3;
  private positions = new Map<number, THREE.Vector3>();
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private built = false;
  private lastSeen: string | null = null;
  private lastSequence:number|null=null;
  private logPrimed = false;
  private highlight: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  private branchMarks: THREE.Sprite[] = [];
  private aimMarks = new Map<number, { ring: THREE.Mesh; label: THREE.Sprite }>();
  private aimKey = '';
  private prize: THREE.Group | null = null;
  private prizeAt = -1;
  /** Where the prize hovers, so its bob does not fight the hill under it. */
  private prizeBase = 1.5;
  /** The chest flying from one spot to the next, with the time it has been up. */
  /** The chest on its way to a new spot; `watched` when the camera goes with it. */
  private prizeFlight: { from: THREE.Vector3; to: THREE.Vector3; t: number; sky?: boolean; watched?: boolean } | null = null;
  private prizeDropAt:number|null=null;
  /** Whoever just won the prize, celebrated before the chest moves on from them. */
  private prizeWinner: { seat: number; until: number } | null = null;
  private extent = { cx: 0, cz: 0, halfW: 12, halfD: 9 };
  private spinners: THREE.Object3D[] = [];
  private districtLabels:THREE.Object3D[]=[];
  private closeCamera=false;
  setCloseCamera(close:boolean){this.closeCamera=close;}
  private dice = [new Die(), new Die()];
  private terrain!: Terrain;
  /** How much scatter to place, cut down on weaker machines. */
  private detail = 1;
  private map: MapKey = 'island';
  /** Set up once the board is built, for the dressing and the space markers. */
  private dresser!: Dresser;
  /** Things the dressing moves every frame. */
  private animators: ((time: number) => void)[] = [];
  /** The water over each tide space, and how far in the tide is, from 0 to 1. */
  private tidePools: THREE.Mesh[] = [];
  private tideLevel = 0;
  private tideIn = false;
  /** Gates across ways that are shut until the last rounds. */
  private gates: THREE.Object3D[] = [];
  private shops = new Map<number, Shop>();
  /** The shop someone is standing in, while they choose what to buy. */
  private shopping: number | null = null;
  private gatesOpen = false;
  private tideWater = this.trackMaterial(
    new THREE.MeshStandardMaterial({ color: '#2aa3d8', roughness: 0.15, transparent: true, opacity: 0.2, depthWrite: false }),
  );
  /** The shot the board is asking the camera to take, if any. */
  private cinema: { at: THREE.Vector3; yaw: number; pitch: number; wide: number; tall: number; life: number; max: number; sweep?:boolean;rollSeat?:number } | null = null;

  constructor(scene: THREE.Scene, private options: { detail?: number; city?:boolean } = {}) {
    this.group.add(this.crawler.group,this.traffic.group,this.rental.group,this.candyVan.group);
    this.detail = options.detail ?? 1;
    this.group.name = 'party-board';
    this.bitGeometry = this.track(new THREE.IcosahedronGeometry(0.09, 0));
    this.chunkGeometry = this.track(new THREE.BoxGeometry(0.28, 0.28, 0.28));
    this.bloodMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#b3121f', roughness: 0.35 }));
    scene.add(this.group);
    this.weapons = new BoardWeapons(this.group,(x,z)=>this.groundAt(x,z),(at,target,blocked,item,coins,first,damage)=>{
      if(blocked){this.pulse(at,'#93c5fd');if(first)this.float(at,'Shield stopped it!','#93c5fd');return;}
      if(target===null){this.pulse(at,'#cbd5e1');return;}
      this.pulse(at,item==='slapshot'?'#67e8f9':'#fbbf24');
      this.splatter(at,item==='slapshot'?15:damage>0?10:4);
      if(first)this.float(at,item==='slapshot'?`Slapshot! −${coins} coins`:damage>0?'−1 heart': 'Hit!',item==='slapshot'?'#67e8f9':damage>0?'#fb7185':'#fdba74',damage>0?2.2:.8);
    });
    this.pedestrians=new PedestrianScene(this.group,(x,z)=>this.groundAt(x,z),seat=>this.seatAt(seat));
    this.finale=new MapFinaleScene(this.group,(x,z)=>this.finaleFloor(x,z));
    this.highlight = new THREE.Mesh(
      this.track(new THREE.RingGeometry(0.62, 0.8, 32)),
      this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ffffff', transparent: true, opacity: 0.9, depthWrite: false })),
    );
    this.highlight.rotation.x = -Math.PI / 2;
    this.highlight.visible = false;
    this.group.add(this.highlight);
    for (const die of this.dice) this.group.add(die.group);
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private trackMaterial<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private build(view: BoardView) {
    this.map = view.map in DRESSINGS ? view.map : 'island';
    const design = DESIGNS[this.map];
    this.measure(view, design?.coast);
    // On a hand-built board, the ways laid as bridges and decks.
    const pathOf = (space: { path?: number }) => design?.paths[space.path ?? 0];
    const bridged = new Set(view.spaces.filter((space) => pathOf(space)?.bridge).map((space) => space.id));
    // The land ignores the spaces in tunnels and on bridges, so a hill can
    // rise over a tunnel and water can run under a bridge.
    const surface = view.spaces.filter((space) => !space.under && !bridged.has(space.id));
    this.terrain = new Terrain(this.extent, surface, WATER_Y, view.spaces.length * 7919, this.map, design);
    const island = this.terrain.build();
    this.track(island.geometry);
    this.trackMaterial(island.material as THREE.Material);
    this.group.add(island);
    this.dresser = {
      group: this.group,
      terrain: this.terrain,
      extent: this.extent,
      spaces: view.spaces,
      clearings: [],
      detail: this.detail,
      area: (view.world_scale ?? 1) ** 2,
      shore: view.world_scale ?? 1,
      lava: this.trackMaterial(
        new THREE.MeshStandardMaterial({ color: '#d9480f', emissive: '#ff3300', emissiveIntensity: 1, roughness: 0.45 }),
      ),
      animate: (step) => this.animators.push(step),
      track: (geometry) => this.track(geometry),
      trackMaterial: (material) => this.trackMaterial(material),
    };
    const lava = this.dresser.lava;
    this.animators.push((time) => {
      lava.emissiveIntensity = 0.9 + Math.sin(time * 1.7) * 0.25;
    });

    const tunnels = findTunnels(view.spaces);
    const deep = tunnelHeights(tunnels, view.spaces, this.terrain);
    for (const space of view.spaces) {
      // A bridge is held up at the dry land's level, and well clear of any water.
      const bridgeAt = bridged.has(space.id) ? Math.max(this.terrain.raw(space.x, space.z, false), WATER_Y + 1.3) : null;
      this.positions.set(space.id, new THREE.Vector3(space.x, deep.get(space.id) ?? bridgeAt ?? this.terrain.height(space.x, space.z), space.z));
      this.nextOf.set(space.id, space.next);
      for (const next of space.next) this.prevOf.set(next, space.id);
      if (space.under) this.underground.add(space.id);
    }
    // Paths first, so the tiles sit on top of them.
    const pathMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: PATH_COLORS[this.map], roughness: 1 }));
    const plankMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#8b5e3c', roughness: 0.9 }));
    const deck = design
      ? new Set(view.spaces.filter((space) => pathOf(space)?.deck || bridged.has(space.id)).map((space) => space.id))
      : this.map === 'shipwreck'
        ? onDeck(view.spaces)
        : new Set<number>();
    this.deckSpaces = deck;
    if(!this.options.city){
    this.buildBridges(view, bridged);
    for (const space of view.spaces) {
      const center = this.positions.get(space.id)!;
      for (const nextId of space.next) {
        if (space.link === nextId) continue;
        const toCenter = this.positions.get(nextId)!;
        // Each end tucks just inside a tile's rim at the tile's foot, so a way
        // that climbs or drops never runs over the top of the lower tile.
        const flat = new THREE.Vector3(toCenter.x - center.x, 0, toCenter.z - center.z);
        const inset = Math.min(0.62, flat.length() * 0.45);
        flat.normalize().multiplyScalar(inset);
        const from = center.clone().add(flat);
        const to = toCenter.clone().sub(flat);
        const length = from.distanceTo(to);
        const path = new THREE.Mesh(this.track(new THREE.BoxGeometry(length, 0.08, 0.5)), deck.has(space.id) && deck.has(nextId) ? plankMaterial : pathMaterial);
        path.position.copy(from).add(to).multiplyScalar(0.5);
        path.position.y += 0.05;
        path.rotation.y = Math.atan2(from.z - to.z, to.x - from.x);
        // Ramps where the way climbs, so the path lies on the hill.
        path.rotation.z = Math.asin(THREE.MathUtils.clamp((to.y - from.y) / Math.max(0.001, length), -1, 1));
        path.receiveShadow = true;
        this.group.add(path);
      }
    }

    const tileGeometry = this.track(new THREE.CylinderGeometry(0.72, 0.72, 0.18, 26));
    const glyphGeometry = this.track(new THREE.CircleGeometry(0.62, 32));
    const glyphMaterials = new Map<SpaceKind, THREE.MeshBasicMaterial>();
    const glyphMaterial = (kind: SpaceKind) => {
      let material = glyphMaterials.get(kind);
      if (!material) {
        material = this.trackMaterial(
          new THREE.MeshBasicMaterial({ map: glyphTexture(kind), transparent: true, depthWrite: false }),
        );
        glyphMaterials.set(kind, material);
      }
      return material;
    };
    for (const space of view.spaces) {
      const tile = new THREE.Mesh(
        tileGeometry,
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: SPACE_COLORS[space.kind], roughness: 0.6 })),
      );
      tile.position.copy(this.positions.get(space.id)!);
      tile.position.y += 0.09;
      tile.castShadow = true;
      tile.receiveShadow = true;
      this.group.add(tile);
      this.tiles.set(space.id, tile);
      tile.userData.kind = space.kind;
      tile.userData.space = space.id;
      const glyph = new THREE.Mesh(glyphGeometry, glyphMaterial(space.kind));
      glyph.rotation.x = -Math.PI / 2;
      glyph.position.copy(tile.position);
      glyph.position.y += 0.095;
      glyph.renderOrder = 2;
      this.group.add(glyph);
      this.decorate(space.kind, tile.position, space.id);
      if (space.kind === 'tide') {
        const pool = groundPatch(this.dresser, space.x, space.z, 1.7, this.tideWater, 0);
        pool.userData.base = pool.position.y;
        this.tidePools.push(pool);
      }
      if (space.kind === 'teleport' && space.link !== null) {
        const to = this.positions.get(space.link);
        if (to) {
          const line = new THREE.Mesh(
            this.track(new THREE.TorusGeometry(tile.position.distanceTo(to) / 2, 0.05, 6, 40, Math.PI)),
            this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#22d3ee', transparent: true, opacity: 0.5 })),
          );
          line.position.copy(tile.position).add(to).multiplyScalar(0.5);
          line.position.y += 0.2;
          line.rotation.y = -Math.atan2(to.z - tile.position.z, to.x - tile.position.x);
          this.group.add(line);
        }
      }
    }
    }
    buildTunnels(this.dresser, tunnels, this.positions, this.map);
    if (design) dressDesign(this.dresser, design, this.map);
    else DRESSINGS[this.map](this.dresser);
    this.districtLabels=this.group.children.filter(object=>!!object.userData.districtTexture);
    this.specials = { palms: design?.specials?.palms ?? [], flotsam: design?.specials?.flotsam ?? [], snowmen: design?.specials?.snowmen ?? [] };
    // A tall palm at each coconut palm, so it reads as the one to shake.
    for (const [x, z] of this.specials.palms) {
      void place(this.group, 'tree_palmTall', { x, z, y: this.terrain.height(x, z), scale: 2.2 });
    }
    if(!this.options.city){this.buildGates(view);this.prize = this.buildPrize();this.group.add(this.prize);}
    this.built = true;
  }

  /** Rope and posts along each side of a bridge, down to the ground or the water under it. */
  private buildBridges(view: BoardView, bridged: Set<number>) {
    if (!bridged.size) return;
    const post = this.track(new THREE.CylinderGeometry(0.07, 0.09, 1, 6));
    const wood = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#6b4a2f', roughness: 0.9 }));
    const rope = this.trackMaterial(new THREE.LineBasicMaterial({ color: '#d6c09a' }));
    const sides: [THREE.Vector3[], THREE.Vector3[]] = [[], []];
    const flush = () => {
      for (const side of sides) {
        if (side.length > 1) this.group.add(new THREE.Line(this.track(new THREE.BufferGeometry().setFromPoints(side)), rope));
        side.length = 0;
      }
    };
    // Walked in order along each way, so the rope runs from post to post.
    for (const space of view.spaces) {
      if (!bridged.has(space.id)) {
        flush();
        continue;
      }
      const at = this.positions.get(space.id)!;
      const ahead = this.positions.get(space.next[0]) ?? at;
      const along = new THREE.Vector3(ahead.x - at.x, 0, ahead.z - at.z).normalize();
      const across = new THREE.Vector3(-along.z, 0, along.x).multiplyScalar(0.62);
      [1, -1].forEach((sign, index) => {
        const x = at.x + across.x * sign;
        const z = at.z + across.z * sign;
        const foot = Math.max(this.terrain.height(x, z), WATER_Y - 1);
        const top = at.y + 0.85;
        const mesh = new THREE.Mesh(post, wood);
        mesh.scale.y = Math.max(0.3, top - foot);
        mesh.position.set(x, (top + foot) / 2, z);
        mesh.castShadow = true;
        this.group.add(mesh);
        sides[index].push(new THREE.Vector3(x, top - 0.1, z));
      });
    }
    flush();
  }

  private measure(view: BoardView, coast?: [number, number][]) {
    // A hand-built board reaches out to its coastline, not just its spaces.
    const xs = [...view.spaces.map((space) => space.x), ...(coast ?? []).map((p) => p[0])];
    const zs = [...view.spaces.map((space) => space.z), ...(coast ?? []).map((p) => p[1])];
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minZ = Math.min(...zs);
    const maxZ = Math.max(...zs);
    this.extent = {
      cx: (minX + maxX) / 2,
      cz: (minZ + maxZ) / 2,
      halfW: (maxX - minX) / 2 + 1.5,
      halfD: (maxZ - minZ) / 2 + 1.5,
    };
  }

  /** The height of the island at a point, so the camera never sinks into it. */
  groundAt(x: number, z: number) {
    return this.terrain ? this.terrain.height(x, z) : 0;
  }

  /** Follow the same bridge/deck/tunnel surfaces as board tokens, without
   * changing the terrain lookup used by traffic or pedestrian helpers. */
  private finaleFloor(x:number,z:number){
    let height=this.groundAt(x,z),nearest=2.1;
    const view=this.lastView,design=DESIGNS[this.map];
    for(const space of view?.spaces??[]){
      const path=design?.paths[space.path??0];
      if(!space.under&&!path?.bridge&&!path?.deck)continue;
      const from=this.positions.get(space.id);if(!from)continue;
      for(const id of space.next){const to=this.positions.get(id);if(!to)continue;const dx=to.x-from.x,dz=to.z-from.z;const t=THREE.MathUtils.clamp(((x-from.x)*dx+(z-from.z)*dz)/Math.max(.001,dx*dx+dz*dz),0,1);const distance=Math.hypot(x-from.x-dx*t,z-from.z-dz*t);if(distance<nearest){nearest=distance;height=THREE.MathUtils.lerp(from.y,to.y,t);}}
    }
    return height;
  }

  /** The HUD/camera's viewer determines which physical gate is highlighted. */
  setFinaleViewer(seat:number|null){this.finale.setViewer(seat);}

  /** How much ground the board covers, so the camera knows what to frame. */
  bounds() {
    return { center: new THREE.Vector3(this.extent.cx, 0, this.extent.cz), halfW: this.extent.halfW, halfD: this.extent.halfD };
  }

  /** A gate across each way onto a gated path, standing until the gates open. */
  private buildGates(view: BoardView) {
    for (const space of view.spaces) {
      for (const way of space.next) {
        // Roads can now be entered at either end. Mark both physical
        // entrances, including the old one-way route's exit.
        if (!!space.gated === !!view.spaces[way]?.gated) continue;
        const from = this.positions.get(space.id)!;
        const to = this.positions.get(way)!;
        const at = from.clone().lerp(to, 0.5);
        const yaw = Math.atan2(to.x - from.x, to.z - from.z);
        void place(this.group, 'gate', { x: at.x, z: at.z, y: this.terrain.height(at.x, at.z), scale: 2, yaw }).then((gate) => {
          if (!gate) return;
          gate.visible = !this.gatesOpen;
          this.gates.push(gate);
        });
      }
    }
  }

  /** A marker beside each space, from the kits, so it reads at a glance. */
  private decorate(kind: SpaceKind, at: THREE.Vector3, space: number) {
    const { cx, cz } = this.extent;
    // Markers that stand sit off the path, on the side away from the middle.
    const away = new THREE.Vector3(at.x - cx, 0, at.z - cz).normalize().multiplyScalar(1.15);
    const besideX = at.x + away.x;
    const besideZ = at.z + away.z;
    // Down in a tunnel, the floor is the tile's own level, not the land overhead.
    const beside = { x: besideX, z: besideZ, y: this.underground.has(space) ? at.y - 0.09 : this.terrain.height(besideX, besideZ) };
    const ground = at.y - 0.09;
    const facing = Math.atan2(-away.x, -away.z);
    const spin = (object: THREE.Object3D | null) => {
      if (object) this.spinners.push(object);
      return object;
    };
    const pickup = (object: THREE.Object3D | null) => {
      if (!object) return;
      this.pickups.push(object);
      object.visible = !this.clear;
    };
    switch (kind) {
      // Coin spaces point out the floating field; their tiles have no props.
      case 'coin':
        return;
      case 'loss':
        void place(this.group, 'coin-bronze', { x: at.x, z: at.z, y: ground + 0.5, scale: 1.4 }).then(spin).then(pickup);
        return;
      case 'item':
        void place(this.group, 'crate-item', { ...beside, scale: 1.1, yaw: facing }).then((box) => {
          pickup(box);
          if (box) this.specialModels.set(pullKey(PULL.box, space), box);
        });
        return;
      case 'event':
        void place(this.group, 'banner-red', { ...beside, scale: 1.6, yaw: facing });
        return;
      case 'shop': {
        // A proper stall set back from the path, with someone minding it.
        const back = 0.85;
        const sx = beside.x + away.x * back;
        const sz = beside.z + away.z * back;
        const stall = new THREE.Vector3(sx, this.terrain.height(sx, sz), sz);
        void place(this.group, 'stall-red', { x: stall.x, z: stall.z, y: stall.y, scale: 1.7, yaw: facing });
        void place(this.group, 'lantern', { x: stall.x - away.z * 0.95, z: stall.z + away.x * 0.95, y: stall.y, scale: 0.9 });
        const keeper = buildCharacter(SHOPKEEPER);
        keeper.group.scale.setScalar(0.8);
        // A step aside from the stall's front post, so it doesn't hide their face.
        keeper.group.position.set(stall.x + away.x * 0.15 + away.z * 0.32, stall.y, stall.z + away.z * 0.15 - away.x * 0.32);
        keeper.group.rotation.y = facing;
        this.group.add(keeper.group);
        this.shops.set(space, { at: stall, facing, keeper });
        // Nothing grows in the shop, or between it and the camera looking in.
        this.dresser.clearings.push({ x: stall.x, z: stall.z, r: 2.4 });
        for (const step of [2.5, 5, 7.5]) {
          this.dresser.clearings.push({ x: stall.x - away.x * step, z: stall.z - away.z * step, r: 1.6 });
        }
        return;
      }
      case 'hazard':
        void place(this.group, 'trap-spikes', { x: at.x, z: at.z, y: ground + 0.2, scale: 1.4 });
        if (this.map === 'volcano') {
          // A vent spitting lava beside the way, ringed with cooled rock.
          lavaPool(this.dresser, beside.x, beside.z, 0.55);
          void place(this.group, 'stone_smallG', { ...beside, x: beside.x + away.x * 0.6, z: beside.z + away.z * 0.6, scale: 1.8, paint: { stone: '#2f2826' } });
        } else {
          void place(this.group, 'cannon', { ...beside, scale: 0.7, yaw: facing + Math.PI });
        }
        return;
      case 'ice':
        iceSheet(this.dresser, at.x, at.z, 1.15);
        return;
      case 'teleport':
        void place(this.group, 'spring', { x: at.x, z: at.z, y: ground + 0.2, scale: 1.1 });
        return;
      case 'cannon':
        void place(this.group, 'cannon', { ...beside, scale: 1.1, yaw: facing + Math.PI });
        return;
      case 'start':
        void place(this.group, 'flag-pirate-high', { ...beside, scale: 0.6, yaw: facing });
        return;
      default:
        return;
    }
  }

  /** The storm: a small cloud raining on whichever space it is sitting on,
   * with a ring on the ground so the space it holds reads from above. */
  private buildStorm() {
    const storm = new THREE.Group();
    const cloud = this.trackMaterial(
      new THREE.MeshStandardMaterial({
        // Solid, not see-through: a see-through cloud is sorted against every
        // other see-through thing on the board and could drop out for a moment.
        color: '#4b5563',
        roughness: 1,
        flatShading: true,
        emissive: '#cbd5e1',
        emissiveIntensity: 0,
      }),
    );
    const puff = this.track(new THREE.IcosahedronGeometry(1, 1));
    // Puffs around a taller middle, kept narrow so the spaces below still show.
    for (const [x, y, z, r] of [
      [0, 0.12, 0, 0.5],
      [0.42, 0, 0.1, 0.36],
      [-0.4, 0, -0.08, 0.38],
      [0.08, -0.02, 0.4, 0.34],
      [-0.06, 0.02, -0.4, 0.33],
    ]) {
      const piece = new THREE.Mesh(puff, cloud);
      piece.position.set(x, 2.3 + y, z);
      piece.scale.set(r, r * 0.75, r);
      piece.frustumCulled = false;
      storm.add(piece);
    }

    // Short streaks that fall from the cloud to the space and start again.
    const drops = Array.from({ length: 22 }, (_, i) => {
      const angle = i * 2.4;
      const spread = 0.15 + (i % 5) * 0.12;
      return { x: Math.cos(angle) * spread, z: Math.sin(angle) * spread, phase: (i * 0.37) % 1 };
    });
    const rainGeometry = this.track(new THREE.BufferGeometry());
    rainGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(drops.length * 6), 3));
    const rain = new THREE.LineSegments(
      rainGeometry,
      this.trackMaterial(new THREE.LineBasicMaterial({ color: '#bfdbfe', transparent: true, opacity: 0.7 })),
    );
    rain.frustumCulled = false;
    storm.add(rain);

    const ring = new THREE.Mesh(
      this.track(new THREE.RingGeometry(0.84, 1, 40)),
      this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#94a3b8', transparent: true, opacity: 0.8, depthWrite: false })),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.13;
    storm.add(ring);

    this.stormParts = { base: 0, rain, drops, cloud, ring, nextFlash: 2 };
    return storm;
  }

  /** The dashes that mark where the storm goes next. */
  private buildStormNext() {
    const next = new THREE.Group();
    const material = this.trackMaterial(
      new THREE.MeshBasicMaterial({ color: '#94a3b8', transparent: true, opacity: 0.45, depthWrite: false }),
    );
    const dashes = 8;
    for (let i = 0; i < dashes; i++) {
      const dash = new THREE.Mesh(
        this.track(new THREE.RingGeometry(0.86, 0.98, 6, 1, (i / dashes) * Math.PI * 2, (Math.PI * 2) / dashes / 2)),
        material,
      );
      dash.rotation.x = -Math.PI / 2;
      next.add(dash);
    }
    return next;
  }

  /** One real CC0 candy bar, with a small halo instead of a chest/jewel stack. */
  private buildPrize() {
    const prize=new THREE.Group();prize.name='candy-objective';
    void place(prize,'objective-candy',{x:0,z:0}).then(model=>{
      if(!model)return;
      // The objective may already have moved while its GLB was fetching.
      // Normalize the detached source in local space, never around world x/z.
      model.removeFromParent();model.position.set(0,0,0);model.updateMatrixWorld(true);
      const box=new THREE.Box3().setFromObject(model),size=box.getSize(new THREE.Vector3());
      const scale=2/Math.max(.001,size.x,size.z);
      model.scale.setScalar(scale);
      model.position.sub(box.getCenter(new THREE.Vector3()).multiplyScalar(scale));
      model.rotation.z=.1;prize.add(model);
      this.dresser.animate(time=>{model.rotation.y=time*.65;});
    });
    const glow=new THREE.Mesh(
      this.track(new THREE.TorusGeometry(1.18,.035,8,36)),
      this.trackMaterial(new THREE.MeshBasicMaterial({color:'#a7f3d0',transparent:true,opacity:.6,depthWrite:false})),
    );
    glow.rotation.x=-Math.PI/2;glow.position.y=-.3;prize.add(glow);
    return prize;
  }

  sync(view: BoardView, members: Member[], roster: Map<string, CharacterDef>) {
    const previousWalk=this.lastView?.walk;
    const previousStep=this.lastView?.turn.step;
    const previousFinale=this.lastView?.finale;
    this.lastView = view;
    if(view.finale&&!previousFinale){this.stormFlights.clear();this.spikeHits.clear();this.cinema=null;}
    if (view.walk && (view.walk.id !== this.walkClock?.id || view.walk.elapsed !== this.walkClock?.elapsed)) {
      this.walkClock = {id:view.walk.id,elapsed:view.walk.elapsed,at:performance.now()};
    }
    if (!view.walk) this.walkClock = null;
    const newAction=view.walk?previousWalk?.id!==view.walk.id:previousStep!==view.turn.step&&['vacuum','lob','swing','check'].includes(view.turn.step);
    if(newAction && (this.stormFlights.has(view.turn_seat)||this.spikeHits.has(view.turn_seat))) {
      const token=this.tokens.get(view.turn_seat);
      const state=view.tokens.find(token=>token.seat===view.turn_seat);
      if(token&&state){const at=this.spot(state.space,state.seat,view);token.from=at.clone();token.to=at.clone();token.hop=1;token.space=state.space;token.rig.group.position.copy(at);token.rig.group.rotation.z=0;}
      this.stormFlights.delete(view.turn_seat);this.spikeHits.delete(view.turn_seat);this.cinema=null;
    }
    if (!this.built) this.build(view);
    if(this.options.city){this.traffic.sync(view.traffic,(x,z)=>this.groundAt(x,z));this.pedestrians.sync(view.pedestrians??[]);this.crawler.group.visible=false;this.rental.group.visible=false;this.candyVan.group.visible=false;return;}
    this.crawler.sync(this.options.city?null:view.crawler,(x,z)=>this.groundAt(x,z));
    this.traffic.sync(view.traffic,(x,z)=>this.groundAt(x,z));
    this.rental.sync(view.driving,(x,z)=>this.groundAt(x,z));
    this.candyVan.sync(view.candy_van,(x,z)=>this.groundAt(x,z));

    for (const [seat, token] of this.tokens) {
      const member = members.find((m) => m.seat === seat);
      const def = member && roster.get(member.character);
      if (!view.tokens.some((t) => t.seat === seat) || !member || !def || characterAppearanceKey(def) !== token.appearanceKey) {
        token.rig.dispose();
        this.disposeTag(token);
        this.tokens.delete(seat);
        const bag = this.bags.get(seat);
        bag?.strap.removeFromParent();
        bag?.built.dispose();
        this.bags.delete(seat);
      }
    }
    for (const token of view.tokens) {
      if (this.tokens.has(token.seat)) continue;
      const member = members.find((m) => m.seat === token.seat);
      const def = member && roster.get(member.character);
      if (!def) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.85);
      this.group.add(rig.group);
      const at = this.spot(token.space, token.seat, view);
      rig.group.position.copy(at);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthTest: false }));
      tag.renderOrder = 25;
      this.group.add(tag);
      this.tokens.set(token.seat, {
        seat: token.seat,
        characterId: def.id,
        appearanceKey: characterAppearanceKey(def),
        rig,
        bagStyle: def.costume?.bag ?? null,
        bagTint: (def.costume?.bag && def.costume.tints?.bag) || null,
        space: token.space,
        from: at,
        to: at,
        hop: 1,
        yaw: 0,
        tag,
        tagKey: '',
        shake: 0,
        path: [],
        wait: 0,
        flying: false,
        colors: [def.body, def.accent, def.belly ?? def.body],
        burst: 0,
        positioning: null,
      });
    }

    // Free positioning is a visual offset; the real tile, purse, health and
    // path stay authoritative. Keep its last rendered location separately
    // from the ordinary token hop, which still targets the home space.
    for (const token of this.tokens.values()) {
      const raid=view.raids?.find(raid=>raid.seat===token.seat);
      const visit=view.candy_runs?.find(run=>run.seat===token.seat);
      const position = visit?{...visit,active:false}:view.positioning?.seat===token.seat ? view.positioning : raid ? {...raid,active:false} : null;
      if (position?.seat === token.seat) {
        const target = new THREE.Vector3(position.x, this.groundAt(position.x, position.z) + 0.1, position.z);
        if (!token.positioning || token.positioning.returning) {
          token.positioning = {at:token.rig.group.position.clone(),target,from:token.rig.group.position.clone(),returning:false,elapsed:0,active:position.active};
        } else { token.positioning.target.copy(target); token.positioning.active = position.active; }
      } else if (token.positioning && !token.positioning.returning) {
        const state = view.tokens.find(state => state.seat === token.seat);
        if (!state) { token.positioning = null; continue; }
        token.positioning.from.copy(token.positioning.at);
        token.positioning.target.copy(this.spot(state.space, state.seat, view));
        token.positioning.returning = true;
        token.positioning.elapsed = 0;
      }
    }

    this.pedestrians.sync(this.options.city?[]:view.pedestrians??[]);
    this.finale.sync(view.finale??null);
    this.maxHealth = Math.max(this.maxHealth, ...view.tokens.map((t) => t.health));
    for (const token of view.tokens) {
      const view3d = this.tokens.get(token.seat);
      if (!view3d) continue;
      this.bagOf(view3d).held = Math.min(token.items.length, BAG_SIZES.length - 1);
      const key = tagKey(token);
      if (key === view3d.tagKey) continue;
      view3d.tagKey = key;
      view3d.tag.material.map?.dispose();
      view3d.tag.material.map = statusTexture(token, this.maxHealth);
      view3d.tag.material.needsUpdate = true;
      view3d.tag.scale.set(3.36, 0.72, 1);
      view3d.tag.userData.token = token;
    }

    // The log carries rolls and the spaces walked, so read it before moving anyone.
    this.prizeDropAt=null;
    this.readLog(view, members);
    if (view.finale || view.turn.step !== 'shop') this.shopping = null;
    else if (this.shopping === null) {
      // Opened mid-turn: the log that put them in the shop has gone by.
      const here = view.tokens.find((t) => t.seat === view.turn_seat)?.space;
      if (here !== undefined && this.shops.has(here)) this.shopping = here;
    }

    // Anything else that moved a token (a knock back, a teleport) is one hop,
    // unless what moves it is still in the air.
    for (const token of view.tokens) {
      const view3d = this.tokens.get(token.seat);
      if (view3d && previousWalk?.seat===token.seat && !view.walk && !this.stormFlights.has(token.seat) && !this.spikeHits.has(token.seat)) {
        view3d.to=this.spot(token.space,token.seat,view);
        view3d.from.copy(view3d.to);view3d.hop=1;
      }
      if (!view3d || view3d.space === token.space || this.stayPut.has(token.seat)) continue;
      this.hopTo(view3d, token.space, view);
    }

    for (const [space, mark] of this.trapMarks) {
      if (!view.traps.some((trap) => trap.space === space) && !this.springing.has(space)) {
        mark.removeFromParent();
        this.trapMarks.delete(space);
      }
    }
    for (const trap of view.traps) {
      const space = trap.space;
      if (this.trapMarks.has(space)) continue;
      const mark = new THREE.Mesh(
        this.track(new THREE.TorusGeometry(0.42, 0.09, 8, 18)),
        this.trackMaterial(
          new THREE.MeshStandardMaterial({ color: trap.mine ? '#a78bfa' : '#6b7280', roughness: 0.6 }),
        ),
      );
      mark.rotation.x = -Math.PI / 2;
      mark.position.copy(this.positions.get(space) ?? new THREE.Vector3());
      mark.position.y += 0.14;
      this.group.add(mark);
      this.trapMarks.set(space, mark);
    }

    if (view.storm !== null && !this.storm) {
      this.storm = this.buildStorm();
      this.group.add(this.storm);
    }
    if (this.storm) {
      this.storm.visible = view.storm !== null;
      if (view.storm !== null && view.storm !== this.stormAt) {
        this.stormAt = view.storm;
        const at = this.positions.get(view.storm);
        if (at) {
          this.storm.position.copy(at);
          if (this.stormParts) this.stormParts.base = at.y;
        }
      }
    }
    const nextAt = view.storm !== null && view.storm_to !== view.storm ? view.storm_to : null;
    if (nextAt !== null && !this.stormNext) {
      this.stormNext = this.buildStormNext();
      this.group.add(this.stormNext);
    }
    if (this.stormNext) {
      this.stormNext.visible = nextAt !== null;
      if (nextAt !== null && nextAt !== this.stormNextAt) {
        this.stormNextAt = nextAt;
        const at = this.positions.get(nextAt);
        if (at) this.stormNext.position.set(at.x, at.y + 0.12, at.z);
      }
    }

    const extra = view.open_prizes.filter((space) => space !== view.prize_at);
    for (const [space, chest] of this.spares) {
      if (extra.includes(space)) continue;
      chest.removeFromParent();
      this.spares.delete(space);
    }
    for (const space of extra) {
      if (this.spares.has(space)) continue;
      const at = this.positions.get(space);
      if (!at) continue;
      const chest = this.buildPrize();
      chest.position.copy(at).setY(at.y + 1.5);
      this.group.add(chest);
      this.spares.set(space, chest);
    }

    this.syncCrates(view);
    this.syncSpecials(view);

    const dropped = new Set(view.loose.map((pile) => pile.space));
    for (const [space, pile] of this.spills) {
      if (dropped.has(space)) continue;
      pile.removeFromParent();
      this.spills.delete(space);
    }
    for (const pile of view.loose) {
      if (this.spills.has(pile.space)) continue;
      const at = this.positions.get(pile.space);
      if (!at) continue;
      const group = new THREE.Group();
      group.position.copy(at);
      this.group.add(group);
      this.spills.set(pile.space, group);
      // One coin per two dropped, up to a small heap, so the size reads.
      const coins = Math.max(1, Math.min(4, Math.round(pile.coins / 2)));
      for (let i = 0; i < coins; i++) {
        const angle = (i / coins) * Math.PI * 2;
        void place(group, 'coin-gold', {
          x: Math.cos(angle) * 0.3,
          z: Math.sin(angle) * 0.3,
          y: 0.45 + i * 0.05,
          scale: 1.1,
        }).then((coin) => coin && this.spinners.push(coin));
      }
    }

    if (this.prize && view.prize_at !== this.prizeAt) {
      const first = this.prizeAt < 0;
      this.prizeAt = view.prize_at;
      const at = this.positions.get(view.prize_at);
      if(at && this.prizeDropAt===view.prize_at) {
        const to=at.clone().setY(at.y+1.5);
        this.prizeFlight={from:to.clone().add(new THREE.Vector3(0,11,0)),to,t:0,sky:true};
        this.prize.position.copy(this.prizeFlight.from);
        this.prizeBase=to.y;
        this.watch(at,{yaw:this.angleBetween(at,this.bounds().center),pitch:36,wide:12,life:1.75});
        if(this.cinema)this.cinema.sweep=true;
      } else if (at && first) {
        this.prize.position.copy(at).setY(at.y + 1.5);
        this.prizeBase = at.y + 1.5;
      }
      else if (at) {
        this.prizeFlight = { from: this.prize.position.clone(), to: at.clone().setY(at.y + 1.5), t: 0 };
        this.prizeBase = at.y + 1.5;
        // Said once it gets there, after any win has been celebrated.
        if (!this.prizeWinner) this.float(at, 'The prize moved here!', '#f9a8d4');
      }
    }

    this.tideIn = view.tide === true;
    this.gatesOpen = view.gates_open === true;
    for (const gate of this.gates) gate.visible = !this.gatesOpen;
    this.markBranch(view.turn.step === 'branch' ? view.turn.options.map((option) => option.node) : []);

    const active = this.positions.get(view.tokens.find((t) => t.seat === view.turn_seat)?.space ?? 0);
    this.highlight.visible = !!active;
    if (active) this.highlight.position.copy(active).setY(active.y + 0.13);
  }

  /** Rings and distances over rivals while an item is being aimed. */
  setAim(marks: { seat: number; spaces: number; reachable: boolean }[]) {
    const key = marks.map((mark) => `${mark.seat}:${mark.spaces}:${mark.reachable}`).join('|');
    if (key === this.aimKey) return;
    this.aimKey = key;
    for (const [seat, mark] of this.aimMarks) {
      mark.ring.removeFromParent();
      (mark.ring.geometry as THREE.BufferGeometry).dispose();
      (mark.ring.material as THREE.Material).dispose();
      mark.label.material.map?.dispose();
      mark.label.material.dispose();
      mark.label.removeFromParent();
      this.aimMarks.delete(seat);
    }
    for (const mark of marks) {
      const color = mark.reachable ? '#ffd23f' : '#64748b';
      const ring = new THREE.Mesh(
        new THREE.RingGeometry(0.66, 0.92, 28),
        new THREE.MeshBasicMaterial({ color, transparent: true, opacity: mark.reachable ? 0.95 : 0.5, depthWrite: false }),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.renderOrder = 20;
      const texture = labelTexture(`${mark.spaces}`, color);
      const label = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      label.scale.set((image.width / image.height) * 0.75, 0.75, 1);
      label.renderOrder = 32;
      this.group.add(ring, label);
      this.aimMarks.set(mark.seat, { ring, label });
    }
    this.placeAim();
  }

  private placeAim() {
    for (const [seat, mark] of this.aimMarks) {
      const at = this.tokens.get(seat)?.rig.group.position;
      if (!at) continue;
      mark.ring.position.set(at.x, at.y + 0.06, at.z);
      mark.label.position.set(at.x, at.y + 2.3, at.z);
    }
  }

  /** Letters over the tiles a junction leads to, matching the buttons. */
  private markBranch(nodes: number[]) {
    if (this.branchMarks.length === nodes.length && nodes.every((node, i) => this.branchMarks[i].userData.node === node)) {
      return;
    }
    for (const mark of this.branchMarks) {
      mark.material.map?.dispose();
      mark.material.dispose();
      mark.removeFromParent();
    }
    this.branchMarks = nodes.map((node, index) => {
      const texture = labelTexture(['A', 'B', 'C'][index] ?? '?', '#ffffff');
      const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      sprite.scale.set((image.width / image.height) * 0.9, 0.9, 1);
      sprite.position.copy(this.positions.get(node) ?? new THREE.Vector3()).add(new THREE.Vector3(0, 1.5, 0));
      sprite.userData.base = sprite.position.y;
      sprite.renderOrder = 31;
      sprite.userData.node = node;
      this.group.add(sprite);
      return sprite;
    });
  }

  /** Players share a tile, so fan them out around its centre. */
  private spot(space: number, seat: number, view: BoardView) {
    const base = this.positions.get(space) ?? new THREE.Vector3();
    const sharing = view.tokens.filter((t) => t.space === space).map((t) => t.seat);
    const index = Math.max(0, sharing.indexOf(seat));
    const angle = (Math.PI * 2 * index) / Math.max(1, sharing.length) + Math.PI / 4;
    // Spread round the space, wider the more there are, so each one reads on its own.
    const spread = sharing.length > 1 ? 0.5 + sharing.length * 0.12 : 0;
    return new THREE.Vector3(base.x + Math.cos(angle) * spread, base.y + 0.1, base.z + Math.sin(angle) * spread);
  }

  private readLog(view: BoardView, members: Member[]) {
    // The log is capped and drops old lines, so find where we left off.
    const stamped = view.log.map((entry) => JSON.stringify(entry));
    const from = !this.logPrimed ? stamped.length : this.lastSeen === null ? 0 : stamped.lastIndexOf(this.lastSeen) + 1;
    const sequence=Math.max(0,...view.log.map(entry=>typeof entry.seq==='number'?entry.seq:0));
    const newBoard=sequence>0 && this.lastSequence!==null && sequence<this.lastSequence;
    const fresh = sequence>0
      ? !this.logPrimed || newBoard ? [] : view.log.filter(entry=>typeof entry.seq==='number' && entry.seq>(this.lastSequence??0))
      : view.log.slice(from === 0 && this.lastSeen !== null ? 0 : from);
    if(newBoard){this.stormFlights.clear();this.spikeHits.clear();this.cinema=null;}
    if(sequence>0)this.lastSequence=sequence;
    this.logPrimed = true;
    this.lastSeen = stamped[stamped.length - 1] ?? this.lastSeen;
    const knockedOut=new Set(fresh.filter(entry=>entry.kind==='knockout').map(entry=>entry.seat));
    // A knockout says it all itself: no heart count or dropped-coins line on top of it.
    this.knockedNow=knockedOut;
    for (const entry of fresh) {
      if(entry.kind==='prize' && typeof entry.moved_to==='number')this.prizeDropAt=entry.moved_to;
      if((entry.kind==='event' && entry.event==='prize_moves' || entry.kind==='chest_blown') && typeof entry.to==='number')this.prizeDropAt=entry.to;
      if(entry.kind==='round' && entry.round===1)this.prizeDropAt=view.prize_at;
      if(entry.kind==='space' && entry.effect==='hazard' && typeof entry.seat==='number' && !this.replaying && !knockedOut.has(entry.seat)) {
        this.spikeImpact(entry);
        continue;
      }
      const spike=entry.kind==='hurt'?this.spikeHits.get(entry.seat):null;
      if(spike && !spike.impact){spike.hurt.push(entry);continue;}
      if(entry.kind==='traffic_hit' && typeof entry.x==='number' && typeof entry.z==='number'){
        const at=new THREE.Vector3(entry.x,this.groundAt(entry.x,entry.z),entry.z);
        this.traffic.impact(Number(entry.car));if(Number(entry.car)===255||this.rental.group.userData.seat===entry.seat)this.rental.impact();
        this.pulse(at,'#fb923c');this.float(at,'Traffic collision · knocked out','#fb7185',2.5);
        if(entry.seat===view.turn_seat){this.watch(at,{yaw:this.tokens.get(entry.seat)?.yaw??0,pitch:26,wide:8,life:1.4});if(this.cinema)this.cinema.sweep=true;}
      }
      if(entry.kind==='npc_run_over' && typeof entry.x==='number' && typeof entry.z==='number'){
        const at=new THREE.Vector3(entry.x,this.groundAt(entry.x,entry.z)+.08,entry.z);
        this.splatter(at,20);this.pulse(at,'#fb7185');
        continue;
      }
      if(entry.kind==='npc_robbed' && typeof entry.npc==='number' && typeof entry.seat==='number'){
        this.pedestrians.robbed(entry.npc,entry.seat,Number(entry.coins)||0);
        continue;
      }
      if (entry.kind === 'roll' && typeof entry.seat === 'number') {
        const token = this.tokens.get(entry.seat);
        if (token) {
          this.watch(token.rig.group.position, {
            yaw: this.angleBetween(token.rig.group.position, this.bounds().center),
            pitch: 32,
            wide: 8.5,
            life: DIE_SECONDS,
            rollSeat:entry.seat,
          });
          const faces: number[] = Array.isArray(entry.dice) ? entry.dice : [Number(entry.roll)];
          const gold = entry.golden === true;
          faces.slice(0, this.dice.length).forEach((face, index) => {
            const side = faces.length > 1 ? index * 2 - 1 : 0;
            this.dice[index].roll(face, token.rig.group.position, side, gold);
          });
          if (entry.cursed) this.float(token.rig.group.position, 'Cursed roll', '#c084fc');
          if (typeof entry.handicap === 'number') this.float(token.rig.group.position, `Catch-up +${entry.handicap}`, '#7dd3fc');
          token.wait = DIE_SECONDS;
          this.sounds?.dice(faces.length);
        }
        continue;
      }
      if (entry.kind === 'cannon' && typeof entry.seat === 'number' && Array.isArray(entry.path)) {
        // Fired rather than walked: one long arc to wherever it came down.
        const token = this.tokens.get(entry.seat);
        const to = this.positions.get(entry.to as number);
        if (token && to) {
          token.path = [to.clone()];
          token.space = entry.to as number;
          token.flying = true;
          this.watch(token.rig.group.position, {
            yaw: this.angleBetween(token.rig.group.position, this.bounds().center),
            pitch: 28,
            wide: 14,
            life: 2,
          });
        }
        continue;
      }
            if (entry.kind === 'move' && typeof entry.seat === 'number' && Array.isArray(entry.path)) {
        const token = this.tokens.get(entry.seat);
        if (token) {
          for (const space of entry.path as number[]) token.path.push(this.spot(space, entry.seat, view));
          token.space = (entry.path as number[])[(entry.path as number[]).length - 1] ?? token.space;
        }
        continue;
      }
      if (entry.kind === 'dev_teleport' && typeof entry.seat === 'number') {
        // The sandbox's jump: straight there, no walking.
        const token = this.tokens.get(entry.seat);
        if (token) {
          this.stormFlights.delete(entry.seat);this.spikeHits.delete(entry.seat);token.rig.group.rotation.z=0;
          const at = this.spot(entry.space, entry.seat, view);
          token.path = [];
          token.space = entry.space;
          token.hop = 1;
          token.from = at;
          token.to = at;
          token.rig.group.position.copy(at);
          this.pulse(at, '#e879f9');
        }
        continue;
      }
      if (entry.kind === 'tide' && entry.flooded) this.sounds?.splash();
      this.hear(entry);
      if (entry.kind === 'tide' && entry.flooded && Array.isArray(entry.caught)) {
        for (const seat of entry.caught as number[]) {
          const at = this.tokens.get(seat)?.rig.group.position;
          if (at) this.float(at, `Tide! -${entry.coins}`, '#7dd3fc');
        }
        continue;
      }
      if (entry.kind === 'item' && (entry.item === 'drive_by' || entry.item === 'slapshot') && Array.isArray(entry.shots)) {
        const origin=this.castAt(entry);
        if(origin)this.weapons.fire(entry,origin);
        continue;
      }
      if (entry.kind === 'raid_hit') {
        const at=new THREE.Vector3(Number(entry.target_x),0,Number(entry.target_z));
        if(Number.isFinite(at.x)&&Number.isFinite(at.z)){
          at.y=this.groundAt(at.x,at.z)+.1;
          this.pulse(at,entry.quality==='blocked'?'#93c5fd':'#38bdf8');
          if(entry.quality==='hit')this.splatter(at,10);
        }
        // Off-turn stick clashes never take the active player's camera.
        continue;
      }
      if (entry.kind === 'item' && typeof entry.flight === 'number' && entry.item !== 'bomb') {
        // Thrown: what it does waits until it comes down.
        this.throwThing(entry, members);
        continue;
      }
      if (entry.kind === 'item' && entry.item === 'bat' && typeof entry.yaw === 'number') this.swoosh(entry);
      if (entry.kind === 'item' && typeof entry.flight !== 'number' && entry.item !== 'bat' && entry.item !== 'bomb') {
        // How a skill check went, over whoever took it.
        const graded: Record<string, [string, string]> = {
          perfect: ['Perfect!', '#fde047'],
          good: ['Good!', '#fdba74'],
          weak: ['Weak…', '#cbd5e1'],
          fail: ['Fumbled!', '#f87171'],
          alarm: ['The alarm went off!', '#f87171'],
        };
        const said = graded[entry.quality as string];
        const at = this.seatAt(entry.by);
        if (said && at) this.float(at.clone().add(new THREE.Vector3(0, 0.6, 0)), said[0], said[1]);
      }
      if (entry.kind === 'item' && entry.quality === 'dropped') {
        const at = this.seatAt(entry.by);
        if (at) this.float(at, 'Dropped it!', '#cbd5e1');
        continue;
      }
      this.playEffect(entry, members);
      if(entry.kind==='blown')continue;
      const seat = typeof entry.seat === 'number' ? entry.seat : typeof entry.target === 'number' ? entry.target : null;
      const token = seat === null ? undefined : this.tokens.get(seat);
      const at = token?.rig.group.position;
      if (!at || !token) continue;
      // What a space does is said once they are standing on it, not as they set off.
      const landing = ['space', 'slide', 'toll_road', 'pickup', 'revenge', 'spill'].includes(String(entry.kind));
      if (landing && (token.path.length > 0 || token.hop < 1 || token.wait > 0)) {
        const later = { ...entry };
        this.arrivals.push({ seat: token.seat, wait: 10, run: () => this.sayOver(later, at) });
        continue;
      }
      this.sayOver(entry, at);
    }
  }

  /** The words over a token for a line of the log, if it has any. */
  private sayOver(entry: BoardLogEntry, at: THREE.Vector3) {
    if (entry.kind === 'space' && entry.effect === 'shop') this.float(at, 'Shop!', '#ffa62b');
    else if (entry.kind === 'space' && entry.effect === 'coins') this.float(at, `+${entry.coins}`, '#ffd23f');
    else if (entry.kind === 'space' && entry.effect === 'loss') this.float(at, `-${entry.coins}`, '#ff6b6b');
    else if (entry.kind === 'space' && entry.effect === 'soaked') this.float(at, `Soaked! -${entry.coins}`, '#7dd3fc');
    else if (entry.kind === 'space' && entry.effect === 'washed_up') this.float(at, `Washed up +${entry.coins}`, '#ffd23f');
    else if (entry.kind === 'slide') this.float(at, 'Slid!', '#bae6fd');
    else if (entry.kind === 'space' && entry.effect === 'toll') this.float(at, `+${entry.coins}`, '#ffd23f');
    else if (entry.kind === 'toll_road' && entry.coins) this.float(at, `Toll -${entry.coins}`, '#ff6b6b');
    else if (entry.kind === 'space' && entry.effect === 'item') this.bagItem(entry.seat, entry.item, at.clone().add(new THREE.Vector3(0, 1.6, 0)));
    else if (entry.kind === 'trap') this.springTrap(entry);
    else if (entry.kind === 'knockout') this.knockOut(entry, at);
    else if (entry.kind === 'item') this.float(at, `${itemLabel(entry.item)}!`, ITEMS[entry.item as Item]?.color ?? '#ffffff');
    else if (entry.kind === 'blocked') this.float(at, 'Blocked!', '#93c5fd');
    else if (entry.kind === 'minigame' && Number(entry.coins) > 0) this.float(at, `+${entry.coins}`, '#ffd23f');
    else if (entry.kind === 'pickup') this.float(at, `Found ${entry.coins}!`, '#ffd23f');
    else if (entry.kind === 'revenge') this.float(at, `Revenge! +${entry.coins}`, '#fb7185');
    else if (entry.kind === 'taunt') this.float(at, String(entry.label ?? ''), '#ffffff');
    else if (entry.kind === 'bet' && entry.coins) this.float(at, `Staked ${entry.coins}`, '#fbbf24');
    else if (entry.kind === 'blown') this.float(at, `Blown back! -${entry.coins}`, '#94a3b8');
    else if (entry.kind === 'spill' && !this.knockedNow.has(entry.seat)) this.float(at, 'Dropped their coins!', '#fbbf24');
  }

  private seatAt(seat: unknown) {
    return typeof seat === 'number' ? this.tokens.get(seat)?.rig.group.position ?? null : null;
  }

  /** The item can resolve in the same packet that sends its owner home. The
   * authored server cast point keeps its flight/swing at the actual origin. */
  private castAt(entry: BoardLogEntry) {
    if (Number.isFinite(entry.origin_x) && Number.isFinite(entry.origin_z)) {
      return new THREE.Vector3(entry.origin_x, this.groundAt(entry.origin_x, entry.origin_z) + 0.1, entry.origin_z);
    }
    return this.seatAt(entry.by)?.clone() ?? null;
  }

  /// Hits, blasts, grapples and awards: what happened, drawn where it happened.
  private playEffect(entry: BoardLogEntry, _members: Member[]) {
    if(entry.kind==='blown' && typeof entry.seat==='number') {
      const token=this.tokens.get(entry.seat);
      const from=this.positions.get(entry.from)?.clone();
      const to=this.lastView?this.spot(entry.to,entry.seat,this.lastView):this.positions.get(entry.to)?.clone();
      if(token && from && to) {
        from.y+=0.1;
        this.stormFlights.set(entry.seat,{from,to,elapsed:0});
        token.path=[];token.space=entry.to;token.from=from.clone();token.to=from.clone();token.hop=1;token.wait=0;
        token.rig.group.position.copy(from);
        this.watch(from.clone().lerp(to,0.45),{yaw:this.angleBetween(from,to)+0.5,pitch:35,wide:Math.max(11,from.distanceTo(to)*1.3),life:2.1});
        this.pulse(from,'#a5f3fc');
        this.float(from,`Storm caught! −${entry.coins} coins`,'#a5f3fc',2.7);
      }
    }
    if (entry.kind === 'prize') this.celebratePrize(entry);
    if (entry.kind === 'space' && entry.effect === 'shop' && typeof entry.space === 'number') {
      this.shopping = this.shops.has(entry.space) ? entry.space : null;
      this.lookIntoShop();
    }
    if (entry.kind === 'scatter' && Array.isArray(entry.dropped)) {
      // Coins knocked loose, flying off to wherever they come down.
      const from = this.positions.get(entry.from);
      (entry.dropped as { space: number; coins: number }[]).forEach((pile, i) => this.coinFlow(from, this.positions.get(pile.space), pile.coins, i * 0.05));
    }
    if (entry.kind === 'suction') {
      // Everything in reach pulled in, with a swirl round whoever pulled it.
      const to = this.seatAt(entry.seat);
      for (const pile of (entry.pulled as { space: number; coins: number }[]) ?? []) this.coinFlow(this.positions.get(pile.space), to, pile.coins);
      if (to) {
        this.pulse(to, '#22d3ee');
        if (entry.crate !== null && entry.crate !== undefined) this.float(to, 'Got a crate!', '#fbbf24');
        else if (!entry.coins) this.float(to, 'Nothing in reach', '#cbd5e1');
      }
      this.sounds?.useItem();
    }
    if (entry.kind === 'vacuumed') {
      const to = this.seatAt(entry.seat);
      const special = (list: [number, number][], index: number) => {
        const point = list[index];
        return point ? new THREE.Vector3(point[0], this.terrain.height(point[0], point[1]) + 1, point[1]) : null;
      };
      const from =
        entry.what === 'coconut'
          ? special(this.specials.palms, entry.palm)?.setY(this.terrain.height(this.specials.palms[entry.palm][0], this.specials.palms[entry.palm][1]) + 2.15)
          : entry.what === 'flotsam'
            ? special(this.specials.flotsam, entry.point)?.setY(WATER_Y + 0.5)
            : entry.what === 'snowball'
              ? special(this.specials.snowmen, entry.snowman)
              : this.positions.get(entry.space);
      if (entry.coins) this.coinFlow(from, to, entry.coins);
      const said: Record<string, [string, string]> = {
        air_coin:[`+${entry.coins} floating ${entry.coins===1?'coin':'coins'}`,'#67e8f9'],
        crate: ['Got a crate!', '#fbbf24'],
        snowball: ['Packed a snowball!', '#e0f2fe'],
        lava: [`Obsidian! +${entry.coins}`, '#fb923c'],
        coconut: [entry.item ? 'Golden coconut!' : `Coconut! +${entry.coins}`, '#a3e635'],
        flotsam: [entry.item ? 'Flotsam!' : `Flotsam! +${entry.coins}`, '#7dd3fc'],
        box: ['Bag full', '#c39bff'],
      };
      const line = entry.what === 'box' && entry.item ? undefined : said[entry.what as string];
      if (line && to) this.float(to, line[0], line[1]);
      if (entry.item) {
        this.sounds?.item();
        this.bagItem(entry.seat, entry.item, from);
      }
    }
    if (entry.kind === 'storm_dragged') {
      const at = this.positions.get(entry.to);
      if (at) this.float(at, 'Dragged the storm!', '#94a3b8');
      this.sounds?.thunder();
    }
    if (entry.kind === 'storm_blown') {
      const at = this.positions.get(entry.to);
      if (at) this.float(at, 'Blew the storm away!', '#fdba74');
      this.sounds?.thunder();
    }
    if (entry.kind === 'chest_blown') {
      const at = this.positions.get(entry.to);
      if (at) this.float(at, 'The chest blew over here!', '#fdba74');
    }
    if (entry.kind === 'blown_back') {
      const at = this.seatAt(entry.target);
      if (at) this.float(at, 'Blown back a space!', '#fdba74');
    }
    if (entry.kind === 'upgraded') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, `Vacuum upgrade: ${UPGRADES[entry.upgrade as Upgrade]?.label ?? 'new part'}!`, '#67e8f9');
    }
    if (entry.kind === 'lava_rocks') {
      for (const space of (entry.spaces as number[]) ?? []) {
        const at = this.positions.get(space);
        if (at) {
          this.blast(at);
          this.float(at, 'Lava rock!', '#fb923c');
        }
      }
      this.sounds?.blast(0.5);
    }
    if (entry.kind === 'lava_burn') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, `Burned! -${entry.coins}`, '#fb923c');
    }
    if (entry.kind === 'item' && entry.item === 'snowball' && entry.quality === 'hit' && typeof entry.target === 'number') {
      const at = this.seatAt(entry.target);
      if (at) {
        // Snow bursts off them and a block of ice closes round them for a moment.
        this.puffs(at, ['#ffffff', '#bae6fd'], 10, 0.25);
        const ice = new THREE.Mesh(
          new THREE.BoxGeometry(1.4, 2, 1.4),
          new THREE.MeshStandardMaterial({ color: '#7dd3fc', emissive: '#0ea5e9', emissiveIntensity: 0.3, transparent: true, opacity: 0.45, roughness: 0.1, depthWrite: false }),
        );
        ice.position.copy(at).add(new THREE.Vector3(0, 1, 0));
        this.group.add(ice);
        this.effects.push({ object: ice, life: 2.4, max: 2.4, grow: 0 });
        this.float(at, 'Frozen solid!', '#0284c7');
      }
    }
    if (entry.kind === 'item' && entry.item === 'crate' && entry.quality === 'hit' && typeof entry.space === 'number') {
      const at = this.positions.get(entry.space);
      if (at) {
        this.puffs(at, ['#a8a29e', '#d6d3d1'], 8, 0.25);
        this.float(at, 'Barricade up!', '#fbbf24');
      }
    }
    if (entry.kind === 'siphon') this.coinFlow(this.seatAt(entry.target), this.seatAt(entry.seat), entry.coins);
    if (entry.kind === 'space' && entry.effect === 'prize_here') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, 'Vacuum the candy in!', '#67e8f9');
    }
    if (entry.kind === 'barricade') {
      // Walked into: it breaks when they get there, not when they set off.
      const at = this.positions.get(entry.space);
      if (at) {
        const space = Number(entry.space);
        this.breaking.add(space);
        this.arrivals.push({
          seat: Number(entry.seat),
          wait: 8,
          run: () => {
            this.breaking.delete(space);
            if (!this.lastView?.blockades?.includes(space)) {
              this.barricadeModels.get(space)?.removeFromParent();
              this.barricadeModels.delete(space);
            }
            this.splinters(at);
            this.float(at, 'Barricade!', '#fbbf24');
            const token = this.tokens.get(Number(entry.seat));
            if (token) token.shake = 0.4;
          },
        });
      }
    }
    if (entry.kind === 'smashed') {
      const at = this.positions.get(entry.space);
      if (at) this.splinters(at);
    }
    if (entry.kind === 'scrap' && typeof entry.winner === 'number' && entry.coins > 0) {
      const loser = entry.winner === entry.by ? entry.against : entry.by;
      this.coinFlow(this.seatAt(loser), this.seatAt(entry.winner), entry.coins, 0.6);
    }
    if (entry.kind === 'space' && entry.effect === 'toll' && Array.isArray(entry.paid)) {
      const to = this.seatAt(entry.seat);
      (entry.paid as { seat: number; coins: number }[]).forEach((payer, i) => this.coinFlow(this.seatAt(payer.seat), to, payer.coins, i * 0.15));
    }
    if (entry.kind === 'scrap') {
      // Two of them on one space; the camera drops in for the roll-off.
      const here = this.seatAt(entry.by);
      const there = this.seatAt(entry.against);
      if (here) {
        this.watch(here, {
          yaw: this.angleBetween(here, this.bounds().center),
          pitch: 24,
          wide: 6,
          life: 1.8,
        });
        this.pulse(here, entry.winner === null ? '#cbd5e1' : '#fbbf24');
      }
      const rolls = (entry.rolls as number[]) ?? [];
      if (here) this.float(here, `Scrap! ${rolls[0] ?? ''}`, '#fde68a');
      if (there) this.float(there, `${rolls[1] ?? ''}`, '#fde68a');
      if (entry.winner !== null && entry.coins) {
        const won = this.seatAt(entry.winner);
        if (won) this.float(won, `+${entry.coins}`, '#ffd23f');
      }
    }
    if (entry.kind === 'item' && entry.item === 'compass' && typeof entry.to === 'number' && this.lastView) {
      // The compass carries you there through the air, and the camera goes with you.
      const token = this.tokens.get(Number(entry.by));
      if (token && token.space !== entry.to) {
        this.pulse(token.rig.group.position, ITEMS.compass.color);
        this.hopTo(token, entry.to, this.lastView, true);
        this.riding = token.seat;
        const space = entry.to;
        this.arrivals.push({ seat: token.seat, wait: 4, run: () => this.pulse(this.positions.get(space) ?? token.rig.group.position, ITEMS.compass.color) });
      }
    } else if (entry.kind === 'item') {
      // Swing round behind whoever used it and look along the throw.
      const from = this.castAt(entry);
      const to =
        this.seatAt(entry.target) ??
        (typeof entry.to === 'number' ? this.positions.get(entry.to) : null) ??
        (typeof entry.space === 'number' ? this.positions.get(entry.space) : null) ??
        from;
      if (from && to) {
        const middle = from.clone().add(to).multiplyScalar(0.5);
        const apart = from.distanceTo(to);
        this.watch(middle, {
          yaw: apart > 1 ? this.angleBetween(from, to) + Math.PI / 2 : this.angleBetween(from, this.bounds().center),
          // Close in, look down over the trees; from across the island, flatten
          // out so the throw reads as a line between the two of them.
          pitch: Math.max(34, 48 - apart),
          wide: Math.max(11, apart * 0.8),
          life: 2.1,
        });
      }
    }
    if (entry.kind === 'hurt') {
      const token = this.tokens.get(entry.seat);
      const at = this.seatAt(entry.seat);
      if (token) token.shake = 0.5;
      if (at && !this.knockedNow.has(entry.seat)) this.float(at, `-${entry.damage} heart`, '#fb7185');
      if (at) this.splatter(at, 10);
    } else if (entry.kind === 'prize') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, 'Candy collected!', '#fcd34d', 2.4);
    } else if (entry.kind === 'knockout') {
      this.burst(entry.seat);
    } else if (entry.kind === 'event' && (entry.event === 'bounty' || entry.event === 'plague')) {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, entry.event === 'bounty' ? `Bounty! ${entry.coins} on their head` : 'Caught the plague!', entry.event === 'bounty' ? '#f87171' : '#a3e635');
    } else if (entry.kind === 'event' && entry.event === 'shakedown') {
      const robbed = this.seatAt(entry.target);
      const robber = this.seatAt(entry.seat);
      if (robbed) this.float(robbed, `Shaken down! -${entry.coins}`, '#f87171');
      if (robber) this.float(robber, `+${entry.coins}`, '#ffd23f');
      this.coinFlow(robbed, robber, entry.coins);
    } else if (entry.kind === 'bounty') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, `Bounty claimed! +${entry.coins}`, '#ffd23f');
      this.coinFlow(this.seatAt(entry.target), at, entry.coins);
    } else if (entry.kind === 'plague') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, 'Caught the plague!', '#a3e635');
    } else if (entry.kind === 'plague_bite') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, 'The plague bites', '#a3e635');
    } else if (entry.kind === 'award') {
      const at = this.seatAt(entry.seat);
      if (entry.item) this.bagItem(entry.seat, entry.item);
      else if (at) this.float(at, 'Bag full', '#c39bff');
      if (entry.item && entry.replaced) this.bagOut(entry.seat, entry.replaced);
    } else if (entry.kind === 'bought' && entry.item) {
      // Off the shop counter and into the sack.
      const stall = this.shopping === null ? null : this.shops.get(this.shopping)?.at;
      this.bagItem(entry.seat, entry.item, stall ? stall.clone().add(new THREE.Vector3(0, 1.4, 0)) : null);
    } else if (entry.kind === 'item' && entry.item === 'bomb') {
      this.throwBomb(entry);
    } else if (entry.kind === 'event' && entry.event === 'windfall') {
      for (const token of this.tokens.values()) this.float(token.rig.group.position, `+${entry.coins}`, '#ffd23f');
    } else if (entry.kind === 'event' && entry.event === 'item_rain') {
      ((entry.given as { seat: number; item: Item }[]) ?? []).forEach((gift, i) => this.bagItem(gift.seat, gift.item, null, i * 0.12));
    } else if (entry.kind === 'bets' && entry.each) {
      for (const seat of (entry.winners as number[]) ?? []) {
        const at = this.seatAt(seat);
        if (at) this.float(at, `Pot! +${entry.each}`, '#ffd23f');
      }
    } else if (entry.kind === 'event' && entry.event === 'underdog') {
      const at = this.seatAt(entry.seat);
      if (at) {
        this.pulse(at, '#4ade80');
        this.float(at, 'Cheap candy!', '#4ade80');
        if (entry.item) this.bagItem(entry.seat, entry.item, null, 0.3);
      }
    } else if (entry.kind === 'item' && (entry.item === 'swap' || entry.item === 'pickpocket')) {
      const from = this.seatAt(entry.by);
      const to = this.seatAt(entry.target);
      if (from && to) this.beam(from, to, ITEMS[entry.item as Item].color);
      if (entry.item === 'pickpocket' && entry.took) this.bagItem(entry.by, entry.took, this.bagMouth(entry.target), 0.2);
    } else if (entry.kind === 'item' && entry.item === 'curse') {
      // A purple cloud boils up round whoever it caught.
      const at = this.seatAt(entry.target);
      if (at) {
        this.puffs(at, ['#7c3aed', '#7c3aed', '#c4b5fd'], 9, 0.35);
        this.tokens.get(Number(entry.target))!.shake = 0.4;
      }
    } else if (entry.kind === 'item' && entry.item === 'heist') {
      const from = this.seatAt(entry.target);
      const to = this.seatAt(entry.by);
      if (from) this.float(from, 'Prize taken!', '#f472b6');
      if (from && to) {
        // The prize lifts off them and arcs over to the thief.
        const gem = new THREE.Mesh(
          this.track(new THREE.OctahedronGeometry(0.4)),
          this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f472b6', emissive: '#db2777', emissiveIntensity: 0.8, roughness: 0.3 })),
        );
        this.group.add(gem);
        const thief = Number(entry.by);
        this.flights.push({
          bomb: gem,
          from: from.clone().add(new THREE.Vector3(0, 1.6, 0)),
          to: to.clone().add(new THREE.Vector3(0, 1.6, 0)),
          t: 0,
          seconds: 1.1,
          land: () => {
            const at = this.seatAt(thief) ?? to;
            this.pulse(at, ITEMS.heist.color);
            this.puffs(at, ['#f472b6', '#fbcfe8'], 8, 0.2);
            this.float(at, '+1 Star, stolen!', '#f472b6');
          },
        });
      }
    } else if (entry.kind === 'item' && entry.item === 'compass' && entry.quality!=='fail') {
      const at=this.positions.get(entry.to);
      const chest=this.lastView?this.positions.get(this.lastView.prize_at):null;
      const spaces=entry.quality==='perfect'?1:entry.quality==='good'?3:6;
      if(at) {
        this.pulse(at,'#f9a8d4');
        this.float(at,`Landed · ${spaces} ${spaces===1?'space':'spaces'} before the chest`,'#f9a8d4',2.8);
        if(chest) {
          this.beam(at,chest,'#f9a8d4',1.8);
          this.pulse(chest,'#fcd34d');
          this.float(chest,'PRIZE · vacuum here','#fcd34d',2.8);
          this.watch(at.clone().lerp(chest,0.4),{yaw:this.angleBetween(at,chest)+0.3,pitch:36,wide:Math.max(10,at.distanceTo(chest)*1.3),life:1.8});
        }
      }
    } else if (entry.kind === 'item' && (entry.item === 'medkit' || entry.item === 'shield' || entry.item === 'double_dice' || entry.item === 'golden_die')) {
      const at = this.seatAt(entry.by);
      if (at) {
        const color = ITEMS[entry.item as Item].color;
        this.pulse(at, color);
        // What it did, said over them once the grade has been, and shown.
        if (entry.item === 'medkit') {
          this.puffs(at, ['#4ade80', '#bbf7d0', '#f87171'], 10, 0.22);
          const healed = Number(entry.healed) || 0;
          this.float(at, healed > 0 ? `+${healed} ${healed === 1 ? 'heart' : 'hearts'}` : 'Already at full health', '#4ade80');
        } else if (entry.item === 'shield') {
          this.float(at, entry.up ? 'Shield up!' : 'The shield fizzled', '#93c5fd');
        } else if (entry.item === 'double_dice') {
          this.puffs(at, ['#fbbf24', '#fde68a'], 8, 0.2);
          this.float(at, `${Number(entry.dice) || 2} dice next roll!`, '#fde68a');
        } else if (entry.item === 'golden_die') {
          this.puffs(at, ['#facc15', '#fef08a'], 8, 0.2);
          this.float(at, `Rolls a ${entry.number} next!`, '#fde047');
        }
        // A close look at them while it happens.
        this.watch(at, { yaw: this.angleBetween(at, this.bounds().center), pitch: 22, wide: 5, life: 2 });
      }
    } else if (entry.kind === 'item' && entry.item === 'grapple') {
      const from = this.seatAt(entry.by);
      const to = this.seatAt(entry.target);
      const coins=Number(entry.coins)||0;
      if (coins>0 && entry.quality==='hit') {
        if (from && to) this.beam(from, to, '#22d3ee', 1.8);
        this.coinFlow(to, from, coins, 0.12, 1.5);
        if (to) this.float(to, `Hooked! −${coins} coins`, '#ff6b6b', 2.8);
        if (from) this.float(from, `Reeled in ${coins} coins!`, '#ffd23f', 2.8);
      } else if (from) this.float(from, 'Hook landed · empty pockets', '#cbd5e1', 2.8);
    }
  }

  /** A bomb lobbed from the thrower to where it lands, going off as it comes down. */
  private throwBomb(entry: BoardLogEntry) {
    const from = this.castAt(entry);
    if (!from || typeof entry.x !== 'number' || typeof entry.z !== 'number') return;
    const center = new THREE.Vector3(entry.x, this.groundAt(entry.x, entry.z), entry.z);
    const labels: Record<string, [string, string]> = {
      direct: ['Direct hit!', '#fde047'],
      splash: ['Caught them!', '#fdba74'],
      miss: ['Missed!', '#cbd5e1'],
      in_hand: ['Held it too long!', '#f87171'],
    };
    const land = () => {
      this.blast(center);
      if (this.mature) this.stain(center, entry.quality === 'in_hand' ? 1.4 : 2.2, '#1c1917', 40);
      this.sounds?.blast();
      const [label, color] = labels[entry.quality as string] ?? labels.miss;
      this.float(center, label, color);
      for (const seat of (entry.direct as number[]) ?? []) {
        const at = this.seatAt(seat);
        if (at) this.float(at, `Direct hit! -${entry.direct_coins}`, '#ff6b6b');
      }
      for (const seat of (entry.caught as number[]) ?? []) {
        const at = this.seatAt(seat);
        if (at) this.float(at, `-${entry.coins}`, '#ff6b6b');
      }
    };
    if (entry.quality === 'in_hand') {
      land();
      return;
    }
    const bomb = new THREE.Group();
    const shell = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.34, 14, 10)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#1f2937', roughness: 0.4, metalness: 0.3 })),
    );
    const spark = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.1, 8, 6)),
      this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#fde047' })),
    );
    spark.position.y = 0.42;
    bomb.add(shell, spark);
    this.group.add(bomb);
    this.sounds?.useItem();
    this.flights.push({
      bomb,
      from: from.add(new THREE.Vector3(0, 1.4, 0)),
      to: center,
      t: 0,
      seconds: Math.max(0.5, Number(entry.flight) || 1),
      land,
    });
  }

  /** The first part of the arc a throw would take, while a bomb is aimed; null clears it. */
  setArc(points: THREE.Vector3[] | null) {
    if (!points) {
      if (this.arc) this.arc.visible = false;
      return;
    }
    if (!this.arc) {
      this.arc = new THREE.Points(
        this.track(new THREE.BufferGeometry()),
        // A few pixels across however near, so the start of the arc by the hand is not a wall.
        this.trackMaterial(new THREE.PointsMaterial({ color: '#fde68a', size: 7, sizeAttenuation: false, transparent: true, opacity: 0.9, depthWrite: false })),
      );
      this.arc.frustumCulled = false;
      this.arc.renderOrder = 21;
      this.group.add(this.arc);
    }
    this.arc.geometry.setFromPoints(points);
    this.arc.visible = true;
  }

  /** Hides the coins, crates, dropped piles and the prize, so the people stand
   * out while something is aimed; trees and rocks stay, since they get in the way. */
  setClear(clear: boolean) {
    if (clear === this.clear) return;
    this.clear = clear;
    for (const object of this.pickups) object.visible = !clear;
    for (const pile of this.spills.values()) pile.visible = !clear;
    if (this.prize) this.prize.visible = !clear;
  }

  /** Something other than a bomb, thrown: it flies, and does what it does where it comes down. */
  private throwThing(entry: BoardLogEntry, members: Member[]) {
    const from = this.castAt(entry);
    if (!from || typeof entry.x !== 'number' || typeof entry.z !== 'number') return;
    const center = new THREE.Vector3(entry.x, this.groundAt(entry.x, entry.z), entry.z);
    const color = ITEMS[entry.item as Item]?.color ?? '#ffffff';
    const thing = new THREE.Group();
    const shape =
      entry.item === 'crate'
        ? new THREE.BoxGeometry(0.6, 0.6, 0.6)
        : entry.item === 'grapple'
        ? new THREE.TorusGeometry(0.26, 0.07, 6, 12, Math.PI * 1.4)
        : entry.item === 'curse'
          ? new THREE.OctahedronGeometry(0.3)
          : entry.item === 'pickpocket'
            ? new THREE.DodecahedronGeometry(0.28)
            : new THREE.SphereGeometry(0.3, 14, 10);
    const body = new THREE.Mesh(
      this.track(shape),
      this.trackMaterial(
        entry.item === 'crate'
          ? new THREE.MeshStandardMaterial({ color: '#a16207', roughness: 0.9 })
          : new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: entry.item === 'snowball' ? 0.2 : 0.7, roughness: 0.4 }),
      ),
    );
    thing.add(body);
    this.group.add(thing);
    this.sounds?.useItem();
    const label: Record<string, [string, string]> = {
      miss: ['Missed!', '#cbd5e1'],
      blocked: ['Blocked!', '#93c5fd'],
      empty: ['Empty pockets!', '#cbd5e1'],
    };
    const slowCatch=entry.item==='grapple' && entry.quality==='hit' && Number(entry.coins)>0;
    const originalFlight=Math.max(0.5, Number(entry.flight)||1);
    if (slowCatch) {
      const target=this.seatAt(entry.target)??center;
      const middle=from.clone().lerp(target,0.5);
      this.watch(middle,{yaw:this.angleBetween(from,target)+0.4,pitch:30,wide:Math.max(10,from.distanceTo(target)*1.4),life:grappleFlightSeconds(entry)+2});
    }
    // A swap trades places when it hits, not before: the two leap past each other then.
    const swapped = entry.item === 'swap' && entry.quality === 'hit' ? [Number(entry.by), Number(entry.target)] : [];
    for (const seat of swapped) this.stayPut.add(seat);
    this.flights.push({
      bomb: thing,
      from: from.add(new THREE.Vector3(0, 1.4, 0)),
      to: center,
      t: 0,
      seconds: entry.item==='grapple'?grappleFlightSeconds(entry):originalFlight,
      rope: entry.item === 'grapple',
      slowCatchFlight: slowCatch?originalFlight:undefined,
      land: () => {
        this.pulse(center, color);
        for (const seat of swapped) {
          this.stayPut.delete(seat);
          const view3d = this.tokens.get(seat);
          const space = this.lastView?.tokens.find((token) => token.seat === seat)?.space;
          if (view3d && space !== undefined && space !== view3d.space && this.lastView) this.hopTo(view3d, space, this.lastView, true);
        }
        const said = label[entry.quality as string];
        if (said) {
          this.float(center, entry.item==='grapple' && entry.quality==='empty'?'Hook landed · empty pockets':said[0], said[1], 2.4);
          this.sounds?.blocked();
          return;
        }
        this.playEffect(entry, members);
        this.hear(entry);
        const at = this.seatAt(entry.target);
        if (at && entry.item !== 'grapple' && entry.item !== 'pickpocket') this.float(at, `${itemLabel(entry.item)}!`, color);
      },
    });
  }

  /** The bat's swing: a sweep of light round the arc in front of the batter. */
  private swoosh(entry: BoardLogEntry) {
    const at = this.castAt(entry);
    if (!at) return;
    const bat = this.batOf(Number(entry.by));
    bat.yaw = Number(entry.yaw);
    bat.swing = 0;
    const arc = 0.75;
    const sweep = new THREE.Mesh(
      new THREE.RingGeometry(0.8, 5, 24, 1, -arc, arc * 2),
      new THREE.MeshBasicMaterial({ color: entry.quality === 'perfect' ? '#fde047' : '#ffffff', transparent: true, opacity: 0.55, depthWrite: false, side: THREE.DoubleSide }),
    );
    sweep.rotation.x = -Math.PI / 2;
    // The ring's angle 0 runs along +x; turn it to face the way the swing went.
    sweep.rotation.z = Math.PI / 2 - Number(entry.yaw);
    sweep.position.set(at.x, at.y + 0.9, at.z);
    this.group.add(sweep);
    this.effects.push({ object: sweep, life: 0.45, max: 0.45, grow: 0 });
    if (entry.quality === 'whiff' || entry.quality === 'miss') this.float(at, entry.quality === 'whiff' ? 'Whiff!' : 'Missed!', '#cbd5e1');
    this.sounds?.useItem();
  }

  /**
   * Turns a seat's token to face the way it is aiming, with a glowing ring at
   * its feet and an arrow on the ground ahead, so you can tell which is you
   * among the others on your space. Null clears it.
   */
  setAimer(seat: number | null, yaw = 0) {
    this.aimer = seat === null ? null : { seat, yaw };
    if (!this.aimerMark) {
      const glow = this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#fde047', transparent: true, opacity: 0.9, depthWrite: false }));
      const ring = new THREE.Mesh(this.track(new THREE.RingGeometry(0.55, 0.72, 32)), glow);
      ring.rotation.x = -Math.PI / 2;
      const arrow = new THREE.Mesh(this.track(new THREE.ConeGeometry(0.28, 0.7, 3)), glow);
      // Lying flat on the ground, pointing ahead.
      arrow.rotation.x = Math.PI / 2;
      arrow.position.set(0, 0, 1.25);
      const mark = new THREE.Group();
      mark.add(ring, arrow);
      mark.renderOrder = 22;
      this.group.add(mark);
      this.aimerMark = mark;
    }
    this.aimerMark.visible = seat !== null;
  }

  /**
   * Hides every prop standing between `eye` and `at`, or around the eye
   * itself, so a camera close in over a shoulder never looks through a palm.
   * Null puts them all back.
   */
  private staticProps=new StaticProps(this.group,placed);
  clearSightline(eye: THREE.Vector3 | null, at?: THREE.Vector3) {
    const hide = new Set<THREE.Object3D>();
    if (eye && at) {
      const reach = at.distanceTo(eye);
      const ray = new THREE.Ray(at, eye.clone().sub(at).normalize());
      const hit = new THREE.Vector3();
      for (const prop of placed) {
        if (!prop.parent) {
          placed.delete(prop);
          continue;
        }
        let box = prop.userData.box as THREE.Box3 | undefined;
        if (!box) {
          box = new THREE.Box3().setFromObject(prop);
          prop.userData.box = box;
        }
        // Not whatever they are standing on or in.
        if (box.containsPoint(at)) continue;
        // In the way, or with the lens inside it.
        const blocking = (ray.intersectBox(box, hit) !== null && hit.distanceTo(at) < reach) || box.distanceToPoint(eye) < 0.4;
        // Only what is on show: something hidden for its own reasons stays hidden after.
        if ((prop.visible || this.cleared.has(prop)) && blocking) hide.add(prop);
      }
    }
    for (const prop of this.cleared) if (!hide.has(prop)) prop.visible = true;
    for (const prop of hide) prop.visible = false;
    this.cleared = hide;
    this.staticProps.update(performance.now()/1000);
  }

  /** The thing in the air now, or where it came down a moment ago, for the camera to follow. */
  chase(): THREE.Vector3 | null {
    const flight = this.flights[this.flights.length - 1];
    // Partway to where it comes down, so the target stays in shot as well as the throw.
    if (flight) return flight.bomb.position.clone().lerp(flight.to, 0.45);
    if (this.landing && performance.now() - this.landing.time < 1600) return this.landing.at.clone();
    return null;
  }

  /**
   * Someone vacuumed the chest in: the camera comes close, the jewel bursts
   * into confetti over them, and their coins go to the chest.
   */
  private celebratePrize(entry: BoardLogEntry) {
    const seat = Number(entry.seat);
    const at = this.seatAt(seat);
    if (!at) return;
    this.prizeWinner = { seat, until: performance.now() + 2600 };
    this.watch(at, { yaw: this.angleBetween(at, this.bounds().center), pitch: 20, wide: 6, life: 2.8 });
    const colors = ['#f472b6', '#fde047', '#60a5fa', '#4ade80', '#f97316', '#ffffff'];
    const floor = this.groundAt(at.x, at.z);
    const paper = this.track(new THREE.PlaneGeometry(0.16, 0.24));
    for (let i = 0; i < 60; i++) {
      const mesh = new THREE.Mesh(paper, new THREE.MeshBasicMaterial({ color: colors[i % colors.length], side: THREE.DoubleSide }));
      mesh.position.copy(at).add(new THREE.Vector3(0, 2.2, 0));
      this.group.add(mesh);
      const angle = Math.random() * Math.PI * 2;
      const speed = 1.5 + Math.random() * 3;
      this.bits.push({
        mesh,
        velocity: new THREE.Vector3(Math.cos(angle) * speed, 6 + Math.random() * 5, Math.sin(angle) * speed),
        spin: new THREE.Vector3(Math.random() * 12, Math.random() * 12, Math.random() * 12),
        life: 2.4,
        floor,
      });
    }
    this.float(at, '+1 Star!', '#f472b6');
    if (entry.coins) this.float(at, `-${entry.coins} coins`, '#fde68a');
  }

  /** Little clouds boiling up round a point, in the given colours, then gone. */
  private puffs(at: THREE.Vector3, colors: string[], count: number, size: number) {
    for (let i = 0; i < count; i++) {
      const puff = new THREE.Mesh(
        new THREE.SphereGeometry(size + Math.random() * size * 0.7, 10, 8),
        new THREE.MeshBasicMaterial({ color: colors[i % colors.length], transparent: true, opacity: 0.55, depthWrite: false }),
      );
      const angle = (i / count) * Math.PI * 2;
      puff.position.copy(at).add(new THREE.Vector3(Math.cos(angle) * 0.7, 0.4 + Math.random() * 1.4, Math.sin(angle) * 0.7));
      this.group.add(puff);
      this.effects.push({ object: puff, life: 1.4, max: 1.4, grow: 1.2, rise: 0.8 + Math.random() * 0.6 });
    }
  }

  /** One hop to a space, or one high leap when `flying`. */
  private hopTo(view3d: TokenView, space: number, view: BoardView, flying = false) {
    view3d.from = view3d.rig.group.position.clone();
    view3d.to = this.spot(space, view3d.seat, view);
    view3d.hop = 0;
    view3d.space = space;
    view3d.flying = flying;
    view3d.yaw = Math.atan2(view3d.to.x - view3d.from.x, view3d.to.z - view3d.from.z);
  }

  /** Puts the bat on a seat's shoulder while they aim it, or away with null. */
  holdBat(seat: number | null) {
    this.batHolder = seat;
  }

  private batOf(seat: number) {
    let bat = this.bats.get(seat);
    if (!bat) {
      const wood = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#b45309', roughness: 0.7 }));
      const grip = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#1f2937', roughness: 0.9 }));
      // Along +z from the hands, thick end out.
      const barrel = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.13, 0.05, 1.3, 10)), wood);
      barrel.rotation.x = Math.PI / 2;
      barrel.position.z = 0.75;
      const handle = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.055, 0.055, 0.3, 8)), grip);
      handle.rotation.x = Math.PI / 2;
      handle.position.z = 0.05;
      barrel.castShadow = true;
      const pivot = new THREE.Group();
      pivot.rotation.order = 'YXZ';
      pivot.add(barrel, handle);
      pivot.visible = false;
      this.group.add(pivot);
      bat = { pivot, yaw: 0, swing: null };
      this.bats.set(seat, bat);
    }
    return bat;
  }

  /**
   * Marks each rival the bat would hit from a seat facing `yaw` with a red
   * arrow over their head, and returns how many that is. Null clears them.
   */
  setBatReach(seat: number | null, yaw = 0): number {
    this.batInReach.clear();
    const swinger = seat === null ? null : this.tokens.get(seat);
    const from = swinger ? this.positions.get(swinger.space) : undefined;
    if (swinger && from) {
      const team = this.pairs.find((one) => one.seat === swinger.seat)?.team;
      for (const token of this.tokens.values()) {
        if (token.seat === swinger.seat) continue;
        if (team !== undefined && this.pairs.some((one) => one.seat === token.seat && one.team === team)) continue;
        const at = this.positions.get(token.space);
        if (!at) continue;
        const dx = at.x - from.x;
        const dz = at.z - from.z;
        const away = Math.hypot(dx, dz);
        const off = THREE.MathUtils.euclideanModulo(Math.atan2(dx, dz) - yaw + Math.PI, Math.PI * 2) - Math.PI;
        if (away <= BAT.range && (away < BAT_CLOSE || Math.abs(off) <= BAT.arc)) this.batInReach.add(token.seat);
      }
    }
    for (const [seat, mark] of this.batMarks) mark.visible = this.batInReach.has(seat);
    return this.batInReach.size;
  }

  /** Hides a seat's name tag while the camera looks over its shoulder, so it is not in the way. */
  hideSeat(seat: number | null) {
    this.hiddenTag = seat;
  }

  /** Keep the local character from clipping through an eye-level camera. */
  hideBody(seat: number | null) {
    if (this.hiddenBody !== seat && this.hiddenBody !== null) {
      const previous = this.tokens.get(this.hiddenBody);
      if (previous) previous.rig.group.visible = previous.burst <= 0;
    }
    this.hiddenBody = seat;
    if (seat !== null) {
      const token = this.tokens.get(seat);
      if (token) token.rig.group.visible = false;
    }
  }

  facing(seat: number | null) {
    return seat === null ? 0 : this.tokens.get(seat)?.rig.group.rotation.y ?? 0;
  }

  /** Predict our own cosmetic turn while its authenticated update travels. */
  faceSeat(seat: number, yaw: number) {
    if (Number.isFinite(yaw)) this.localFacing = {seat,yaw,at:performance.now()};
  }

  /** The seat whose token is under the pointer, if any. */
  pickToken(raycaster: THREE.Raycaster): number | null {
    let best: { seat: number; distance: number } | null = null;
    for (const token of this.tokens.values()) {
      const hit = raycaster.intersectObject(token.rig.group, true)[0];
      if (hit && (!best || hit.distance < best.distance)) best = { seat: token.seat, distance: hit.distance };
    }
    return best?.seat ?? null;
  }

  /** Holds a token up over a space while it is being dragged there, or puts it back with null. */
  dragPreview(seat: number, space: number | null) {
    const over = space === null ? null : this.positions.get(space);
    this.held = over ? { seat, at: over.clone().add(new THREE.Vector3(0, 0.9, 0)) } : null;
  }

  /** The space under the pointer, if any. */
  pickSpace(raycaster: THREE.Raycaster): number | null {
    const tile = raycaster.intersectObjects([...this.tiles.values()], false)[0];
    const id = tile?.object.userData.space;
    return typeof id === 'number' ? id : null;
  }

  /**
   * Coins taken from one player flying over to another, one after another in
   * an arc, so it is plain who paid whom. A few stand for a big pile.
   */
  private coinFlow(from: THREE.Vector3 | null | undefined, to: THREE.Vector3 | null | undefined, coins: number, delay = 0, durationScale = 1) {
    if (!from || !to || coins <= 0) return;
    if (!this.coinGeometry) {
      this.coinGeometry = this.track(new THREE.CylinderGeometry(0.27, 0.27, 0.07, 18));
      this.coinMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#fbbf24', metalness: 0.4, roughness: 0.3, emissive: '#7c4a03', emissiveIntensity: 0.3 }));
    }
    const shown = Math.min(coins, 12);
    const start = from.clone().add(new THREE.Vector3(0, 1.2, 0));
    const end = to.clone().add(new THREE.Vector3(0, 1.2, 0));
    const across = new THREE.Vector3(-(end.z - start.z), 0, end.x - start.x).normalize();
    const distance = start.distanceTo(end);
    for (let i = 0; i < shown; i++) {
      const mesh = new THREE.Mesh(this.coinGeometry, this.coinMaterial!);
      mesh.visible = false;
      mesh.scale.setScalar(0.85 + Math.random() * 0.35);
      this.group.add(mesh);
      this.coinsInFlight.push({
        mesh,
        from: start.clone(),
        to: end.clone(),
        // A little irregular, the way a handful comes out, not a metronome.
        t: -(delay + i * 0.075 + Math.random() * 0.03),
        seconds: (0.6 + Math.min(0.55, distance * 0.02) + Math.random() * 0.12) * durationScale,
        // Fanned out either side of the line between them, and each arcing its own height.
        side: across.clone().multiplyScalar((Math.random() - 0.5) * Math.min(2.4, 0.6 + distance * 0.12)),
        height: (1.3 + distance * 0.12) * (0.7 + Math.random() * 0.6),
        flip: (2 + Math.random() * 2.5) * (Math.random() < 0.5 ? -1 : 1),
        phase: Math.random() * Math.PI * 2,
        yaw: Math.random() * Math.PI * 2,
        index: i,
        last: i === shown - 1,
      });
    }
  }

  /** The coconuts, lava rocks, flotsam and snowman as the board has them now. */
  private syncSpecials(view: BoardView) {
    const keep = (key: number, wanted: boolean, make: () => THREE.Object3D) => {
      const have = this.specialModels.get(key);
      if (wanted && !have) {
        const model = make();
        this.group.add(model);
        this.specialModels.set(key, model);
      } else if (!wanted && have) {
        have.removeFromParent();
        this.specialModels.delete(key);
      }
    };
    // Coconuts up in each palm, one model per palm holding as many as are left.
    this.specials.palms.forEach(([x, z], index) => {
      const left = view.coconuts?.[index] ?? 0;
      const key = pullKey(PULL.palm, index);
      const have = this.specialModels.get(key);
      if (have && have.userData.left !== left) {
        have.removeFromParent();
        this.specialModels.delete(key);
      }
      keep(key, left > 0, () => {
        const group = new THREE.Group();
        group.userData.left = left;
        group.position.set(x, this.terrain.height(x, z) + 2.15, z);
        const nut = this.coconutMaterial ?? (this.coconutMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#6b3f1d', roughness: 0.8 })));
        for (let i = 0; i < left; i++) {
          const mesh = new THREE.Mesh(this.track(new THREE.SphereGeometry(0.32, 10, 8)), nut);
          mesh.position.set(Math.cos(i * 2.6) * 0.45, -Math.random() * 0.2, Math.sin(i * 2.6) * 0.45);
          group.add(mesh);
        }
        return group;
      });
    });
    for (const space of view.lava_rocks ?? []) {
      keep(pullKey(PULL.lava, space), true, () => {
        const at = this.positions.get(space)!;
        const rock = new THREE.Mesh(
          this.track(new THREE.DodecahedronGeometry(0.45, 0)),
          this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#2a1a14', emissive: '#ff4d00', emissiveIntensity: 0.9, roughness: 0.7, flatShading: true })),
        );
        rock.position.set(at.x + 0.5, at.y + 0.45, at.z - 0.3);
        rock.userData.glow = true;
        return rock;
      });
    }
    for (const [key] of this.specialModels) {
      if (key >> 8 === PULL.lava && !(view.lava_rocks ?? []).includes(key & 255)) keep(key, false, () => new THREE.Group());
    }
    this.specials.flotsam.forEach(([x, z], index) => {
      keep(pullKey(PULL.flotsam, index), (view.flotsam ?? []).includes(index), () => {
        const group = new THREE.Group();
        group.position.set(x, WATER_Y + 0.1, z);
        group.userData.bob = true;
        void place(group, 'barrel', { x: 0, z: 0, y: 0, scale: 0.9, yaw: index });
        return group;
      });
    });
    this.specials.snowmen.forEach(([x, z], index) => {
      keep(pullKey(PULL.snowman, index), view.snowmen?.[index] ?? false, () => {
        const group = new THREE.Group();
        group.position.set(x, this.terrain.height(x, z), z);
        const snow = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f8fafc', roughness: 0.9 }));
        [0.75, 0.55, 0.38].reduce((y, r) => {
          const ball = new THREE.Mesh(this.track(new THREE.SphereGeometry(r, 14, 10)), snow);
          ball.position.y = y + r;
          group.add(ball);
          return y + r * 1.75;
        }, 0);
        const carrot = new THREE.Mesh(this.track(new THREE.ConeGeometry(0.08, 0.35, 6)), this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f97316' })));
        carrot.rotation.x = Math.PI / 2;
        carrot.position.set(0, 2.45, 0.38);
        group.add(carrot);
        // A red scarf and a black hat, so it stands out against the snow.
        const scarf = new THREE.Mesh(this.track(new THREE.TorusGeometry(0.42, 0.11, 8, 18)), this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#dc2626' })));
        scarf.rotation.x = Math.PI / 2;
        scarf.position.y = 2.1;
        const black = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#111827' }));
        const brim = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.5, 0.5, 0.06, 18)), black);
        brim.position.y = 2.85;
        const crown = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.32, 0.32, 0.5, 18)), black);
        crown.position.y = 3.1;
        group.add(scarf, brim, crown);
        group.scale.setScalar(1.3);
        return group;
      });
    });
    // A purple space's box is gone once someone has vacuumed it this round.
    for (const [key, model] of this.specialModels) {
      if (key >> 8 === PULL.box) model.visible = !(view.boxes_taken ?? []).includes(key & 255) && !this.clear;
    }
  }

  /** Crates and barricades, added and taken away as the board changes. */
  private syncCrates(view: BoardView) {
    const keep = (models: Map<number, THREE.Object3D>, spaces: number[], make: (at: THREE.Vector3, space: number) => THREE.Object3D) => {
      const wanted = new Set(spaces);
      for (const [space, model] of models) {
        if (wanted.has(space)) continue;
        model.removeFromParent();
        models.delete(space);
      }
      for (const space of spaces) {
        if (models.has(space)) continue;
        const at = this.positions.get(space);
        if (!at) continue;
        const model = make(at, space);
        this.group.add(model);
        models.set(space, model);
      }
    };
    keep(this.crateModels, view.crates ?? [], (at, space) => {
      // Beside the tile, so it does not hide the space under it.
      const group = new THREE.Group();
      const angle = space * 2.3;
      group.position.set(at.x + Math.cos(angle) * 0.95, at.y, at.z + Math.sin(angle) * 0.95);
      group.rotation.y = angle;
      void place(group, 'crate', { x: 0, z: 0, y: 0, scale: 0.75 }).then((crate) => crate && this.pickups.push(crate));
      return group;
    });
    // A barricade someone is still walking into stays up until they reach it.
    keep(this.barricadeModels, [...new Set([...(view.blockades ?? []), ...this.breaking])], (at, space) => {
      // Across the path: crates stacked behind a fence, facing the way it runs.
      const group = new THREE.Group();
      group.position.copy(at);
      const ahead = this.positions.get(this.nextOf.get(space)?.[0] ?? -1);
      if (ahead) group.rotation.y = Math.atan2(ahead.x - at.x, ahead.z - at.z);
      void place(group, 'iron-fence', { x: 0, z: 0, y: 0.1, scale: 1.3 });
      void place(group, 'crate', { x: -0.45, z: -0.35, y: 0.1, scale: 0.7, yaw: 0.3 });
      void place(group, 'crate', { x: 0.45, z: -0.35, y: 0.1, scale: 0.7, yaw: -0.4 });
      void place(group, 'barrel', { x: 0, z: -0.5, y: 0.6, scale: 0.6 });
      return group;
    });
  }

  vacuumFeedback():string|null {return this.vacuumTargetMessage;}
  private vacuumTargetMessage:string|null=null;
  private vacuumPitch=0;

  /** The vacuum out in a seat's hands, pointed along `yaw`; null puts it away. */
  setVacuum(seat: number | null, view: VacuumView | null, yaw?: number, pitch=0) {
    if (view !== this.vacuumView) this.vacuumAt = performance.now();
    this.vacuumSeat = seat;
    this.vacuumView = view;
    this.vacuumYaw = yaw ?? view?.yaw ?? 0;
    this.vacuumPitch=pitch;
    if (!this.vacuumReticle) {
      const reticle = new THREE.Group();
      reticle.name = 'vacuum-range-reticle';
      const ink = this.trackMaterial(new THREE.MeshBasicMaterial({color:'#082f49',transparent:true,opacity:0.9,depthTest:false,depthWrite:false,side:THREE.DoubleSide,fog:false}));
      const light = this.trackMaterial(new THREE.MeshBasicMaterial({color:'#a5f3fc',transparent:true,opacity:1,depthTest:false,depthWrite:false,side:THREE.DoubleSide,fog:false}));
      reticle.add(new THREE.Mesh(this.track(new THREE.RingGeometry(0.20,0.30,40)),ink));
      reticle.add(new THREE.Mesh(this.track(new THREE.RingGeometry(0.235,0.265,40)),light));
      for (const side of [-1,1]) {
        const horizontal=new THREE.Mesh(this.track(new THREE.BoxGeometry(0.18,0.025,0.01)),light);
        horizontal.position.x=side*0.39;reticle.add(horizontal);
        const vertical=new THREE.Mesh(this.track(new THREE.BoxGeometry(0.025,0.18,0.01)),light);
        vertical.position.y=side*0.39;reticle.add(vertical);
      }
      reticle.renderOrder=100;
      reticle.traverse(part=>part.renderOrder=100);
      this.vacuumReticle=reticle;this.group.add(reticle);
      const puff=this.track(new THREE.SphereGeometry(0.045,6,4));
      const glow=this.trackMaterial(new THREE.MeshBasicMaterial({color:'#a5f3fc',transparent:true,opacity:0.8,depthWrite:false,fog:false}));
      this.vacuumStream=Array.from({length:40},()=>{
        const mesh=new THREE.Mesh(puff,glow);mesh.name='vacuum-hitbox-particle';this.group.add(mesh);return mesh;
      });
    }
  }

  /** Particle positions use the authoritative horizontal hit sector, including
   * its circular range boundary. The world reticle is its far centre. */
  private updateVacuum(time: number) {
    const token = this.vacuumSeat === null ? null : this.tokens.get(this.vacuumSeat);
    const view = token ? this.vacuumView : null;
    const out = view ? view.out + (this.replaying ? 0 : (performance.now() - this.vacuumAt) / 1000) : 0;
    const cone=view?.cone??VACUUM.cone;
    const range=view?.range??VACUUM.range;
    const rate=view?.rate??1;
    const origin=token?.rig.group.position.clone().add(new THREE.Vector3(0,1.3,0));
    let target:{at:THREE.Vector3;text:string;blocked:boolean;score:number}|null=null;
    const candidate=(x:number,z:number,y:number,text:string,blocked=false)=>{
      if(!view||!origin)return;
      const dx=x-origin.x,dz=z-origin.z,distance=Math.hypot(dx,dz);
      const off=Math.abs(Math.atan2(Math.sin(Math.atan2(dx,dz)-this.vacuumYaw),Math.cos(Math.atan2(dx,dz)-this.vacuumYaw)));
      if(distance>range||off>cone)return;
      const score=Math.abs(Math.atan2(y-origin.y,distance)-this.vacuumPitch)*8+off*10+distance*.03;
      if(!target||score<target.score)target={at:new THREE.Vector3(x,y,z),text,blocked,score};
    };
    const crawler=this.lastView?.crawler;
    if(crawler)candidate(crawler.x,crawler.z,this.groundAt(crawler.x,crawler.z)+.4,view?.on?(view.blowing?'PUSHING ZOMBIE':'PULLING ZOMBIE'):'ZOMBIE · HOLD TO PULL / RMB TO PUSH');
    const board=this.lastView;
    const coins=board?.tokens.find(t=>t.seat===this.vacuumSeat)?.coins??0;
    const price=board?.prize_cost??15;
    if(this.prize&&board){const at=this.prize.position;candidate(at.x,at.z,at.y+.25,coins<price?`CANDY · $${price} · NEED $${price-coins} MORE`:'CANDY · HOLD TO COLLECT',coins<price);}
    for(const npc of board?.pedestrians??[])if(npc.remaining_cash>0)candidate(npc.x,npc.z,this.groundAt(npc.x,npc.z)+(npc.knocked_down?.25:1.0),`WALLET · $${npc.remaining_cash} · HOLD TO ROB`);
    this.crawler.highlight(!!target&&(target as {text:string}).text.includes('ZOMBIE'));
    this.vacuumTargetMessage=target?(target as {text:string}).text:null;
    if(this.vacuumReticle) {
      this.vacuumReticle.visible=!!view;
      if(view&&origin) {
        const end=target?(target as {at:THREE.Vector3}).at:vacuumPoint(origin,this.vacuumYaw,range,cone,1,0,Math.sin(this.vacuumPitch)*range);
        this.vacuumReticle.position.set(end.x,end.y,end.z);
        this.vacuumReticle.rotation.set(0,this.vacuumYaw,0);
        const tint=target&&(target as {blocked:boolean}).blocked?'#fb7185':view.blowing?'#fdba74':view.pulling.some(([,progress])=>progress>0)?'#fcd34d':'#a5f3fc';
        this.vacuumReticle.traverse(part=>{if(part instanceof THREE.Mesh && part.material.color.getHexString()!=='082f49')part.material.color.set(tint);});
      }
    }
    const streamMaterial=this.vacuumStream[0]?.material as THREE.MeshBasicMaterial|undefined;
    if(streamMaterial){streamMaterial.opacity=view?.on?0.85:0.32;streamMaterial.color.set(view?.blowing?'#ffedd5':'#a5f3fc');}
    this.vacuumStream.forEach((mesh,i)=>{
      mesh.visible=!!view;
      if(!view||!origin)return;
      const flow=(time*(view.on?0.65+(view.power??1):0.35)*rate+i/this.vacuumStream.length)%1;
      const distance=view.on&&!view.blowing?1-flow:flow;
      // Dedicated boundary particles mark the full valid cone. Interior rays
      // vary through it without ever widening beyond the server's hitbox.
      const spread=i%5===0?(i%2===0?-1:1):Math.sin(i*2.399+time*0.35);
      const lift=Math.sin(i*2.4+time)*Math.tan(cone)*range*distance*0.25;
      const at=vacuumPoint(origin,this.vacuumYaw,range,cone,distance,spread,lift);
      mesh.position.set(at.x,at.y,at.z);
      mesh.scale.setScalar(0.6+Math.sin(distance*Math.PI)*0.8);
    });
    // What it is drawing in creeps toward the nozzle, shaking as it comes;
    // blown, it is pushed away from it instead.
    const pulling = new Map(view?.pulling ?? []);
    const nozzle = token?.rig.group.position;
    const toward = nozzle && view?.blowing ? (() => {
      const away = new THREE.Vector3(Math.sin(this.vacuumYaw), 0, Math.cos(this.vacuumYaw)).multiplyScalar(VACUUM.range);
      return nozzle.clone().add(away);
    })() : nozzle;
    const tug = (model: THREE.Object3D | undefined, space: number) => {
      if (!model) return;
      if (model.userData.home === undefined) model.userData.home = model.position.clone();
      const home = model.userData.home as THREE.Vector3;
      const pull = pulling.get(space) ?? 0;
      model.position.copy(home);
      if (pull > 0 && toward) {
        model.position.lerp(toward.clone().setY(home.y), pull * 0.75);
        model.position.x += Math.sin(time * 40 + space) * 0.08 * pull;
        model.position.y += pull * 0.8;
      }
    };
    for (const [space, model] of this.crateModels) tug(model, pullKey(PULL.crate, space));
    for (const [space, model] of this.spills) tug(model, pullKey(PULL.loose, space));
    for (const [key, model] of this.specialModels) {
      tug(model, key);
      if (model.userData.bob) model.position.y += Math.sin(time * 1.6 + key) * 0.15;
      if (model.userData.glow) ((model as THREE.Mesh).material as THREE.MeshStandardMaterial).emissiveIntensity = 0.7 + Math.sin(time * 4 + key) * 0.3;
    }
    // The storm shudders while it is dragged.
    const dragged = pulling.get(pullKey(PULL.storm, 0)) ?? 0;
    if (this.storm && dragged > 0) this.storm.position.x += Math.sin(time * 30) * 0.15 * dragged;
    // The chest hangs off its spot and drifts while anyone's vacuum is out.
    if (this.prize && !this.prizeFlight) {
      const spot = this.positions.get(this.prizeAt);
      if (spot) {
        const [ox, oz] = view ? chestDrift(out) : [0, 0];
        const pull = pulling.get(pullKey(PULL.chest, this.prizeAt)) ?? 0;
        this.prize.position.x = spot.x + ox;
        this.prize.position.z = spot.z + oz;
        if (pull > 0 && toward) {
          this.prize.position.x += (toward.x - this.prize.position.x) * pull * 0.6 + Math.sin(time * 35) * 0.1 * pull;
          this.prize.position.z += (toward.z - this.prize.position.z) * pull * 0.6;
        }
      }
    }
  }

  /** Wood flying from a smashed crate or barricade. */
  private splinters(at: THREE.Vector3) {
    const floor = this.groundAt(at.x, at.z);
    const wood = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#8b5e3c', roughness: 0.9 }));
    for (let i = 0; i < 10; i++) {
      const mesh = new THREE.Mesh(this.chunkGeometry, wood);
      mesh.scale.set(0.4 + Math.random() * 0.4, 0.15, 1 + Math.random());
      mesh.position.copy(at).add(new THREE.Vector3(0, 0.6, 0));
      this.group.add(mesh);
      const angle = Math.random() * Math.PI * 2;
      this.bits.push({
        mesh,
        velocity: new THREE.Vector3(Math.cos(angle) * 3, 4 + Math.random() * 3, Math.sin(angle) * 3),
        spin: new THREE.Vector3(Math.random() * 9, Math.random() * 9, Math.random() * 9),
        life: 1.4,
        floor,
      });
    }
    this.sounds?.trap();
  }

  /**
   * A trap going off under someone: once their token gets there, the jaws
   * snap shut on them, they jolt, and the coins fly to whoever set it.
   */
  private springTrap(entry: BoardLogEntry) {
    const seat = Number(entry.seat);
    const token = this.tokens.get(seat);
    if (!token) return;
    const space = token.space;
    this.springing.add(space);
    this.arrivals.push({
      seat,
      wait: 8,
      run: () => {
        this.springing.delete(space);
        const mark = this.trapMarks.get(space);
        if (mark && !this.lastView?.traps.some((trap) => trap.space === space)) {
          mark.removeFromParent();
          this.trapMarks.delete(space);
        }
        const at = token.rig.group.position.clone();
        this.snap(at);
        token.shake = 0.6;
        this.float(at, `Trap! -${entry.coins}`, '#ff6b6b');
        this.coinFlow(at, this.seatAt(entry.owner), Number(entry.coins), 0.25);
        this.sounds?.trap();
      },
    });
  }

  private snap(at: THREE.Vector3) {
    const steel = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#94a3b8', metalness: 0.8, roughness: 0.35, transparent: true }));
    const group = new THREE.Group();
    group.position.set(at.x, this.groundAt(at.x, at.z) + 0.05, at.z);
    const jaws: THREE.Object3D[] = [];
    for (const side of [1, -1]) {
      // Each jaw is half a ring of teeth, hinged along the middle; open, it lies flat.
      const jaw = new THREE.Group();
      const arc = new THREE.Mesh(this.track(new THREE.TorusGeometry(0.62, 0.06, 6, 14, Math.PI)), steel);
      jaw.add(arc);
      for (let i = 1; i < 7; i++) {
        const angle = (i / 7) * Math.PI;
        const tooth = new THREE.Mesh(this.track(new THREE.ConeGeometry(0.07, 0.24, 4)), steel);
        tooth.position.set(Math.cos(angle) * 0.62, Math.sin(angle) * 0.62 - 0.12, 0);
        tooth.rotation.z = Math.PI;
        jaw.add(tooth);
      }
      jaw.rotation.y = side > 0 ? 0 : Math.PI;
      jaw.rotation.order = 'YXZ';
      group.add(jaw);
      jaws.push(jaw);
    }
    const plate = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.3, 0.3, 0.06, 12)), steel);
    group.add(plate);
    this.group.add(group);
    this.snaps.push({ group, jaws, t: 0 });
  }

  /** Cartoon blood flying from a hit, and a pool where it lands. Mature games only. */
  private splatter(at: THREE.Vector3, amount: number) {
    if (!this.mature) return;
    const floor = this.groundAt(at.x, at.z);
    for (let i = 0; i < amount; i++) {
      const mesh = new THREE.Mesh(this.bitGeometry, this.bloodMaterial);
      mesh.position.copy(at).add(new THREE.Vector3(0, 0.9, 0));
      mesh.scale.setScalar(0.7 + Math.random() * 1.1);
      this.group.add(mesh);
      const angle = Math.random() * Math.PI * 2;
      const out = 1.5 + Math.random() * 3.5;
      this.bits.push({
        mesh,
        velocity: new THREE.Vector3(Math.cos(angle) * out, 3 + Math.random() * 4, Math.sin(angle) * out),
        spin: new THREE.Vector3(),
        life: 2.5,
        floor,
      });
    }
    this.stain(at, 0.6 + amount * 0.04, '#7f0d16', 25);
  }

  /** A mark left lying on the ground, fading over `seconds`. */
  private stain(at: THREE.Vector3, radius: number, color: string, seconds: number) {
    const geometry = new THREE.CircleGeometry(radius, 14);
    const position = geometry.attributes.position as THREE.BufferAttribute;
    for (let i = 1; i < position.count; i++) {
      const ragged = 0.75 + Math.random() * 0.5;
      position.setXY(i, position.getX(i) * ragged, position.getY(i) * ragged);
    }
    const mesh = new THREE.Mesh(geometry, new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.85, depthWrite: false }));
    mesh.rotation.x = -Math.PI / 2;
    mesh.rotation.z = Math.random() * Math.PI * 2;
    mesh.position.set(at.x + (Math.random() - 0.5) * 0.6, this.groundAt(at.x, at.z) + 0.11, at.z + (Math.random() - 0.5) * 0.6);
    mesh.renderOrder = 3;
    this.group.add(mesh);
    this.stains.push({ mesh, life: seconds, max: seconds });
  }

  /** A knocked-out character bursting into pieces of itself, then putting itself back. */
  private burst(seat: number) {
    const token = this.tokens.get(seat);
    if (!token || !this.mature) return;
    const at = token.rig.group.position;
    token.burst = 1.6;
    token.rig.group.visible = false;
    const floor = this.groundAt(at.x, at.z);
    for (let i = 0; i < 12; i++) {
      const color = token.colors[i % token.colors.length];
      const mesh = new THREE.Mesh(this.chunkGeometry, new THREE.MeshStandardMaterial({ color, roughness: 0.6 }));
      mesh.position.copy(at).add(new THREE.Vector3((Math.random() - 0.5) * 0.6, 0.5 + Math.random() * 0.9, (Math.random() - 0.5) * 0.6));
      mesh.scale.setScalar(0.6 + Math.random() * 0.9);
      this.group.add(mesh);
      const angle = Math.random() * Math.PI * 2;
      const out = 2 + Math.random() * 4;
      this.bits.push({
        mesh,
        velocity: new THREE.Vector3(Math.cos(angle) * out, 5 + Math.random() * 5, Math.sin(angle) * out),
        spin: new THREE.Vector3(Math.random() * 8, Math.random() * 8, Math.random() * 8),
        life: 1.5,
        floor,
      });
    }
    this.splatter(at, 26);
  }

  /** The gore in flight, the stains fading, and the bounty and plague marks. */
  private updateGore(time: number, dt: number, view: { bounty?: number | null; plague?: number | null } | null) {
    for (let i = this.bits.length - 1; i >= 0; i--) {
      const bit = this.bits[i];
      bit.life -= dt;
      bit.velocity.y -= 22 * dt;
      bit.mesh.position.addScaledVector(bit.velocity, dt);
      bit.mesh.rotation.x += bit.spin.x * dt;
      bit.mesh.rotation.y += bit.spin.y * dt;
      if (bit.mesh.position.y < bit.floor + 0.1) {
        bit.mesh.position.y = bit.floor + 0.1;
        bit.velocity.set(0, 0, 0);
        bit.spin.set(0, 0, 0);
      }
      if (bit.life <= 0) {
        bit.mesh.removeFromParent();
        if (bit.mesh.material !== this.bloodMaterial) (bit.mesh.material as THREE.Material).dispose();
        this.bits.splice(i, 1);
      }
    }
    for (let i = this.stains.length - 1; i >= 0; i--) {
      const stain = this.stains[i];
      stain.life -= dt;
      (stain.mesh.material as THREE.MeshBasicMaterial).opacity = 0.85 * Math.min(1, stain.life / (stain.max * 0.3));
      if (stain.life <= 0) {
        stain.mesh.removeFromParent();
        stain.mesh.geometry.dispose();
        (stain.mesh.material as THREE.Material).dispose();
        this.stains.splice(i, 1);
      }
    }
    for (const token of this.tokens.values()) {
      if (token.burst <= 0) continue;
      token.burst -= dt;
      if (token.burst <= 0) token.rig.group.visible = true;
    }
    const marked = this.mature && view?.bounty != null ? this.tokens.get(view.bounty) : null;
    if (marked && !this.bountyMark) {
      this.bountyMark = new THREE.Mesh(
        this.track(new THREE.TorusGeometry(0.42, 0.06, 6, 24)),
        this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ef4444', depthTest: false, transparent: true })),
      );
      this.bountyMark.renderOrder = 24;
      this.group.add(this.bountyMark);
    }
    if (this.bountyMark) {
      this.bountyMark.visible = !!marked;
      if (marked) {
        this.bountyMark.position.copy(marked.rig.group.position).add(new THREE.Vector3(0, 3.1 + Math.sin(time * 3) * 0.1, 0));
        this.bountyMark.rotation.set(Math.PI / 2, 0, time * 2);
      }
    }
    const sick = this.mature && view?.plague != null ? this.tokens.get(view.plague) : null;
    if (sick && !this.fumes.length) {
      const material = this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#84cc16', transparent: true, opacity: 0.5, depthWrite: false }));
      const puff = this.track(new THREE.IcosahedronGeometry(0.22, 0));
      this.fumes = Array.from({ length: 7 }, () => {
        const mesh = new THREE.Mesh(puff, material);
        this.group.add(mesh);
        return mesh;
      });
    }
    this.fumes.forEach((mesh, i) => {
      mesh.visible = !!sick;
      if (!sick) return;
      const t = (time * 0.5 + i / this.fumes.length) % 1;
      const angle = i * 2.4 + time;
      mesh.position.copy(sick.rig.group.position).add(new THREE.Vector3(Math.cos(angle) * 0.5, 0.4 + t * 2.4, Math.sin(angle) * 0.5));
      mesh.scale.setScalar(0.6 + t * 1.4);
    });
  }

  private blast(center: THREE.Vector3) {
    // A fireball with a white-hot core, a flash that lights the ground round
    // it, smoke rolling up, and dirt thrown out.
    const fire = new THREE.Mesh(
      new THREE.SphereGeometry(0.6, 16, 12),
      new THREE.MeshBasicMaterial({ color: '#fb923c', transparent: true, opacity: 0.95, depthWrite: false }),
    );
    fire.position.copy(center).setY(center.y + 0.6);
    this.group.add(fire);
    this.effects.push({ object: fire, life: 0.45, max: 0.45, grow: 3.2 });
    const core = new THREE.Mesh(
      new THREE.SphereGeometry(0.45, 12, 10),
      new THREE.MeshBasicMaterial({ color: '#fef3c7', transparent: true, opacity: 1, depthWrite: false }),
    );
    core.position.copy(fire.position);
    this.group.add(core);
    this.effects.push({ object: core, life: 0.22, max: 0.22, grow: 2.4 });
    for (let i = 0; i < 7; i++) {
      const puff = new THREE.Mesh(
        new THREE.SphereGeometry(0.5 + Math.random() * 0.3, 10, 8),
        new THREE.MeshStandardMaterial({ color: '#57534e', transparent: true, opacity: 0.7, depthWrite: false, roughness: 1 }),
      );
      const angle = (i / 7) * Math.PI * 2;
      puff.position.copy(center).add(new THREE.Vector3(Math.cos(angle) * 0.8, 0.5 + Math.random() * 0.6, Math.sin(angle) * 0.8));
      this.group.add(puff);
      this.effects.push({ object: puff, life: 1.6, max: 1.6, grow: 1.8, rise: 1.3 + Math.random() });
    }
    const flash = new THREE.PointLight('#fdba74', 60, 14, 2);
    flash.position.copy(center).setY(center.y + 1.2);
    this.group.add(flash);
    const born = performance.now();
    const dim = () => {
      const t = (performance.now() - born) / 350;
      if (t >= 1) {
        flash.removeFromParent();
        flash.dispose();
        return;
      }
      flash.intensity = 60 * (1 - t);
      requestAnimationFrame(dim);
    };
    requestAnimationFrame(dim);
    const dirt = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#78543a', roughness: 1 }));
    const floor = this.groundAt(center.x, center.z);
    for (let i = 0; i < 12; i++) {
      const mesh = new THREE.Mesh(this.chunkGeometry, dirt);
      mesh.scale.setScalar(0.4 + Math.random() * 0.5);
      mesh.position.copy(center).add(new THREE.Vector3(0, 0.4, 0));
      this.group.add(mesh);
      const angle = Math.random() * Math.PI * 2;
      const speed = 2.5 + Math.random() * 3;
      this.bits.push({
        mesh,
        velocity: new THREE.Vector3(Math.cos(angle) * speed, 5 + Math.random() * 4, Math.sin(angle) * speed),
        spin: new THREE.Vector3(Math.random() * 9, Math.random() * 9, Math.random() * 9),
        life: 1.4,
        floor,
      });
    }
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(0.5, 0.75, 40),
      new THREE.MeshBasicMaterial({ color: '#ff7849', transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.copy(center).setY(center.y + 0.3);
    this.group.add(ring);
    this.effects.push({ object: ring, life: 0.9, max: 0.9, grow: 7 });
  }

  private pulse(at: THREE.Vector3, color: string) {
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(0.45, 0.6, 40),
      new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.copy(at).setY(at.y + 0.3);
    this.group.add(ring);
    this.effects.push({ object: ring, life: 0.7, max: 0.7, grow: 3 });
  }

  private beam(from: THREE.Vector3, to: THREE.Vector3, color = '#22d3ee', seconds = 0.9) {
    const geometry = new THREE.BufferGeometry().setFromPoints([
      from.clone().setY(from.y + 1),
      to.clone().setY(to.y + 1),
    ]);
    const line = new THREE.Line(geometry, new THREE.LineBasicMaterial({ color, transparent: true, opacity: 1 }));
    this.group.add(line);
    this.effects.push({ object: line, life: seconds, max: seconds, grow: 0 });
  }

  private disposeTag(token: TokenView) {
    token.tag.material.map?.dispose();
    token.tag.material.dispose();
    token.tag.removeFromParent();
  }

  /**
   * Knocked out: a K.O.! stamped over them, the coins it cost said under it,
   * a close look, and they go down flat with stars round their head until
   * they are back up.
   */
  private knockOut(entry: BoardLogEntry, at: THREE.Vector3) {
    this.stamp(at, 'K.O.!');
    const coins = Number(entry.coins) || 0;
    if (coins > 0) this.float(at, `-${coins} coins`, '#ff6b6b', 2.2);
    const token = this.tokens.get(Number(entry.seat));
    if (token) {
      token.shake = 0.6;
      this.watch(at, { yaw: this.angleBetween(at, this.bounds().center), pitch: 26, wide: 6, life: 1.8 });
    }
  }

  private stamp(at: THREE.Vector3, text: string) {
    const canvas = document.createElement('canvas');
    canvas.width = 512;
    canvas.height = 220;
    const ctx = canvas.getContext('2d')!;
    ctx.translate(256, 112);
    ctx.rotate(-0.12);
    ctx.font = 'italic 900 150px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.lineJoin = 'round';
    ctx.lineWidth = 26;
    ctx.strokeStyle = '#1e1b4b';
    ctx.strokeText(text, 0, 0);
    const fill = ctx.createLinearGradient(0, -60, 0, 60);
    fill.addColorStop(0, '#fde047');
    fill.addColorStop(0.55, '#fb923c');
    fill.addColorStop(1, '#e11d48');
    ctx.fillStyle = fill;
    ctx.fillText(text, 0, 0);
    ctx.lineWidth = 5;
    ctx.strokeStyle = '#fff7ed';
    ctx.strokeText(text, 0, -4);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false, sizeAttenuation: false }));
    sprite.position.copy(at).add(new THREE.Vector3(0, 1.5, 0));
    sprite.renderOrder = 31;
    this.group.add(sprite);
    this.stamps.push({ sprite, age: 0, life: 1.7, size: 0.16 });
  }

  /** Stamps pop in big and settle, then fade out. */
  private updateStamps(dt: number) {
    for (let i = this.stamps.length - 1; i >= 0; i--) {
      const stamp = this.stamps[i];
      stamp.age += dt;
      const pop = stamp.age < 0.18 ? 1.9 - (stamp.age / 0.18) * 0.9 : 1 + Math.max(0, 0.06 * Math.sin((stamp.age - 0.18) * 30) * Math.exp(-(stamp.age - 0.18) * 8));
      const size = stamp.size * pop;
      stamp.sprite.scale.set(size * (512 / 220), size, 1);
      const material = stamp.sprite.material;
      material.opacity = Math.min(1, stamp.age / 0.08) * Math.min(1, (stamp.life - stamp.age) / 0.35);
      if (stamp.age >= stamp.life) {
        stamp.sprite.removeFromParent();
        material.map?.dispose();
        material.dispose();
        this.stamps.splice(i, 1);
      }
    }
  }

  /** Lying down while knocked out, with stars circling, and getting back up after. */
  private updateDowned(token: TokenView, time: number, dt: number) {
    const status = token.tag.userData.token as BoardToken | undefined;
    const out = (status?.resting ?? 0) > 0 && token.path.length === 0 && token.burst <= 0;
    let down = this.downed.get(token.seat);
    if (!down && !out) return;
    if (!down) {
      const stars = new THREE.Group();
      const material = this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#fde047' }));
      const geometry = this.track(new THREE.OctahedronGeometry(0.2, 0));
      for (let i = 0; i < 5; i++) stars.add(new THREE.Mesh(geometry, material));
      this.group.add(stars);
      down = { amount: 0, stars };
      this.downed.set(token.seat, down);
    }
    down.amount += ((out ? 1 : 0) - down.amount) * (1 - Math.exp(-dt * (out ? 9 : 4)));
    // Over backwards, as if the hit landed square.
    token.rig.group.rotation.x = -down.amount * 1.45;
    const head = token.rig.group.position.clone().add(new THREE.Vector3(Math.sin(token.yaw) * -0.9 * down.amount, 0.35 + (1 - down.amount) * 1.1, Math.cos(token.yaw) * -0.9 * down.amount));
    down.stars.position.copy(head);
    down.stars.visible = down.amount > 0.5;
    down.stars.children.forEach((star, i) => {
      const a = time * 3.2 + (i / 5) * Math.PI * 2;
      star.position.set(Math.cos(a) * 0.6, 0.35 + Math.sin(a * 2) * 0.08, Math.sin(a) * 0.6);
      star.rotation.y = time * 6;
    });
    if (!out && down.amount < 0.01) {
      token.rig.group.rotation.x = 0;
      down.stars.removeFromParent();
      this.downed.delete(token.seat);
    }
  }

  private bagOf(token: TokenView) {
    let bag = this.bags.get(token.seat);
    if (bag && (bag.style !== token.bagStyle || bag.tint !== token.bagTint)) {
      bag.built.dispose();
      bag.pivot.removeFromParent();
      bag.strap.removeFromParent();
      bag = undefined;
    }
    if (!bag) {
      this.strapGeometry ??= this.track(new THREE.CylinderGeometry(0.03, 0.03, 1, 6).translate(0, 0.5, 0));
      const built = buildBag(token.bagStyle, token.bagTint);
      const pivot = new THREE.Group();
      pivot.name = 'item-bag';
      const sack = new THREE.Group();
      sack.add(built.group);
      pivot.add(sack);
      bag = { pivot, sack, strap: new THREE.Mesh(this.strapGeometry, built.strap), built, style: token.bagStyle, tint: token.bagTint, held: 0, size: BAG_SIZES[0], pop: 0 };
      pivot.userData.carried = bag.strap.userData.carried = true;
      this.bags.set(token.seat, bag);
    }
    // A new look for the character is a new rig, so move the sack onto it.
    if (bag.pivot.parent !== token.rig.group) token.rig.group.add(bag.pivot, bag.strap);
    return bag;
  }

  /** The sack swells or shrinks to what is in it, swings as they walk and wobbles as things land. */
  private updateBag(token: TokenView, time: number, dt: number, speed: number) {
    const bag = this.bagOf(token);
    const shown = Math.max(0, bag.held - (this.bagIncoming.get(token.seat) ?? 0));
    bag.size += (BAG_SIZES[shown] - bag.size) * (1 - Math.exp(-dt * 6));
    bag.pop = Math.max(0, bag.pop - dt * 1.8);
    const jelly = Math.sin(bag.pop * 22) * bag.pop * 0.3;
    const size = bag.size;
    bag.sack.scale.set(size * (1 + jelly), size * (1 - jelly), size * (1 + jelly));
    const walk = Math.min(1, speed / 3);
    // Down at the right side, so it shows from the front, and just off the
    // ground however big it gets; it leans out and drags a little under the weight.
    bag.pivot.position.set(-(0.42 + 0.27 * size), 0.04 + Math.abs(Math.sin(time * 9)) * walk * 0.06, -0.05 - 0.05 * size);
    bag.pivot.rotation.set(Math.sin(time * 9) * walk * 0.07, 0, -0.08 - 0.03 * size + Math.sin(time * 4.5) * walk * 0.09);
    const lying = (this.downed.get(token.seat)?.amount ?? 0) > 0.3;
    bag.pivot.visible = bag.strap.visible = !lying;
    // The strap runs from the shoulder to the sack's neck.
    bag.pivot.updateMatrix();
    bag.sack.updateMatrix();
    const neck = this.bagScratch.set(0, bag.built.mouth - 0.08, 0).applyMatrix4(bag.sack.matrix).applyMatrix4(bag.pivot.matrix);
    const along = neck.sub(BAG_SHOULDER);
    const length = along.length();
    bag.strap.position.copy(BAG_SHOULDER);
    bag.strap.quaternion.setFromUnitVectors(THREE.Object3D.DEFAULT_UP, along.divideScalar(length || 1));
    bag.strap.scale.set(1, length, 1);
  }
  private bagScratch = new THREE.Vector3();

  /** Where a seat's sack opens, in the board's space. */
  private bagMouth(seat: number) {
    const token = this.tokens.get(seat);
    const bag = this.bags.get(seat);
    if (!token || !bag || !bag.pivot.visible) return token ? token.rig.group.position.clone().add(new THREE.Vector3(0, 1.2, 0)) : null;
    bag.sack.updateWorldMatrix(true, false);
    return this.group.worldToLocal(bag.sack.localToWorld(new THREE.Vector3(0, bag.built.mouth, 0)));
  }

  private badge(item: Item) {
    let texture = this.badges.get(item);
    if (!texture) this.badges.set(item, (texture = badgeTexture(item)));
    return texture;
  }

  /** An item won or picked up: it pops out where it came from, or out of the sky, and drops into the sack. */
  private bagItem(seat: unknown, item: unknown, from: THREE.Vector3 | null = null, delay = 0) {
    const token = this.tokens.get(Number(seat));
    if (!token || !ITEMS[item as Item]) return;
    const start = from?.clone() ?? token.rig.group.position.clone().add(new THREE.Vector3(0, 2.0, 0));
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: this.badge(item as Item), transparent: true, depthTest: false }));
    sprite.renderOrder = 29;
    sprite.scale.setScalar(0);
    sprite.position.copy(start);
    this.group.add(sprite);
    this.bagIncoming.set(token.seat, (this.bagIncoming.get(token.seat) ?? 0) + 1);
    this.bagFlights.push({ sprite, seat: token.seat, start, age: -delay, life: 1.25, out: false });
  }

  /** The oldest item shoved out of a full sack to make room. */
  private bagOut(seat: unknown, item: unknown) {
    const start = this.bagMouth(Number(seat));
    if (!start || !ITEMS[item as Item]) return;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: this.badge(item as Item), transparent: true, depthTest: false }));
    sprite.renderOrder = 29;
    sprite.scale.setScalar(0);
    this.group.add(sprite);
    this.bagFlights.push({ sprite, seat: Number(seat), start, age: -1.1, life: 1, out: true });
  }

  private updateBagFlights(dt: number) {
    for (let i = this.bagFlights.length - 1; i >= 0; i--) {
      const flight = this.bagFlights[i];
      flight.age += dt;
      const t = Math.min(1, Math.max(0, flight.age) / flight.life);
      const { sprite } = flight;
      const material = sprite.material;
      if (flight.age < 0) {
        sprite.visible = false;
      } else if (flight.out) {
        // Popped out over the shoulder, tumbling away and fading.
        sprite.visible = true;
        sprite.position.copy(flight.start).add(this.bagScratch.set(Math.sin(flight.seat) * t * 1.2, t * 2.2 - t * t * 2.6, Math.cos(flight.seat) * t * 1.2));
        sprite.scale.setScalar(0.7 * Math.min(1, t * 6));
        material.rotation = t * 7;
        material.opacity = 1 - t;
      } else {
        sprite.visible = true;
        const mouth = this.bagMouth(flight.seat);
        if (!mouth) flight.age = flight.life;
        else {
          // A beat to see what it is, turning where it appeared, then down into the sack.
          const hold = 0.38;
          if (t < hold) {
            const k = t / hold;
            sprite.position.copy(flight.start).add(this.bagScratch.set(0, Math.sin(k * Math.PI * 0.5) * 0.35, 0));
            sprite.scale.setScalar(0.9 * (k < 0.5 ? 1.25 * Math.sin(k * Math.PI) : 1));
            material.rotation = Math.sin(k * Math.PI * 2) * 0.35;
          } else {
            const k = (t - hold) / (1 - hold);
            const eased = k * k;
            const lifted = flight.start.clone().add(this.bagScratch.set(0, 0.35, 0));
            sprite.position.lerpVectors(lifted, mouth, eased);
            sprite.position.y += Math.sin(k * Math.PI) * 0.7;
            sprite.scale.setScalar(0.9 - 0.65 * eased);
            material.rotation = k * Math.PI * 2;
          }
        }
      }
      if (flight.age < flight.life) continue;
      sprite.removeFromParent();
      material.dispose();
      this.bagFlights.splice(i, 1);
      if (flight.out) continue;
      this.bagIncoming.set(flight.seat, Math.max(0, (this.bagIncoming.get(flight.seat) ?? 1) - 1));
      const bag = this.bags.get(flight.seat);
      if (bag) {
        bag.pop = 1;
        this.sounds?.bagIn(bag.held);
      }
    }
  }

  private float(at: THREE.Vector3, text: string, color: string, seconds = 1.6) {
    const texture = labelTexture(text, color);
    // The same size on screen however far off it happens, so it can always be read.
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false, sizeAttenuation: false }));
    const image = texture.image as HTMLCanvasElement;
    sprite.scale.set((image.width / image.height) * 0.065, 0.065, 1);
    // Several lines for one player at once stack instead of overlapping.
    const stacked = this.floaters.filter(
      // Near enough to overlap once drawn at screen size: tokens sharing a space are a step apart.
      (f) => f.life > 1.1 && Math.abs(f.sprite.position.x - at.x) < 2.5 && Math.abs(f.sprite.position.z - at.z) < 2.5,
    ).length;
    // Just over the hearts, so a close camera still has it in shot.
    sprite.position.copy(at).add(new THREE.Vector3(0, 2.5, 0));
    // Stacked on screen, since that is the size it is drawn at.
    sprite.center.set(0.5, -stacked * 1.1);
    sprite.renderOrder = 30;
    this.group.add(sprite);
    this.floaters.push({ sprite, life: seconds, rise: 0.9 });
  }

  /** Recoil and the camera use a local 20% impact clock, never room time. */
  private spikeImpact(entry:BoardLogEntry) {
    const at=this.positions.get(entry.space)?.clone();
    const token=this.tokens.get(entry.seat);
    if(!at || !token)return;
    at.y+=0.1;
    this.spikeHits.set(entry.seat,{at,elapsed:0,impact:false,coins:Number(entry.coins)||0,hurt:[]});
    token.path=[];token.from=at.clone();token.to=at.clone();token.hop=1;token.space=entry.space;token.wait=0;
    this.watch(at,{yaw:token.yaw,pitch:30,wide:7,life:SPIKE_IMPACT_SECONDS+0.25});
    if(this.cinema)this.cinema.sweep=true;
  }

  update(time: number, dt: number) {
    const liveDt=dt;
    if(this.lastView?.candy_van)dt=0;
    this.finale.update(time,dt);
    this.crawler.update(dt);
    this.traffic.update(dt);
    this.rental.update(dt);
    this.candyVan.update(dt);
    if(this.options.city){this.pedestrians.update(time,dt,[]);for(const step of this.animators)step(time);for(const label of this.districtLabels)label.visible=false;return;}
    // District signs are for reading the board; keep the action and sky clear
    // during a close-up or a first-person item/walking view.
    for(const label of this.districtLabels)label.visible=!this.cinema && !this.clear && !this.closeCamera;
    this.updateGore(time, dt, this.lastView);
    this.updateStamps(liveDt);
    this.updateBagFlights(liveDt);
    this.updateVacuum(time);
    for (let i = this.coinsInFlight.length - 1; i >= 0; i--) {
      const coin = this.coinsInFlight[i];
      coin.t += dt / coin.seconds;
      if (coin.t < 0) continue;
      const t = Math.min(1, coin.t);
      coin.mesh.visible = true;
      // Out quick and settling in: eased, so the catch reads.
      const eased = 1 - (1 - t) * (1 - t);
      const arc = Math.sin(t * Math.PI);
      coin.mesh.position.copy(coin.from).lerp(coin.to, eased).addScaledVector(coin.side, arc);
      coin.mesh.position.y += arc * coin.height;
      // Flipped end over end, each at its own rate, catching the light as it turns.
      coin.mesh.rotation.set(coin.phase + coin.t * coin.flip * Math.PI * 2, coin.yaw, 0.3, 'YXZ');
      if (t >= 1) {
        coin.mesh.removeFromParent();
        this.sounds?.coinLand(coin.index, coin.last);
        if (coin.last) this.pulse(coin.to.clone().add(new THREE.Vector3(0, -1.2, 0)), '#fde047');
        this.coinsInFlight.splice(i, 1);
      }
    }
    for (let i = this.flights.length - 1; i >= 0; i--) {
      const flight = this.flights[i];
      flight.t = Math.min(1, flight.t + dt / flight.seconds);
      const travel=flight.slowCatchFlight===undefined?flight.t:grappleTravelProgress(flight.t,flight.slowCatchFlight);
      const arc = flight.from.distanceTo(flight.to) * 0.35 + 3;
      flight.bomb.position.copy(flight.from).lerp(flight.to, travel);
      flight.bomb.position.y += Math.sin(travel * Math.PI) * arc;
      flight.bomb.rotation.x += dt * 7 * (flight.slowCatchFlight!==undefined && travel>0.72?0.25:1);
      if (flight.rope) {
        if (!flight.line) {
          flight.line = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial({ color: '#e2e8f0' }));
          flight.line.frustumCulled = false;
          this.group.add(flight.line);
        }
        flight.line.geometry.setFromPoints([flight.from, flight.bomb.position]);
      }
      if (flight.t >= 1) {
        flight.bomb.removeFromParent();
        if (flight.line) {
          flight.line.removeFromParent();
          flight.line.geometry.dispose();
          (flight.line.material as THREE.Material).dispose();
        }
        this.landing = { at: flight.to.clone(), time: performance.now() };
        flight.land();
        this.flights.splice(i, 1);
      }
    }
    if (this.cinema) {
      this.cinema.life -= dt;
      const rollWalk=this.cinema.rollSeat!==undefined&&this.lastView?.walk?.seat===this.cinema.rollSeat?this.lastView.walk:null;
      const elapsed=this.walkElapsed()??0;
      // Keep the roll close-up until the SAME presentation clock starts the
      // walker; never back out to the board between the two camera views.
      if(rollWalk ? elapsed>=rollWalk.delay : this.cinema.life<=0)this.cinema=null;
    }
    if (this.shopping !== null && (this.cinema?.life ?? 0) < 0.6) this.lookIntoShop();
    for (const [space, shop] of this.shops) {
      shop.keeper.animate(space === this.shopping ? 'cheer' : 'idle', time + space, 0);
    }
    for (const die of this.dice) die.update(dt);
    for (const spinner of this.spinners) spinner.rotation.y = time * 1.6 + spinner.position.x;
    for (const step of this.animators) step(time);
    const rider = this.riding === null ? undefined : this.tokens.get(this.riding);
    if (rider) {
      const at = rider.rig.group.position;
      const landed = rider.hop >= 1 && rider.path.length === 0;
      this.watch(at, { yaw: this.angleBetween(rider.from, rider.to), pitch: 26, wide: 9, life: landed ? 1.4 : 0.6 });
      if (landed) this.riding = null;
    }
    // Taking a skill check: the camera comes in close on them for it.
    const checking = this.lastView?.turn.step === 'check' ? this.tokens.get(this.lastView.turn_seat) : undefined;
    if (checking) {
      const at = checking.rig.group.position;
      this.watch(at, { yaw: this.angleBetween(at, this.bounds().center), pitch: 22, wide: 5, life: 0.6 });
    }
    for (const token of this.tokens.values()) {
      const shielded = !!this.lastView?.tokens.find((one) => one.seat === token.seat)?.shield;
      let bubble = this.bubbles.get(token.seat);
      if (shielded && !bubble) {
        bubble = new THREE.Mesh(
          this.track(new THREE.SphereGeometry(1, 24, 16)),
          this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#93c5fd', emissive: '#3b82f6', emissiveIntensity: 0.4, transparent: true, opacity: 0.25, depthWrite: false })),
        );
        this.group.add(bubble);
        this.bubbles.set(token.seat, bubble);
      }
      if (bubble) {
        bubble.visible = shielded;
        bubble.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 0.85, 0));
        bubble.scale.setScalar(1 + Math.sin(time * 3 + token.seat) * 0.04);
      }
    }
    for (let i = this.arrivals.length - 1; i >= 0; i--) {
      const arrival = this.arrivals[i];
      const token = this.tokens.get(arrival.seat);
      arrival.wait -= dt;
      if (token && token.path.length === 0 && token.hop >= 1 && token.wait <= 0 || arrival.wait <= 0) {
        this.arrivals.splice(i, 1);
        arrival.run();
      }
    }
    for (let i = this.snaps.length - 1; i >= 0; i--) {
      const snap = this.snaps[i];
      snap.t += dt;
      // Flat open, then shut in a tenth of a second; a moment to see it, then gone.
      const shut = Math.min(1, snap.t / 0.1);
      for (const jaw of snap.jaws) jaw.rotation.x = -(1 - shut) * (Math.PI / 2);
      const fade = THREE.MathUtils.clamp((snap.t - 1.2) / 0.5, 0, 1);
      for (const jaw of snap.jaws) {
        jaw.traverse((part) => {
          if (part instanceof THREE.Mesh) (part.material as THREE.MeshStandardMaterial).opacity = 1 - fade;
        });
      }
      if (fade >= 1) {
        snap.group.removeFromParent();
        this.snaps.splice(i, 1);
      }
    }
    for (const token of this.tokens.values()) {
      const tokenDt=this.lastView?.candy_runs?.some(run=>run.seat===token.seat)?liveDt:dt;
      let animation:Parameters<CharacterRig['animate']>=['idle',time+token.seat*.4,0];
      const queueAnimation=(...pose:Parameters<CharacterRig['animate']>)=>{animation=pose;};
      if (token.wait > 0) {
        token.wait = Math.max(0, token.wait - tokenDt);
      } else if (token.hop >= 1 && token.path.length) {
        token.from = token.rig.group.position.clone();
        token.to = token.path.shift()!;
        token.hop = 0;
        if (token.flying) this.sounds?.cannon();
        else this.sounds?.hop(this.groundUnder(token.to));
        token.yaw = Math.atan2(token.to.x - token.from.x, token.to.z - token.from.z);
      }
      if (token.hop < 1) {
        token.hop = Math.min(1, token.hop + tokenDt / (token.flying ? FLIGHT_SECONDS : HOP_SECONDS));
        const position = token.from.clone().lerp(token.to, token.hop);
        // The hop arcs over whatever the ground does between the two spaces,
        // and a cannon shot arcs a great deal higher.
        position.y += Math.sin(token.hop * Math.PI) * (token.flying ? 9 : 0.7);
        token.rig.group.position.copy(position);
        token.rig.group.rotation.y = token.yaw;
        queueAnimation('ride', time + token.seat, 6);
      } else {
        token.rig.group.position.copy(token.to);
        if (token.flying) this.sounds?.land();
        token.flying = false;
        queueAnimation(this.prizeWinner?.seat === token.seat ? 'cheer' : 'idle', time + token.seat * 0.4, 0);
      }
      const walk=this.lastView?.walk;
      if (walk && walk.seat===token.seat) {
        const elapsed=this.walkElapsed()??walk.elapsed;
        const motion=Math.max(0,(elapsed-walk.delay)/walk.hop_seconds);
        const segment=Math.min(walk.path.length-1,Math.floor(motion));
        const t=THREE.MathUtils.clamp(motion-segment,0,1);
        const fromId=segment===0?walk.from:walk.path[segment-1];
        const toId=walk.path[segment];
        const from=this.positions.get(fromId);
        const to=this.positions.get(toId);
        if (from && to) {
          const portal=this.lastView?.spaces.find((s)=>s.id===fromId)?.link===toId;
          const at=portal?(elapsed<=walk.delay?from.clone():to.clone()):from.clone().lerp(to,t);
          if ((walk.waiting??0)>0) at.y=this.groundAt(at.x,at.z);
          else if (!portal) at.y+=Math.sin(t*Math.PI)*0.7;
          token.rig.group.position.copy(at);
          token.rig.group.rotation.y=Math.atan2(to.x-from.x,to.z-from.z);
          token.from.copy(at);token.to.copy(at);token.path.length=0;token.wait=0;token.hop=1;token.flying=false;
          queueAnimation(this.lastView?.candy_van||elapsed<walk.delay || (walk.waiting??0)>0?'idle':'ride',time+token.seat,elapsed<walk.delay || (walk.waiting??0)>0?0:6);
          const mark=`${walk.id}:${segment}`;
          if (elapsed>=walk.delay && !(walk.waiting??0) && token.rig.group.userData.walkHop!==mark) {
            token.rig.group.userData.walkHop=mark;
            this.sounds?.hop(this.groundUnder(at));
          }
        }
      }
      const placement = token.positioning;
      if (placement && !this.stormFlights.has(token.seat) && !this.spikeHits.has(token.seat)) {
        const oldX = placement.at.x, oldZ = placement.at.z;
        if (placement.returning) {
          placement.elapsed += tokenDt;
          const t = THREE.MathUtils.clamp(placement.elapsed / 0.52, 0, 1);
          const eased = t * t * (3 - 2 * t);
          placement.at.lerpVectors(placement.from, placement.target, eased);
          // Follow terrain between the actual cast point and the home tile.
          const fromLift = placement.from.y - this.groundAt(placement.from.x, placement.from.z);
          const toLift = placement.target.y - this.groundAt(placement.target.x, placement.target.z);
          placement.at.y = this.groundAt(placement.at.x, placement.at.z) + THREE.MathUtils.lerp(fromLift, toLift, eased);
          if (t >= 1) {
            placement.at.copy(placement.target);
            token.from.copy(placement.target); token.to.copy(placement.target);
            token.hop = 1; token.path.length = 0; token.wait = 0;
            token.positioning = null;
          }
        } else {
          placement.at.lerp(placement.target, 1 - Math.exp(-tokenDt * 18));
          placement.at.y = this.groundAt(placement.at.x, placement.at.z) + 0.1;
        }
        const dx = placement.at.x - oldX, dz = placement.at.z - oldZ;
        const speed = Math.hypot(dx, dz) / Math.max(0.001, tokenDt);
        token.rig.group.position.copy(placement.at);
        if (speed > 0.08) {
          const yaw = Math.atan2(dx, dz);
          const delta = Math.atan2(Math.sin(yaw-token.rig.group.rotation.y),Math.cos(yaw-token.rig.group.rotation.y));
          token.rig.group.rotation.y += delta * (1 - Math.exp(-tokenDt * 14));
        } else if (placement.active) {
          const facing = this.lastView?.tokens.find(state => state.seat === token.seat)?.facing;
          if (typeof facing === 'number') {
            const delta = Math.atan2(Math.sin(facing-token.rig.group.rotation.y),Math.cos(facing-token.rig.group.rotation.y));
            token.rig.group.rotation.y += delta * (1 - Math.exp(-tokenDt * 14));
          }
        }
        queueAnimation(!placement.active && speed > 0.08 ? 'ride' : 'idle', time + token.seat, Math.min(6, speed));
      } else if (placement) token.positioning = null;
      if (!token.positioning && (!walk || walk.seat !== token.seat) && token.hop >= 1 && token.path.length === 0) {
        const predicted = this.localFacing?.seat === token.seat && performance.now() - this.localFacing.at < 400 ? this.localFacing.yaw : null;
        const facing = predicted ?? this.lastView?.tokens.find(view => view.seat === token.seat)?.facing;
        if (typeof facing === 'number' && Number.isFinite(facing)) {
          const delta = Math.atan2(Math.sin(facing-token.rig.group.rotation.y),Math.cos(facing-token.rig.group.rotation.y));
          token.rig.group.rotation.y += delta * (1-Math.exp(-tokenDt*14));
        }
      }
      const finale=this.finale.frame(token.seat);
      if(finale){
        token.rig.group.position.copy(finale.at);
        token.rig.group.rotation.y=finale.yaw;
        queueAnimation(finale.pose,time+token.seat,finale.speed);
      }
      const storm=this.stormFlights.get(token.seat);
      if(storm) {
        storm.elapsed+=tokenDt;
        const t=THREE.MathUtils.clamp((storm.elapsed-0.45)/1.1,0,1);
        const eased=t*t*(3-2*t);
        const at=storm.from.clone().lerp(storm.to,eased);
        at.y+=storm.elapsed<0.45?Math.sin(storm.elapsed/0.45*Math.PI/2)*2:Math.sin(t*Math.PI)*3.8+(1-t)*2;
        token.rig.group.position.copy(at);
        token.rig.group.rotation.y=token.yaw+storm.elapsed*9*(1-t);
        queueAnimation('ride',time+token.seat,6);
        if(t>=1) {
          token.from.copy(storm.to);token.to.copy(storm.to);token.hop=1;
          token.rig.group.position.copy(storm.to);token.rig.group.rotation.y=token.yaw;
          this.stormFlights.delete(token.seat);
          this.sounds?.land();
        }
      }
      const driver=this.lastView?.driving?.find(d=>d.seat===token.seat);
      if(driver){const at=this.rental.seat(token.seat);if(at){token.rig.group.position.copy(at);token.rig.group.position.y+=.22;token.rig.group.rotation.y=driver.yaw;queueAnimation('sit',time+token.seat,0);token.rig.group.userData.seated=true;}}
      else if(token.rig.group.userData.seated){delete token.rig.group.userData.seated;}
      const spike=this.spikeHits.get(token.seat);
      if(spike) {
        spike.elapsed+=tokenDt;
        const hitAge=Math.max(0,spike.elapsed-SPIKE_SWEEP_SECONDS);
        const recoil=Math.sin(Math.min(1,hitAge/1.3)*Math.PI);
        token.rig.group.position.copy(spike.at);
        token.rig.group.position.y+=spike.elapsed<SPIKE_SWEEP_SECONDS?(1-impactSweep(spike.elapsed))*0.45:recoil*0.28;
        token.rig.group.rotation.z=-recoil*0.16;
        queueAnimation('ride',spikeImpactClock(spike.elapsed)+token.seat,3);
        if(!spike.impact && spike.elapsed>=SPIKE_SWEEP_SECONDS) {
          spike.impact=true;
          this.pulse(spike.at,'#fb7185');
          this.float(spike.at,`Spikes! −${spike.coins} coins`,'#fb7185',2.6);
          for(const entry of spike.hurt){this.playEffect(entry,[]);this.hear(entry);}
        }
        if(spike.elapsed>=SPIKE_IMPACT_SECONDS) {
          token.rig.group.rotation.z=0;
          this.spikeHits.delete(token.seat);
        }
      } else token.rig.group.rotation.z=0;
      if (token.shake > 0) {
        token.shake = Math.max(0, token.shake - tokenDt);
        token.rig.group.position.x += Math.sin(time * 60) * 0.12 * (token.shake / 0.5);
      }
      if (this.held?.seat === token.seat) token.rig.group.position.copy(this.held.at);
      const pair = this.pairs.find((one) => one.seat === token.seat);
      let ring = this.pairRings.get(token.seat);
      if (pair && !ring) {
        ring = new THREE.Mesh(
          this.track(new THREE.RingGeometry(0.42, 0.56, 28)),
          this.trackMaterial(new THREE.MeshBasicMaterial({ color: PAIR_COLORS[pair.team % PAIR_COLORS.length], transparent: true, opacity: 0.85, depthWrite: false })),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.renderOrder = 18;
        this.group.add(ring);
        this.pairRings.set(token.seat, ring);
      }
      if (ring) {
        ring.visible = !!pair;
        ring.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 0.1, 0));
      }
      if (this.aimer?.seat === token.seat && token.hop >= 1) {
        token.rig.group.rotation.y = this.aimer.yaw;
        if (this.aimerMark) {
          const pulse = 1 + Math.sin(time * 6) * 0.08;
          this.aimerMark.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 0.12, 0));
          this.aimerMark.rotation.y = this.aimer.yaw;
          this.aimerMark.scale.setScalar(pulse);
        }
      }
      token.rig.animate(...animation);
      this.updateDowned(token, time, dt);
      this.updateBag(token, time, dt, animation[2]);
      const swinging = this.bats.get(token.seat);
      if (this.batHolder === token.seat || swinging?.swing != null) {
        const bat = swinging ?? this.batOf(token.seat);
        bat.pivot.visible = true;
        bat.pivot.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 1.0, 0));
        if (bat.swing === null) {
          // Resting back over the shoulder, ready.
          const yaw = this.aimer?.seat === token.seat ? this.aimer.yaw : token.yaw;
          bat.pivot.rotation.set(-1.15 + Math.sin(time * 4) * 0.05, yaw + 2.5, 0);
        } else {
          // A quick wind through the arc, level by the time it reaches the front, and a follow-through.
          bat.swing += dt;
          const t = Math.min(1, bat.swing / 0.32);
          const ease = t * t * (3 - 2 * t);
          bat.pivot.rotation.set(THREE.MathUtils.lerp(-1.15, -0.12, Math.min(1, t * 2.5)), bat.yaw + 2.5 - ease * 4.6, 0);
          token.rig.group.rotation.y = bat.yaw - ease * 0.6 + 0.3;
          if (bat.swing > 1.0) bat.swing = null;
        }
      } else if (swinging) swinging.pivot.visible = false;
      if (this.batInReach.has(token.seat)) {
        let mark = this.batMarks.get(token.seat);
        if (!mark) {
          mark = new THREE.Mesh(
            this.track(new THREE.ConeGeometry(0.2, 0.4, 4)),
            this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#f43f5e', transparent: true, opacity: 0.95, depthWrite: false, depthTest: false })),
          );
          // Pointing down at them, over their hearts, and seen through anything in the way.
          mark.rotation.x = Math.PI;
          mark.renderOrder = 23;
          this.group.add(mark);
          this.batMarks.set(token.seat, mark);
        }
        mark.visible = true;
        mark.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 3.1 + Math.sin(time * 6) * 0.15, 0));
        mark.rotation.y = time * 3;
      }
      token.tag.material.opacity = this.hiddenTag === token.seat ? 0 : 1;
      token.tag.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 2.25, 0));
    }
    this.weapons.actors([...this.tokens.values()].map(token=>{
      const raid=this.lastView?.raids?.find(raid=>raid.seat===token.seat);
      const position=this.lastView?.positioning;
      const inHand=position?.seat===token.seat?position.item:this.lastView?.turn_seat===token.seat&&this.lastView.turn.step==='lob'?this.lastView.turn.item:null;
      const item=raid?(raid.item??'slash_rush'):['drive_by','slapshot'].includes(inHand??'')?inHand as BoardWeapon:null;
      return{seat:token.seat,at:token.rig.group.position,yaw:raid?.yaw??token.rig.group.rotation.y,item,slash:raid?.slashing??0,body:token.rig.group};
    }),dt);
    this.weapons.update(dt);
    this.pedestrians.update(time,dt,this.lastView?.walk?.pulling??this.lastView?.vacuum?.pulling??[]);
    this.updateWalkCoins(time);
    for (let i = this.effects.length - 1; i >= 0; i--) {
      const effect = this.effects[i];
      effect.life -= dt;
      const t = 1 - effect.life / effect.max;
      if (effect.grow) effect.object.scale.setScalar(1 + t * effect.grow);
      if (effect.rise) effect.object.position.y += effect.rise * dt;
      (effect.object.material as THREE.Material & { opacity: number }).opacity = Math.max(0, effect.life / effect.max);
      if (effect.life <= 0) {
        effect.object.geometry.dispose();
        (effect.object.material as THREE.Material).dispose();
        effect.object.removeFromParent();
        this.effects.splice(i, 1);
      }
    }
    const winner = this.prizeWinner;
    if (this.prize && winner) {
      // Won: the chest is theirs for a moment, then rises off them and flies on.
      const token = this.tokens.get(winner.seat);
      this.prize.visible = false;
      if (performance.now() >= winner.until) {
        this.prizeWinner = null;
        this.prize.visible = !this.clear;
        if (this.prizeFlight && token) {
          this.prizeFlight.from = token.rig.group.position.clone().add(new THREE.Vector3(0, 1.5, 0));
          this.prizeFlight.t = 0;
          this.prizeFlight.watched = true;
        }
      }
    } else if (this.prize && this.prizeFlight) {
      const flight = this.prizeFlight;
      flight.t = Math.min(1, flight.t + dt / (flight.sky?1.25:PRIZE_FLIGHT_SECONDS));
      const ease = flight.sky?Math.min(1,(flight.t/0.82)**2):flight.t * flight.t * (3 - 2 * flight.t);
      this.prize.position.lerpVectors(flight.from, flight.to, ease);
      this.prize.position.y += flight.sky?(flight.t>0.82?Math.sin((flight.t-0.82)/0.18*Math.PI)*0.35:0):Math.sin(flight.t * Math.PI) * 9;
      this.prize.rotation.y += dt * 8 * (1 - flight.t);
      if (flight.watched) {
        // Off to its new spot, with the camera after it, and a look at where it landed.
        this.watch(this.prize.position, { yaw: this.angleBetween(flight.from, flight.to), pitch: 30, wide: 11, life: flight.t >= 1 ? 1.6 : 0.6 });
        if (flight.t >= 1 && !flight.sky) this.float(flight.to, 'The prize moved here!', '#f9a8d4');
      }
      if (flight.t >= 1) {
        this.prizeFlight = null;
        if(flight.sky){this.pulse(flight.to,'#fcd34d');this.sounds?.land();this.float(flight.to,'Fresh candy! Vacuum it in.','#fcd34d',2.2);}
      }
    } else if (this.prize) {
      this.prize.position.y = this.prizeBase + Math.sin(time * 1.8) * 0.05;
    }
    for (const chest of this.spares.values()) {
      chest.rotation.y = time * 0.6;
    }
    if (this.storm?.visible && this.stormParts) {
      const parts = this.stormParts;
      this.storm.rotation.y = time * 0.5;
      this.storm.position.y = parts.base + Math.sin(time * 1.1) * 0.08;
      // Each streak falls from under the cloud to the ground, then starts over.
      const top = 1.95;
      const length = 0.3;
      const positions = parts.rain.geometry.attributes.position.array as Float32Array;
      parts.drops.forEach((drop, i) => {
        const y = top - ((time * 1.6 + drop.phase) % 1) * (top - 0.2);
        positions.set([drop.x, y, drop.z, drop.x, y - length, drop.z], i * 6);
      });
      parts.rain.geometry.attributes.position.needsUpdate = true;
      // Now and then the cloud lights up for a moment.
      parts.nextFlash -= dt;
      if (parts.nextFlash <= 0) parts.nextFlash = 2.5 + ((time * 7.31) % 1) * 3;
      // A flicker that lights the cloud from inside rather than whiting it out.
      parts.cloud.emissiveIntensity = parts.nextFlash < 0.12 ? 0.45 + Math.sin(time * 90) * 0.15 : 0;
      parts.ring.material.opacity = 0.65 + Math.sin(time * 3) * 0.2;
    }
    if (this.stormNext?.visible) this.stormNext.rotation.y = time * 0.6;
    this.branchMarks.forEach((mark, index) => {
      mark.position.y = (mark.userData.base as number) + Math.sin(time * 3 + index) * 0.12;
    });
    this.placeAim();
    for (const mark of this.aimMarks.values()) {
      mark.ring.scale.setScalar(1 + Math.sin(time * 5) * 0.09);
    }
    this.tideLevel += ((this.tideIn ? 1 : 0) - this.tideLevel) * Math.min(1, dt * 1.2);
    for (const pool of this.tidePools) {
      pool.position.y = (pool.userData.base as number) - 0.3 + this.tideLevel * 0.55;
      (pool.material as THREE.MeshStandardMaterial).opacity = 0.2 + this.tideLevel * 0.5;
    }
    this.highlight.scale.setScalar(1 + Math.sin(time * 4) * 0.08);
    this.highlight.material.opacity = 0.6 + Math.sin(time * 4) * 0.2;

    for (let i = this.floaters.length - 1; i >= 0; i--) {
      const floater = this.floaters[i];
      floater.life -= dt;
      floater.sprite.position.y += floater.rise * dt;
      floater.sprite.material.opacity = Math.min(1, floater.life);
      if (floater.life <= 0) {
        floater.sprite.material.map?.dispose();
        floater.sprite.material.dispose();
        floater.sprite.removeFromParent();
        this.floaters.splice(i, 1);
      }
    }
  }

  /** Airborne targets share the walker's vertical movement, matching server aim. */
  private updateWalkCoins(time:number) {
    const walk=this.lastView?.walk;
    const walker=walk && this.tokens.get(walk.seat);
    const field=this.lastView?.air_coins;
    const count=field?.length??walk?.coins.length??0;
    if(!count || !field && (!walk || !walker)){for(const coin of this.walkCoins)coin.visible=false;return;}
    const walking=new Map(walk?.coins.map(coin=>[coin.id,coin])??[]);
    const standing=this.lastView?.vacuum;
    const pulling=new Map(standing?.pulling??[]);
    while (this.walkCoins.length<count) {
      const mesh=new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.24,0.24,0.07,24)),this.trackMaterial(new THREE.MeshStandardMaterial({color:'#fbbf24',emissive:'#b77905',emissiveIntensity:0.35,metalness:0.35,roughness:0.25,transparent:true})));
      this.group.add(mesh);this.walkCoins.push(mesh);
    }
    const base=new THREE.Vector3(),eye=new THREE.Vector3();
    this.walkCoins.forEach((mesh,index)=>{
      const ambient=field?.[index];
      const target=ambient?walking.get(ambient.id):walk?.coins[index];
      const progress=target?.progress??(ambient?pulling.get(pullKey(PULL.air,index))??0:0);
      const collected=ambient?ambient.collected_by!==null:target?.collected??false;
      const coin=ambient??target;
      mesh.visible=!!coin;
      if(!coin)return;
      const collector=ambient?.collected_by!==null&&ambient?.collected_by!==undefined?this.tokens.get(ambient.collected_by):target?walker:this.lastView?this.tokens.get(this.lastView.turn_seat):null;
      base.set(coin.x,(target&&walker?walker.rig.group.position.y:this.groundAt(coin.x,coin.z))+coin.height,coin.z);
      eye.copy(collector?.rig.group.position??base);eye.y+=walk?.eye_height??1.3;
      if(mesh.userData.coinId!==coin.id){mesh.userData.coinId=coin.id;mesh.userData.collectedAt=collected?Number.POSITIVE_INFINITY:null;mesh.userData.collectFrom=null;mesh.position.copy(base);}
      if(ambient&&!ambient.available&&!collected){mesh.visible=false;return;}
      if(collected) {
        if(mesh.userData.collectedAt===Number.POSITIVE_INFINITY){mesh.visible=false;return;}
        if(mesh.userData.collectedAt===null) {
          mesh.userData.collectedAt=time;
          mesh.userData.collectFrom=mesh.visible?mesh.position.clone():base;
          const glow=new THREE.Mesh(new THREE.SphereGeometry(0.18,12,8),new THREE.MeshBasicMaterial({color:'#67e8f9',transparent:true,opacity:0.7,depthWrite:false}));
          glow.position.copy(mesh.userData.collectFrom);
          this.group.add(glow);this.effects.push({object:glow,life:0.35,max:0.35,grow:4});
        }
        const t=THREE.MathUtils.clamp((time-mesh.userData.collectedAt)/0.22,0,1);
        mesh.visible=t<1;
        mesh.position.copy(mesh.userData.collectFrom).lerp(eye,t);
        mesh.scale.setScalar(Math.max(0.01,(1-t)*0.7));
        mesh.material.opacity=1-t;
        mesh.material.color.set('#a5f3fc');mesh.material.emissiveIntensity=1;
        return;
      }
      // Pull along the target-to-eye ray: visual suction stays exactly under
      // the same reticle while the server's hold-progress fills.
      mesh.position.copy(base).lerp(eye,progress*0.7);
      mesh.rotation.set(Math.PI/2,time*2.5+index,0);
      mesh.scale.setScalar(1-progress*0.25);
      mesh.material.color.set(progress>0?'#67e8f9':'#fbbf24');
      const active=ambient?ambient.available:target?.active;
      mesh.material.opacity=active?1:0.38;
      mesh.material.emissiveIntensity=active?0.5+progress:0.15;
    });
  }

  /** Where the camera should look while a seat takes its turn. */
  /**
   * A shot the camera should take instead of its usual one, while something
   * worth watching happens: a roll, or an item thrown at someone. It swings
   * round and drops low, then hands the camera back.
   */
  shot(): BoardShot | null {
    if (!this.cinema) return null;
    const shot = this.cinema;
    const rolling=shot.rollSeat!==undefined&&this.lastView?.walk?.seat===shot.rollSeat;
    const fade = rolling?1:Math.min(1, shot.life / 0.45);
    const age=shot.max-shot.life;
    const sweep=shot.sweep?impactSweep(age):1;
    return {
      at: shot.at,
      wide: shot.sweep?THREE.MathUtils.lerp(12,shot.wide,sweep):shot.wide,
      tall: shot.sweep?THREE.MathUtils.lerp(12*0.76,shot.tall,sweep):shot.tall,
      pitch: shot.sweep?THREE.MathUtils.lerp(44,shot.pitch,sweep):shot.pitch,
      yaw: shot.yaw+(shot.sweep?THREE.MathUtils.lerp(-0.65,0.25,sweep):0),
      // Eased in and out, so the camera is never yanked anywhere.
      weight: fade * fade * (3 - 2 * fade)*(shot.sweep?sweep:1),
    };
  }

  /** Looks in over the counter from the path, for as long as the shopping lasts. */
  private lookIntoShop() {
    const shop = this.shopping === null ? null : this.shops.get(this.shopping);
    if (!shop) return;
    const inside = shop.at.clone();
    inside.y += 0.7;
    this.watch(inside, { yaw: shop.facing, pitch: 14, wide: 3.4, life: 1.2 });
  }

  /** Watch a place from a low angle for `life` seconds. */
  private watch(at: THREE.Vector3, options: { yaw: number; pitch: number; wide: number; life: number;rollSeat?:number }) {
    this.cinema = {
      at: at.clone(),
      yaw: options.yaw,
      pitch: options.pitch,
      wide: options.wide,
      tall: options.wide * 0.76,
      life: options.life,
      max: options.life,
      rollSeat:options.rollSeat,
    };
  }

  /** The way the camera should face to watch `to` from behind `from`. */
  private angleBetween(from: THREE.Vector3, to: THREE.Vector3) {
    return Math.atan2(from.x - to.x, from.z - to.z);
  }

  /** Shared actor/camera clock, so the view switches exactly as movement begins. */
  walkElapsed():number|null {
    const walk=this.lastView?.walk;if(!walk)return null;
    return walk.elapsed+(this.lastView?.candy_van||this.replaying||(walk.waiting??0)>0?0:Math.min(.12,Math.max(0,(performance.now()-(this.walkClock?.at??performance.now()))/1000)));
  }

  /** Follow the path surface, rather than bouncing the camera with every hop. */
  walkFloor():number|null {
    const walk=this.lastView?.walk;if(!walk)return null;
    const movement=Math.max(0,((this.walkElapsed()??walk.elapsed)-walk.delay)/walk.hop_seconds);
    const segment=Math.min(walk.path.length-1,Math.floor(movement)),t=THREE.MathUtils.clamp(movement-segment,0,1);
    const from=this.positions.get(segment===0?walk.from:walk.path[segment-1]),to=this.positions.get(walk.path[segment]);
    if(!from||!to)return null;
    return THREE.MathUtils.lerp(from.y,to.y,t);
  }

  focus(seat: number | null) {
    const token = seat === null ? null : this.tokens.get(seat);
    return token ? token.rig.group.position.clone() : new THREE.Vector3(0, 0, 0);
  }

  /**
   * The shot from inside a tunnel while a seat's token is down in one: from
   * a little behind and above it, looking along the way it is going.
   */
  cave(seat: number | null): { eye: THREE.Vector3; look: THREE.Vector3 } | null {
    const token = seat === null ? null : this.tokens.get(seat);
    if (!token || !this.terrain) return null;
    const at = token.rig.group.position;
    // Below the land by more than a token's height is in a tunnel, which also
    // covers the hops in and out of one.
    if (at.y > this.terrain.height(at.x, at.z) - 1.4) return null;
    const along = new THREE.Vector3(Math.sin(token.yaw), 0, Math.cos(token.yaw));
    const eye = at.clone().addScaledVector(along, -3.3).add(new THREE.Vector3(0, 1.85, 0));
    // Behind the token is the space it came from; the bore follows the path
    // there, so stand over it when the tunnel bends.
    const arrival = this.lastView?.tokens.find((one) => one.seat === seat)?.previous;
    const behind = this.positions.get(arrival ?? this.prevOf.get(token.space) ?? -1);
    if (behind && this.underground.has(token.space) && token.hop >= 1) {
      eye.lerp(behind.clone().add(new THREE.Vector3(0, 1.85, 0)), 0.5);
    }
    const look = at.clone().addScaledVector(along, 2.4).add(new THREE.Vector3(0, 0.7, 0));
    return { eye, look };
  }

  /** The sound for a line of the log, if it has one. Moves and rolls play theirs as they happen. */
  private hear(entry: BoardLogEntry) {
    const sounds = this.sounds;
    if (!sounds) return;
    const kind = entry.kind;
    const effect = entry.effect;
    if(kind==='traffic_near_miss'||kind==='traffic_horn')sounds.cityHorn(Number(entry.car),Number(entry.x),Number(entry.z));
    else if(kind==='traffic_brake')sounds.cityBrake(Number(entry.x),Number(entry.z));
    else if(kind==='traffic_hit')sounds.cityCrash(Number(entry.x),Number(entry.z));
    else if(kind==='npc_robbed')sounds.cityRobbed(Number(entry.x),Number(entry.z));
    else if (kind === 'walk_coin' || kind==='vacuumed' && entry.what==='air_coin') sounds.coins();
    else if (kind === 'space') {
      if (effect === 'coins' || effect === 'toll' || effect === 'washed_up') sounds.coins();
      else if (effect === 'loss') sounds.lose();
      else if (effect === 'shop') sounds.shop();
      else if (effect === 'item') sounds.item();
      else if (effect === 'soaked') sounds.splash();
    } else if (kind === 'minigame' || kind === 'pickup' || kind === 'revenge' || kind === 'award') sounds.coins(true);
    else if (kind === 'bought') {
      if (entry.item) sounds.coins();
    } else if (kind === 'upgraded') sounds.coins(); else if (kind === 'toll_road' && entry.coins) sounds.lose();
    else if (kind === 'slide') sounds.slide();
    else if (kind === 'hurt' || kind === 'scrap') sounds.hurt();
    else if (kind === 'knockout') sounds.knockout();
    else if (kind === 'spill') sounds.lose();
    else if (kind === 'item') sounds.useItem();
    else if (kind === 'blocked') sounds.blocked();
    else if (kind === 'taunt') sounds.taunt();
    else if (kind === 'storm' || kind === 'blown') sounds.thunder();
    else if (kind === 'prize') sounds.prize();
    else if (kind === 'round') sounds.round();
  }

  /** What a token stepping onto a point walks on. */
  private groundUnder(at: THREE.Vector3): Ground {
    let nearest = -1;
    let best = Infinity;
    for (const [id, position] of this.positions) {
      const distance = (position.x - at.x) ** 2 + (position.z - at.z) ** 2;
      if (distance < best) {
        best = distance;
        nearest = id;
      }
    }
    if (this.underground.has(nearest)) return 'stone';
    if (this.deckSpaces.has(nearest)) return 'wood';
    if (this.map === 'frozen') return 'snow';
    if (this.map === 'volcano') return 'stone';
    // Sand crunches much like snow underfoot.
    return this.map === 'shipwreck' ? 'snow' : 'grass';
  }

  /** Where a seat is heading: a few spaces on from where they stand, taking
   * the first way at a fork. Where they stand, if the way runs out. */
  ahead(seat: number | null, steps = 3) {
    const token = seat === null ? null : this.tokens.get(seat);
    if (!token || !this.lastView) return this.focus(seat);
    const { byId, incoming } = this.pathIndex(this.lastView.spaces);
    let space = token.space;
    let previous = this.lastView.tokens.find((t) => t.seat === seat)?.previous ?? null;
    for (let i = 0; i < steps; i++) {
      const connected = byId.get(space)?.link != null ? [] : [...(this.nextOf.get(space) ?? [])];
      for (const id of incoming.get(space) ?? []) if (!connected.includes(id)) connected.push(id);
      const next = connected.find((id) => id !== previous && (this.lastView!.gates_open || !byId.get(id)?.gated));
      if (next === undefined) break;
      previous = space;
      space = next;
    }
    return this.positions.get(space)?.clone() ?? this.focus(seat);
  }

  /** The spaces by id, and the unlinked spaces leading into each, kept until the board's spaces change. */
  private pathIndex(spaces: BoardView['spaces']) {
    if (this.pathIndexOf?.spaces !== spaces) {
      const byId = new Map(spaces.map((space) => [space.id, space]));
      const incoming = new Map<number, number[]>();
      for (const space of spaces) {
        if (space.link != null) continue;
        for (const next of space.next) incoming.set(next, [...(incoming.get(next) ?? []), space.id]);
      }
      this.pathIndexOf = { spaces, byId, incoming };
    }
    return this.pathIndexOf;
  }

  /** What is under the pointer, for a tip: an item badge or the hearts over a
   * token, the storm or where it goes next, or a space. */
  hover(raycaster: THREE.Raycaster): { title: string; body: string } | null {
    for (const token of this.tokens.values()) {
      const status = token.tag.userData.token as BoardToken | undefined;
      if (!status || !token.tag.visible) continue;
      const hit = raycaster.intersectObject(token.tag, false)[0];
      if (!hit?.uv) continue;
      const x = hit.uv.x * TAG_W;
      const y = (1 - hit.uv.y) * TAG_H;
      if (y >= BADGE_TOP - 4 && status.resting <= 0) {
        const badge = badgeLayout(status.items.length).find((b) => x >= b.x - 4 && x <= b.x + BADGE + 4);
        if (badge) {
          const item = ITEMS[status.items[badge.index]];
          return { title: item.label, body: item.help };
        }
      }
      if (y < BADGE_TOP - 4) {
        const notes = [status.shield ? 'Shielded from the next item or trap.' : '', status.cursed ? 'Cursed: the next roll only goes up to 3.' : '']
          .filter(Boolean)
          .join(' ');
        return { title: `${status.health} of ${this.maxHealth} hearts`, body: notes || 'Lose them all and you are knocked out.' };
      }
    }
    // Only the cloud and its ring count: a line's hit area reaches a whole
    // unit either side, so the rain would claim the spaces around it.
    const solid = (hits: THREE.Intersection[]) => hits.some((hit) => hit.object instanceof THREE.Mesh);
    if (this.storm?.visible && solid(raycaster.intersectObject(this.storm, true))) {
      return { title: 'Storm', body: 'Anyone it catches loses 4 coins and is blown back 3 spaces. It moves 4 spaces each round.' };
    }
    if (this.stormNext?.visible && solid(raycaster.intersectObject(this.stormNext, true))) {
      return { title: 'Storm, next round', body: 'Where the storm moves to after this round.' };
    }
    const tile = raycaster.intersectObjects([...this.tiles.values()], false)[0];
    const kind = tile?.object.userData.kind as SpaceKind | undefined;
    if (kind) return { title: SPACE_HELP[kind].label, body: SPACE_HELP[kind].help };
    return null;
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  dispose() {
    for (const texture of this.badges.values()) texture.dispose();
    for (const bag of this.bags.values()) bag.built.dispose();
    this.bags.clear();
    this.badges.clear();
    this.crawler.dispose();
    this.traffic.dispose();
    this.rental.dispose();
    this.candyVan.dispose();
    this.weapons.dispose();
    this.pedestrians.dispose();
    this.finale.dispose();
    this.staticProps.dispose();
    this.group.traverse((object) => {
      (object.userData.districtTexture as THREE.Texture | undefined)?.dispose();
    });
    this.markBranch([]);
    this.setAim([]);
    this.tokens.forEach((token) => {
      token.rig.dispose();
      this.disposeTag(token);
    });
    this.tokens.clear();
    this.shops.forEach((shop) => shop.keeper.dispose());
    this.shops.clear();
    disposeGlyphs();
    this.effects.forEach((effect) => {
      effect.object.geometry.dispose();
      (effect.object.material as THREE.Material).dispose();
    });
    this.effects = [];
    this.floaters.forEach((floater) => {
      floater.sprite.material.map?.dispose();
      floater.sprite.material.dispose();
    });
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
