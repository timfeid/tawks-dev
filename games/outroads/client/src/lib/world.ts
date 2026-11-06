/**
 * The 3D world: streams tiles around the camera, keeps a draw budget with
 * distance culling and a tall-buildings-only far range, and lights the
 * scene for the hour of day.
 */

import * as THREE from 'three';
import { Frame, type LatLon, TILE_ZOOM, tileKey } from './geo';
import { RoadIndex } from './roads';
import { GROUND } from './style';
import type { TileMesh } from './tilemesh';
import { tileService, type FromWorker, type ToWorker } from './tiles.worker';
import TileWorker from './tiles.worker?worker&inline';

export type Quality = {
  /** Tiles load while any part of them is this close, in meters. */
  loadRadius: number;
  /** Buildings and lamps beyond this are not drawn. */
  viewDistance: number;
  /** Past this only tall buildings are drawn. */
  detailDistance: number;
  pixelRatio: number;
  antialias: boolean;
  trees: boolean;
};

export const QUALITY: Record<'high' | 'low', Quality> = {
  high: { loadRadius: 1300, viewDistance: 1400, detailDistance: 520, pixelRatio: 1.5, antialias: true, trees: true },
  low: { loadRadius: 800, viewDistance: 800, detailDistance: 300, pixelRatio: 1, antialias: false, trees: true },
};

type Chunk = { mesh: THREE.Mesh; tall: number; all: number; cx: number; cz: number; radius: number };

type LoadedTile = {
  key: string;
  group: THREE.Group;
  chunks: Chunk[];
  lamps: THREE.InstancedMesh | null;
  heads: THREE.InstancedMesh | null;
  trees: THREE.InstancedMesh | null;
  imagery: THREE.Mesh | null;
  minX: number;
  minZ: number;
  maxX: number;
  maxZ: number;
  stats: TileMesh['stats'];
};

export type WorldStats = { tiles: number; loading: number; calls: number; triangles: number; buildings: number; failed: number };

function windowTextures() {
  const size = 256;
  const cells = 8;
  const day = document.createElement('canvas');
  const night = document.createElement('canvas');
  day.width = day.height = night.width = night.height = size;
  const d = day.getContext('2d', { willReadFrequently: true })!;
  const n = night.getContext('2d', { willReadFrequently: true })!;
  d.fillStyle = '#ffffff';
  d.fillRect(0, 0, size, size);
  n.fillStyle = '#000000';
  n.fillRect(0, 0, size, size);
  const cell = size / cells;
  let seed = 7;
  const rand = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let cx = 0; cx < cells; cx++) {
    for (let cy = 0; cy < cells; cy++) {
      const x = cx * cell + cell * 0.26;
      const y = cy * cell + cell * 0.28;
      const w = cell * 0.48;
      const h = cell * 0.44;
      if (cx === 0 && cy === 0) continue;
      d.fillStyle = rand() < 0.5 ? '#aab7c4' : '#98a6b4';
      d.fillRect(x, y, w, h);
      if (rand() < 0.42) {
        const warm = rand();
        n.fillStyle = warm < 0.7 ? '#ffd79a' : warm < 0.9 ? '#fff2d6' : '#bfd8ff';
        n.fillRect(x, y, w, h);
      }
    }
  }
  // Pixels, not the canvas itself: a GPU canvas can lose its contents and take the texture with it.
  const make = (ctx: CanvasRenderingContext2D) => {
    const t = new THREE.DataTexture(ctx.getImageData(0, 0, size, size).data, size, size);
    t.wrapS = t.wrapT = THREE.RepeatWrapping;
    t.magFilter = THREE.LinearFilter;
    t.minFilter = THREE.LinearMipmapLinearFilter;
    t.generateMipmaps = true;
    t.needsUpdate = true;
    t.colorSpace = THREE.SRGBColorSpace;
    t.anisotropy = 4;
    return t;
  };
  return { day: make(d), night: make(n) };
}

function lampGeometry() {
  const pole = new THREE.BoxGeometry(0.16, 6.2, 0.16).translate(0, 3.1, 0);
  const arm = new THREE.BoxGeometry(0.12, 0.12, 1.6).translate(0, 6.1, 0.75);
  const merged = new THREE.BufferGeometry();
  const parts = [pole.toNonIndexed(), arm.toNonIndexed()];
  const pos = parts.flatMap((p) => Array.from(p.getAttribute('position').array as Float32Array));
  const nor = parts.flatMap((p) => Array.from(p.getAttribute('normal').array as Float32Array));
  merged.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  merged.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3));
  return merged;
}

