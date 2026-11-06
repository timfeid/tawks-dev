/// <reference lib="webworker" />
/** Fetches, caches, decodes and meshes tiles, in a worker or, failing that, on the page. */

import { Frame, type LatLon, tileKey } from './geo';
import { decodeTile } from './mvt';
import { TileCache } from './tilecache';
import { buildTile, transferables } from './tilemesh';

export type ToWorker = { type: 'origin'; origin: LatLon; epoch: number } | { type: 'load'; x: number; y: number; epoch: number };
export type FromWorker =
  | { type: 'tile'; key: string; epoch: number; mesh: ReturnType<typeof buildTile>; bytes: number; ms: number; cached: boolean }
  | { type: 'failed'; key: string; epoch: number; error: string }
  | { type: 'storage'; persistent: boolean };

const TILEJSON = 'https://tiles.openfreemap.org/planet';
const LAYERS = ['transportation', 'transportation_name', 'building', 'water', 'landuse', 'landcover', 'park'];
const PARALLEL = 4;

type Post = (message: FromWorker, transfer?: Transferable[]) => void;

/** The tile pipeline; `post` carries results back to the world. Returns the message handler. */
export function tileService(post: Post): (msg: ToWorker) => void {
const cache = new TileCache();
let frame: Frame | null = null;
let epoch = 0;
let template: Promise<string> | null = null;
const queue: { x: number; y: number; epoch: number }[] = [];
let running = 0;

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

async function run(job: { x: number; y: number; epoch: number }) {
  const key = tileKey(job.x, job.y);
  try {
    const started = performance.now();
    const { data, cached } = await fetchTile(job.x, job.y);
    if (job.epoch !== epoch || !frame) return;
    const mesh = buildTile(decodeTile(new Uint8Array(data), LAYERS), job.x, job.y, frame);
    const message: FromWorker = { type: 'tile', key, epoch: job.epoch, mesh, bytes: data.byteLength, ms: performance.now() - started, cached };
    post(message, transferables(mesh));
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
