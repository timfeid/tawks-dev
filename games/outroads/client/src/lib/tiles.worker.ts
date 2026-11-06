/// <reference lib="webworker" />
/** Fetches, caches, decodes and meshes tiles, in a worker or, failing that, on the page. */

import { Frame, type LatLon, tileKey, tileOf } from './geo';
import { paintGround } from './groundpaint';
import { decodeTile } from './mvt';
import { decodeTerrarium, elevationTilesFor, heightsForTile, TERRAIN_ZOOM, type Heights } from './terrain';
import { TileCache } from './tilecache';
import { buildTile, transferables } from './tilemesh';

export type WorldOptions = {
  /** Where elevation tiles come from, as `${terrain}/{z}/{x}/{y}.png`, or null for flat ground. */
  terrain: string | null;
  /** Pixels across each tile's ground paint. */
  groundPx: number;
  fillers: boolean;
};

export type ToWorker =
  | { type: 'origin'; origin: LatLon; epoch: number; options: WorldOptions }
  | { type: 'load'; x: number; y: number; epoch: number };
export type FromWorker =
  | {
      type: 'tile';
      key: string;
      epoch: number;
      mesh: ReturnType<typeof buildTile>;
      paint: Uint8ClampedArray | null;
      paintPx: number;
      bytes: number;
      ms: number;
      cached: boolean;
    }
  | { type: 'failed'; key: string; epoch: number; error: string }
  | { type: 'storage'; persistent: boolean };

const TILEJSON = 'https://tiles.openfreemap.org/planet';
const LAYERS = ['transportation', 'transportation_name', 'building', 'water', 'landuse', 'landcover', 'park'];
const PARALLEL = 4;
const ELEVATION_TILES_KEPT = 32;

type Post = (message: FromWorker, transfer?: Transferable[]) => void;