function treeGeometry() {
  const trunk = new THREE.CylinderGeometry(0.18, 0.26, 2.4, 5).translate(0, 1.2, 0).toNonIndexed();
  const crown = new THREE.IcosahedronGeometry(2.3, 0).translate(0, 4.2, 0);
  const g = new THREE.BufferGeometry();
  const pos = [...(trunk.getAttribute('position').array as Float32Array), ...(crown.getAttribute('position').array as Float32Array)];
  const nor = [...(trunk.getAttribute('normal').array as Float32Array), ...(crown.getAttribute('normal').array as Float32Array)];
  const col: number[] = [];
  for (let i = 0; i < trunk.getAttribute('position').count; i++) col.push(0.36, 0.25, 0.16);
  for (let i = 0; i < crown.getAttribute('position').count; i++) col.push(0.27, 0.48, 0.24);
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3));
  g.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
  return g;
}

/**
 * Activity frames have no origin, so a worker has to start from a blob. The
 * build inlines the worker; the dev server only serves it by address, so a
 * blob imports it from there.
 */
function startWorker(): Worker {
  if (import.meta.env.DEV) {
    const url = new URL('./tiles' + '.worker.ts?worker_file&type=module', import.meta.url).href;
    const blob = new Blob([`import ${JSON.stringify(url)};`], { type: 'text/javascript' });
    return new Worker(URL.createObjectURL(blob), { type: 'module' });
  }
  return new TileWorker();
}

const SKY = {
  day: new THREE.Color('#9ccbf0'),
  dusk: new THREE.Color('#f0a476'),
  night: new THREE.Color('#0d1730'),
};

export class World {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  readonly index = new RoadIndex();
  frame: Frame | null = null;
  quality: Quality;
  /** 0 by day, 1 at night. */
  night = 0;
  failed = 0;

  private worker: Worker | null;
  private send: (msg: ToWorker) => void;
  private sent: ToWorker[] = [];
  private epoch = 0;
  private tiles = new Map<string, LoadedTile>();
  private pending = new Set<string>();
  private arrived: TileMesh[] = [];
  private hemi = new THREE.HemisphereLight(0xdfefff, 0x6b6450, 1);
  private sun = new THREE.DirectionalLight(0xffffff, 1.2);
  private base: THREE.Mesh;
  private materials: {
    ground: THREE.MeshLambertMaterial;
    roads: THREE.MeshLambertMaterial;
    buildings: THREE.MeshLambertMaterial;
    lamps: THREE.MeshLambertMaterial;
    heads: THREE.MeshBasicMaterial;
    trees: THREE.MeshLambertMaterial;
  };
  private lampGeo = lampGeometry();
  private headGeo = new THREE.BoxGeometry(0.5, 0.18, 0.9).translate(0, 6.0, 1.5);
  private treeGeo = treeGeometry();
  private lodClock = 0;
  private satellite = false;
  private textureLoader = new THREE.TextureLoader();
  private onStorage: ((persistent: boolean) => void) | null = null;
  private timer: { ext: { TIME_ELAPSED_EXT: number; GPU_DISJOINT_EXT: number }; gl: WebGL2RenderingContext; queries: WebGLQuery[] } | null = null;
  /** Mean GPU time per frame over the last few frames, when the browser can measure it. */
  gpuMs: number | null = null;
  private gpuSamples: number[] = [];

