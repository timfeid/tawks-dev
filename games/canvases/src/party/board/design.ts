import type { MapKey } from '../protocol';
import island from './maps/island.json';
import volcano from './maps/volcano.json';
import shipwreck from './maps/shipwreck.json';
import frozen from './maps/frozen.json';

/**
 * The hand-built boards, one map file each, shared with the server, which
 * lays its spaces along the paths. Everything else here is for drawing: the
 * coastline, the land, the landmarks and where the scenery grows.
 */
export type Design = {
  id: MapKey;
  /** The island's outline, as points round it; drawn smooth. */
  coast: [number, number][];
  land: Feature[];
  zones?: { name: string; x: number; z: number; r: number }[];
  /** Named landmarks with a distinct ground palette, visible from the board camera. */
  districts?: { name: string; x: number; z: number; r: number; color: string }[];
  paths: DesignPath[];
  roads?: {points:[number,number][];width:number}[];
  pedestrian_routes?:{points:[number,number][]}[];
  props?: Prop[];
  clusters?: Cluster[];
  /** Where smoke rises from. */
  smoke?: [number, number][];
  /** Small islands off the coast: where, and how wide. */
  islets?: [number, number, number][];
  /** How craggy the high ground is: 0 smooth, about 1.4 by default. */
  rough?: number;
  /** The board's own things for the vacuum, off the path. */
  specials?: { palms?: [number, number][]; flotsam?: [number, number][]; snowmen?: [number, number][] };
};

export type Feature =
  | { kind: 'hill' | 'plateau' | 'crater'; x: number; z: number; r: number; h: number }
  | { kind: 'ridge'; from: [number, number]; to: [number, number]; r: number; h: number }
  /** Wet ground: a pool sunk under the sea's level, frozen over on Frozen Peak. */
  | { kind: 'pool'; x: number; z: number; r: number; h: number; frozen?: boolean }
  /** A river cut down to the sea, or a lava flow. */
  | { kind: 'river'; points: [number, number][]; r: number; h: number; lava?: boolean };

export type DesignPath = {
  id: string;
  spacing?:number;
  closed?: boolean;
  points: unknown[];
  under?: boolean;
  gated?: boolean;
  /** Laid as a rope bridge, held up over water or a drop. */
  bridge?: boolean;
  /** Laid in planks, as on a ship's deck. */
  deck?: boolean;
  join_paths?: boolean;
};

export type Prop = { model: string; label?:string; x: number; z: number; y?: number; lift?: number; yaw?: number; scale?: number; paint?: Record<string, string | null> | string };

/** Scenery placed about a point: anywhere in the circle, or round its edge with `ring`. */
export type Cluster = {
  models: string[];
  x: number;
  z: number;
  r: number;
  count: number;
  scale?: [number, number];
  ring?: boolean;
  /** One colour for the whole model, as basalt or snow. */
  paint?: string;
};

export const DESIGNS: Partial<Record<MapKey, Design>> = {
  island: island as Design,
  volcano: volcano as Design,
  shipwreck: shipwreck as Design,
  frozen: frozen as unknown as Design,
};