/** The tile pipeline; `post` carries results back to the world. Returns the message handler. */
export function tileService(post: Post): (msg: ToWorker) => void {
  const cache = new TileCache();
  let frame: Frame | null = null;
  let epoch = 0;
  let options: WorldOptions = { terrain: null, groundPx: 256, fillers: false };
  let base: Promise<number> = Promise.resolve(0);
  let template: Promise<string> | null = null;
  const queue: { x: number; y: number; epoch: number }[] = [];
  let running = 0;
  const elevation = new Map<string, Promise<Float32Array | null>>();

  void cache.persistent.then((persistent) => post({ type: 'storage', persistent }));

  function tileTemplate(): Promise<string> {
    template ??= fetch(TILEJSON)
      .then((r) => {
        if (!r.ok) throw new Error(`TileJSON ${r.status}`);
        return r.json();
      })
      .then((json: { tiles: string[] }) => json.tiles[0])
      .catch((error) => {
        template = null;
        throw error;
      });
    return template;
  }

  async function fetchTile(x: number, y: number): Promise<{ data: ArrayBuffer; cached: boolean }> {
    const key = tileKey(x, y);
    const hit = await cache.get(key);
    if (hit) return { data: hit, cached: true };
    const url = (await tileTemplate()).replace('{z}', '14').replace('{x}', String(x)).replace('{y}', String(y));
    for (let attempt = 0; ; attempt++) {
      const response = await fetch(url);
      if (response.ok) {
        const data = await response.arrayBuffer();
        void cache.put(key, data);
        return { data, cached: false };
      }
      if (response.status === 404 || response.status === 204) return { data: new ArrayBuffer(0), cached: false };
      if (attempt >= 2) throw new Error(`Tile ${response.status}`);
      await new Promise((r) => setTimeout(r, 800 * (attempt + 1)));
    }
  }

  async function decodePng(data: ArrayBuffer): Promise<Float32Array> {
    const bitmap = await createImageBitmap(new Blob([data], { type: 'image/png' }));
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
    ctx.drawImage(bitmap, 0, 0);
    bitmap.close();
    return decodeTerrarium(ctx.getImageData(0, 0, canvas.width, canvas.height).data);
  }

  /** One zoom 13 elevation tile as heights, or null when it cannot be had. */
  function elevationTile(x: number, y: number): Promise<Float32Array | null> {
    const key = `${x}/${y}`;
    let hit = elevation.get(key);
    if (hit) {
      elevation.delete(key);
      elevation.set(key, hit);
      return hit;
    }
    hit = (async () => {
      const cacheKey = `dem/${TERRAIN_ZOOM}/${key}`;
      let data = await cache.get(cacheKey);
      if (!data) {
        for (let attempt = 0; attempt < 3 && !data; attempt++) {
          const response = await fetch(`${options.terrain}/${TERRAIN_ZOOM}/${key}.png`).catch(() => null);
          if (response?.ok) data = await response.arrayBuffer();
          else if (response && response.status < 500) break;
          else await new Promise((r) => setTimeout(r, 700 * (attempt + 1)));
        }
        if (!data) return null;
        void cache.put(cacheKey, data);
      }
      return decodePng(data).catch(() => null);
    })();
    elevation.set(key, hit);
    while (elevation.size > ELEVATION_TILES_KEPT) elevation.delete(elevation.keys().next().value!);
    return hit;
  }

  async function heightsFor(x: number, y: number, bounds: [number, number, number]): Promise<Heights | null> {
    if (!options.terrain) return null;
    const needed = elevationTilesFor(x, y);
    const images = await Promise.all(needed.map(([ex, ey]) => elevationTile(ex, ey)));
    if (images.some((i) => !i)) return null;
    const byKey = new Map(needed.map(([ex, ey], i) => [`${ex}/${ey}`, images[i]!]));
    const pixel = (px: number, py: number) => {
      const ex = Math.floor(px / 256);
      const ey = Math.floor(py / 256);
      const image = byKey.get(`${ex}/${ey}`);
      return image ? image[(py - ey * 256) * 256 + (px - ex * 256)] : 0;
    };
    return heightsForTile(x, y, pixel, await base, bounds);
  }

  async function originHeight(origin: LatLon): Promise<number> {
    if (!options.terrain) return 0;
    const t = tileOf(origin, TERRAIN_ZOOM);
    const image = await elevationTile(Math.floor(t.x), Math.floor(t.y));
    if (!image) return 0;
    const px = Math.min(255, Math.floor((t.x % 1) * 256));
    const py = Math.min(255, Math.floor((t.y % 1) * 256));
    return image[py * 256 + px];
  }

  async function run(job: { x: number; y: number; epoch: number }) {
    const key = tileKey(job.x, job.y);
    try {
      const started = performance.now();
      const local = frame;
      if (!local) return;
      const [minX, minZ] = local.fromTile(job.x, job.y, 1, 0, 0);
      const bounds: [number, number, number] = [minX, minZ, local.metersPerTile];
      const [{ data, cached }, heights] = await Promise.all([fetchTile(job.x, job.y), heightsFor(job.x, job.y, bounds)]);
      if (job.epoch !== epoch) return;
      const layers = decodeTile(new Uint8Array(data), LAYERS);
      const mesh = buildTile(layers, job.x, job.y, local, heights, { fillers: options.fillers });
      const paint = paintGround(layers, options.groundPx);
      const message: FromWorker = {
        type: 'tile',
        key,
        epoch: job.epoch,
        mesh,
        paint,
        paintPx: options.groundPx,
        bytes: data.byteLength,
        ms: performance.now() - started,
        cached,
      };
      post(message, [...transferables(mesh), ...(paint ? [paint.buffer as ArrayBuffer] : [])]);
    } catch (error) {
      post({ type: 'failed', key, epoch: job.epoch, error: String(error) });
    }
  }

  function pump() {
    while (running < PARALLEL && queue.length) {
      const job = queue.shift()!;
      if (job.epoch !== epoch) continue;
      running++;
      void run(job).finally(() => {
        running--;
        pump();
      });
    }
  }

  return (msg: ToWorker) => {
    if (msg.type === 'origin') {
      frame = new Frame(msg.origin);
      epoch = msg.epoch;
      options = msg.options;
      base = originHeight(msg.origin);
      queue.length = 0;
    } else if (msg.type === 'load') {
      queue.push(msg);
      pump();
    }
  };
}

if (typeof WorkerGlobalScope !== 'undefined' && self instanceof WorkerGlobalScope) {
  const scope = self as unknown as DedicatedWorkerGlobalScope;
  const handle = tileService((message, transfer) => scope.postMessage(message, transfer ?? []));
  scope.onmessage = (event: MessageEvent<ToWorker>) => handle(event.data);
}