  constructor(canvas: HTMLCanvasElement, quality: Quality) {
    this.quality = quality;
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: quality.antialias, powerPreference: 'high-performance', preserveDrawingBuffer: false });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, quality.pixelRatio));
    this.renderer.info.autoReset = false;
    this.camera = new THREE.PerspectiveCamera(62, 1, 0.5, quality.viewDistance + 200);
    this.scene.fog = new THREE.Fog(SKY.day, quality.viewDistance * 0.45, quality.viewDistance);
    this.scene.background = SKY.day.clone();
    this.scene.add(this.hemi, this.sun, this.sun.target);
    const windows = windowTextures();
    const layer = (order: number) => {
      const m = new THREE.MeshLambertMaterial({ vertexColors: true, depthWrite: false, side: THREE.DoubleSide });
      m.userData.order = order;
      return m;
    };
    this.materials = {
      ground: layer(-3),
      roads: layer(-2),
      buildings: new THREE.MeshLambertMaterial({ vertexColors: true, map: windows.day, emissiveMap: windows.night, emissive: 0xffffff, emissiveIntensity: 0 }),
      lamps: new THREE.MeshLambertMaterial({ color: 0x4a4e55 }),
      heads: new THREE.MeshBasicMaterial({ color: 0x55585e }),
      trees: new THREE.MeshLambertMaterial({ vertexColors: true }),
    };
    const g = GROUND.map((c) => c / 255);
    this.base = new THREE.Mesh(
      new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2),
      new THREE.MeshLambertMaterial({ color: new THREE.Color(g[0], g[1], g[2]).convertSRGBToLinear(), depthWrite: false }),
    );
    this.base.scale.setScalar(quality.viewDistance * 3);
    this.base.renderOrder = -4;
    this.scene.add(this.base);

    const worker = startWorker();
    this.worker = worker;
    let answered = false;
    worker.onmessage = (event: MessageEvent<FromWorker>) => {
      if (!answered) this.sent = [];
      answered = true;
      this.receive(event.data);
    };
    worker.onerror = () => {
      if (answered) return;
      console.warn('[outroads] the tile worker did not start; meshing on the page instead');
      worker.terminate();
      this.worker = null;
      this.useInPage();
    };
    this.send = (msg) => {
      if (!answered) this.sent.push(msg);
      worker.postMessage(msg);
    };
  }

  /** Replays what the worker was asked, to a pipeline on the page. */
  private useInPage() {
    const handle = tileService((message) => this.receive(message));
    const replay = this.sent;
    this.sent = [];
    this.send = handle;
    const origin = [...replay].reverse().find((m) => m.type === 'origin');
    if (!origin) return;
    handle(origin);
    for (const m of replay) if (m.type === 'load' && m.epoch === origin.epoch) handle(m);
  }

  storage(listener: (persistent: boolean) => void) {
    this.onStorage = listener;
  }

  private receive(msg: FromWorker) {
    if (msg.type === 'storage') {
      this.onStorage?.(msg.persistent);
      return;
    }
    if (msg.epoch !== this.epoch) return;
    this.pending.delete(msg.key);
    if (msg.type === 'failed') {
      this.failed++;
      console.warn('[outroads] tile failed', msg.key, msg.error);
      return;
    }
    this.arrived.push(msg.mesh);
  }

  resize(width: number, height: number) {
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / Math.max(1, height);
    this.camera.updateProjectionMatrix();
  }

  setQuality(quality: Quality) {
    this.quality = quality;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, quality.pixelRatio));
    this.camera.far = quality.viewDistance + 200;
    this.camera.updateProjectionMatrix();
    const fog = this.scene.fog as THREE.Fog;
    fog.near = quality.viewDistance * 0.45;
    fog.far = quality.viewDistance;
    this.base.scale.setScalar(quality.viewDistance * 3);
  }

  setOrigin(origin: LatLon) {
    this.epoch++;
    for (const key of [...this.tiles.keys()]) this.drop(key);
    this.pending.clear();
    this.arrived = [];
    this.index.clear();
    this.failed = 0;
    const plain = { lat: origin.lat, lon: origin.lon };
    this.frame = new Frame(plain);
    this.send({ type: 'origin', origin: plain, epoch: this.epoch });
  }

  setSatellite(on: boolean) {
    this.satellite = on;
    for (const tile of this.tiles.values()) this.applySatellite(tile);
  }

  /** True once the tile under a point has arrived, so spawning can find a road. */
  ready(x: number, z: number) {
    if (!this.frame) return false;
    const t = this.frame.tileAt(x, z);
    return this.tiles.has(tileKey(Math.floor(t.x), Math.floor(t.y)));
  }

  get loading() {
    return this.pending.size + this.arrived.length;
  }

  private drop(key: string) {
    const tile = this.tiles.get(key);
    if (!tile) return;
    this.scene.remove(tile.group);
    tile.group.traverse((o) => {
      if (o instanceof THREE.Mesh && !(o instanceof THREE.InstancedMesh)) {
        o.geometry.dispose();
        if (o === tile.imagery) {
          const m = o.material as THREE.MeshBasicMaterial;
          m.map?.dispose();
          m.dispose();
        }
      }
    });
    tile.lamps?.dispose();
    tile.heads?.dispose();
    tile.trees?.dispose();
    this.tiles.delete(key);
    this.index.remove(key);
  }

  /** Which tiles should be loaded around a point, nearest first. */
  private wanted(x: number, z: number): { x: number; y: number; d: number }[] {
    const frame = this.frame!;
    const r = this.quality.loadRadius;
    const t = frame.tileAt(x, z);
    const span = Math.ceil(r / frame.metersPerTile) + 1;
    const out: { x: number; y: number; d: number }[] = [];
    const max = 2 ** TILE_ZOOM;
    for (let ty = Math.floor(t.y) - span; ty <= Math.floor(t.y) + span; ty++) {
      for (let tx = Math.floor(t.x) - span; tx <= Math.floor(t.x) + span; tx++) {
        if (ty < 0 || ty >= max) continue;
        const wx = ((tx % max) + max) % max;
        const dx = Math.max(tx - t.x, 0, t.x - (tx + 1)) * frame.metersPerTile;
        const dy = Math.max(ty - t.y, 0, t.y - (ty + 1)) * frame.metersPerTile;
        const d = Math.hypot(dx, dy);
        if (d <= r) out.push({ x: wx, y: ty, d });
      }
    }
    return out.sort((a, b) => a.d - b.d);
  }

  private stream(x: number, z: number) {
    const want = this.wanted(x, z);
    const keep = new Set(want.map((w) => tileKey(w.x, w.y)));
    for (const w of want) {
      const key = tileKey(w.x, w.y);
      if (this.tiles.has(key) || this.pending.has(key) || this.pending.size >= 6) continue;
      this.pending.add(key);
      this.send({ type: 'load', x: w.x, y: w.y, epoch: this.epoch });
    }
    const frame = this.frame!;
    const t = frame.tileAt(x, z);
    for (const [key, tile] of this.tiles) {
      if (keep.has(key)) continue;
      const [, sx, sy] = key.split('/').map(Number);
      const dx = Math.max(sx - t.x, 0, t.x - (sx + 1)) * frame.metersPerTile;
      const dy = Math.max(sy - t.y, 0, t.y - (sy + 1)) * frame.metersPerTile;
      if (Math.hypot(dx, dy) > this.quality.loadRadius + 500) this.drop(tile.key);
    }
  }

  private flatMesh(m: TileMesh['ground'], material: THREE.Material, order: number) {
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(m.pos, 3));
    g.setAttribute('color', new THREE.BufferAttribute(m.col, 3, true));
    g.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(m.pos.length).map((_, i) => (i % 3 === 1 ? 1 : 0)), 3));
    g.setIndex(new THREE.BufferAttribute(m.idx, 1));
    g.computeBoundingSphere();
    const mesh = new THREE.Mesh(g, material);
    mesh.renderOrder = order;
    mesh.matrixAutoUpdate = false;
    return mesh;
  }

  private instanced(geometry: THREE.BufferGeometry, material: THREE.Material, spots: Float32Array, place: (m: THREE.Object3D, a: number, b: number, c: number) => void) {
    const count = spots.length / 3;
    if (!count) return null;
    const mesh = new THREE.InstancedMesh(geometry, material, count);
    const dummy = new THREE.Object3D();
    for (let i = 0; i < count; i++) {
      place(dummy, spots[i * 3], spots[i * 3 + 1], spots[i * 3 + 2]);
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
    mesh.computeBoundingSphere();
    mesh.matrixAutoUpdate = false;
    return mesh;
  }

  private add(t: TileMesh) {
    const key = tileKey(t.x, t.y);
    if (this.tiles.has(key)) return;
    const group = new THREE.Group();
    group.matrixAutoUpdate = false;
    if (t.ground.idx.length) group.add(this.flatMesh(t.ground, this.materials.ground, -3));
    if (t.roads.idx.length) group.add(this.flatMesh(t.roads, this.materials.roads, -2));
    const chunks: Chunk[] = [];
    for (const c of t.buildings) {
      const g = new THREE.BufferGeometry();
      g.setAttribute('position', new THREE.BufferAttribute(c.pos, 3));
      g.setAttribute('normal', new THREE.BufferAttribute(c.nor, 3));
      g.setAttribute('color', new THREE.BufferAttribute(c.col, 3, true));
      g.setAttribute('uv', new THREE.BufferAttribute(c.uv, 2));
      g.setIndex(new THREE.BufferAttribute(c.idx, 1));
      g.computeBoundingSphere();
      const mesh = new THREE.Mesh(g, this.materials.buildings);
      mesh.matrixAutoUpdate = false;
      group.add(mesh);
      chunks.push({ mesh, tall: c.tallCount, all: c.idx.length, cx: c.cx, cz: c.cz, radius: c.radius });
    }
    const lamps = this.instanced(this.lampGeo, this.materials.lamps, t.lamps, (d, x, z, h) => {
      d.position.set(x, 0, z);
      d.rotation.set(0, h, 0);
      d.scale.setScalar(1);
    });
    const heads = lamps ? this.instanced(this.headGeo, this.materials.heads, t.lamps, (d, x, z, h) => {
      d.position.set(x, 0, z);
      d.rotation.set(0, h, 0);
    }) : null;
    const trees = this.quality.trees
      ? this.instanced(this.treeGeo, this.materials.trees, t.trees, (d, x, z, s) => {
          d.position.set(x, 0, z);
          d.rotation.set(0, (x * 13.7 + z) % 6.28, 0);
          d.scale.setScalar(s);
        })
      : null;
    for (const m of [lamps, heads, trees]) if (m) group.add(m);
    let minX = Infinity, minZ = Infinity, maxX = -Infinity, maxZ = -Infinity;
    const frame = this.frame!;
    for (const [px, py] of [[t.x, t.y], [t.x + 1, t.y + 1]]) {
      const [wx, wz] = frame.fromTile(px, py, 1, 0, 0);
      minX = Math.min(minX, wx);
      maxX = Math.max(maxX, wx);
      minZ = Math.min(minZ, wz);
      maxZ = Math.max(maxZ, wz);
    }
    const tile: LoadedTile = { key, group, chunks, lamps, heads, trees, imagery: null, minX, minZ, maxX, maxZ, stats: t.stats };
    this.tiles.set(key, tile);
    this.index.add(key, t);
    this.applySatellite(tile);
    this.scene.add(group);
  }

  /** Sentinel-2 cloudless 2016 by EOX under the tile, in place of the drawn land use. */
  private applySatellite(tile: LoadedTile) {
    const ground = tile.group.children.find((c) => (c as THREE.Mesh).material === this.materials.ground);
    if (ground) ground.visible = !this.satellite;
    if (!this.satellite) {
      if (tile.imagery) tile.imagery.visible = false;
      return;
    }
    if (tile.imagery) {
      tile.imagery.visible = true;
      return;
    }
    const [, x, y] = tile.key.split('/');
    const url = `https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless_3857/default/g/${TILE_ZOOM}/${y}/${x}.jpg`;
    const texture = this.textureLoader.load(url);
    texture.colorSpace = THREE.SRGBColorSpace;
    const w = tile.maxX - tile.minX;
    const d = tile.maxZ - tile.minZ;
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(w, d).rotateX(-Math.PI / 2), new THREE.MeshBasicMaterial({ map: texture, depthWrite: false }));
    plane.position.set(tile.minX + w / 2, 0.01, tile.minZ + d / 2);
    plane.updateMatrix();
    plane.matrixAutoUpdate = false;
    plane.renderOrder = -3;
    tile.imagery = plane;
    tile.group.add(plane);
  }

  /** Culls and thins building chunks by distance from the focus. */
  private lod(x: number, z: number) {
    const { viewDistance, detailDistance } = this.quality;
    for (const tile of this.tiles.values()) {
      for (const c of tile.chunks) {
        const d = Math.hypot(c.cx - x, c.cz - z) - c.radius;
        if (d > viewDistance) c.mesh.visible = false;
        else if (d > detailDistance) {
          c.mesh.visible = c.tall > 0;
          c.mesh.geometry.setDrawRange(0, c.tall);
        } else {
          c.mesh.visible = true;
          c.mesh.geometry.setDrawRange(0, c.all);
        }
      }
      const near = Math.max(0, Math.hypot(Math.max(tile.minX - x, 0, x - tile.maxX), Math.max(tile.minZ - z, 0, z - tile.maxZ)));
      for (const m of [tile.lamps, tile.heads, tile.trees]) if (m) m.visible = near < detailDistance;
    }
  }

  /** Lights, sky and window glow for an hour of the day. */
  setHour(hour: number) {
    const elevation = Math.sin(((hour - 6) / 12) * Math.PI);
    const night = THREE.MathUtils.clamp((0.12 - elevation) / 0.3, 0, 1);
    const dusk = THREE.MathUtils.clamp(1 - Math.abs(elevation - 0.08) / 0.3, 0, 1);
    this.night = night;
    const sky = SKY.day.clone().lerp(SKY.dusk, dusk * (1 - night)).lerp(SKY.night, night);
    (this.scene.background as THREE.Color).copy(sky);
    (this.scene.fog as THREE.Fog).color.copy(sky);
    const azimuth = ((hour - 12) / 12) * Math.PI;
    this.sun.position.set(Math.sin(azimuth) * 400, Math.max(30, elevation * 600), -Math.cos(azimuth) * 250);
    this.sun.intensity = Math.max(0, elevation) * 1.6 + 0.05;
    this.sun.color.set(dusk > 0.5 ? 0xffc28a : 0xffffff);
    this.hemi.intensity = 0.32 + (1 - night) * 0.75;
    this.materials.buildings.emissiveIntensity = night * 0.9;
    this.materials.heads.color.set(night > 0.3 ? 0xffe2a8 : 0x55585e);
  }

  /** Streams, uploads at most one tile, and draws. */
  render(dt: number, focusX: number, focusZ: number) {
    if (!this.frame) return;
    this.stream(focusX, focusZ);
    const next = this.arrived.shift();
    if (next) this.add(next);
    this.lodClock -= dt;
    if (this.lodClock <= 0 || next) {
      this.lodClock = 0.25;
      this.lod(focusX, focusZ);
    }
    this.base.position.set(Math.round(focusX / 50) * 50, 0, Math.round(focusZ / 50) * 50);
    this.renderer.info.reset();
    const timer = this.timer;
    const query = timer && timer.queries.length < 4 ? timer.gl.createQuery() : null;
    if (timer && query) timer.gl.beginQuery(timer.ext.TIME_ELAPSED_EXT, query);
    this.renderer.render(this.scene, this.camera);
    if (timer && query) {
      timer.gl.endQuery(timer.ext.TIME_ELAPSED_EXT);
      timer.queries.push(query);
    }
    this.readTimers();
  }

  /** Starts measuring GPU time per frame, for the stats overlay and benchmarks. */
  measureGpu() {
    if (this.timer) return;
    const gl = this.renderer.getContext() as WebGL2RenderingContext;
    const ext = gl.getExtension('EXT_disjoint_timer_query_webgl2');
    if (ext) this.timer = { ext, gl, queries: [] };
  }

  private readTimers() {
    const t = this.timer;
    if (!t) return;
    while (t.queries.length) {
      const q = t.queries[0];
      if (!t.gl.getQueryParameter(q, t.gl.QUERY_RESULT_AVAILABLE)) break;
      t.queries.shift();
      const ns = t.gl.getQueryParameter(q, t.gl.QUERY_RESULT) as number;
      t.gl.deleteQuery(q);
      if (t.gl.getParameter(t.ext.GPU_DISJOINT_EXT)) continue;
      this.gpuSamples.push(ns / 1e6);
      if (this.gpuSamples.length > 30) this.gpuSamples.shift();
      this.gpuMs = Math.round((this.gpuSamples.reduce((a, b) => a + b, 0) / this.gpuSamples.length) * 100) / 100;
    }
  }

  stats(): WorldStats {
    let buildings = 0;
    for (const t of this.tiles.values()) buildings += t.stats.buildings;
    return {
      tiles: this.tiles.size,
      loading: this.loading,
      calls: this.renderer.info.render.calls,
      triangles: this.renderer.info.render.triangles,
      buildings,
      failed: this.failed,
    };
  }

  dispose() {
    this.worker?.terminate();
    for (const key of [...this.tiles.keys()]) this.drop(key);
    this.renderer.dispose();
  }
}
