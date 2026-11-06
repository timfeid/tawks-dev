/**
 * Raw tiles kept between sessions in IndexedDB, capped by size and trimmed
 * oldest first. Activity frames have no origin, so the browser refuses
 * storage there; the cache then lives in memory for the session only.
 */

const DB = 'outroads-tiles';
const STORE = 'tiles';
const MAX_BYTES = 120 * 1024 * 1024;
const MEMORY_TILES = 64;

type Row = { key: string; data: ArrayBuffer; size: number; used: number };

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

export class TileCache {
  private memory = new Map<string, ArrayBuffer>();
  private db: Promise<IDBDatabase | null>;
  private bytes = -1;

  constructor() {
    this.db = this.open();
  }

  get persistent(): Promise<boolean> {
    return this.db.then((db) => !!db);
  }

  private open(): Promise<IDBDatabase | null> {
    try {
      if (typeof indexedDB === 'undefined') return Promise.resolve(null);
      const req = indexedDB.open(DB, 1);
      req.onupgradeneeded = () => {
        const store = req.result.createObjectStore(STORE, { keyPath: 'key' });
        store.createIndex('used', 'used');
      };
      return request(req).catch(() => null);
    } catch {
      return Promise.resolve(null);
    }
  }

  async get(key: string): Promise<ArrayBuffer | null> {
    const hit = this.memory.get(key);
    if (hit) {
      this.memory.delete(key);
      this.memory.set(key, hit);
      return hit;
    }
    const db = await this.db;
    if (!db) return null;
    try {
      const tx = db.transaction(STORE, 'readwrite');
      const store = tx.objectStore(STORE);
      const row = (await request(store.get(key))) as Row | undefined;
      if (!row) return null;
      row.used = Date.now();
      store.put(row);
      this.remember(key, row.data);
      return row.data;
    } catch {
      return null;
    }
  }

  async put(key: string, data: ArrayBuffer) {
    this.remember(key, data);
    const db = await this.db;
    if (!db) return;
    try {
      if (this.bytes < 0) this.bytes = await this.total(db);
      const tx = db.transaction(STORE, 'readwrite');
      tx.objectStore(STORE).put({ key, data, size: data.byteLength, used: Date.now() } satisfies Row);
      this.bytes += data.byteLength;
      if (this.bytes > MAX_BYTES) await this.trim(db);
    } catch {
      /* A full or blocked store only costs a refetch. */
    }
  }

  private remember(key: string, data: ArrayBuffer) {
    this.memory.delete(key);
    this.memory.set(key, data);
    while (this.memory.size > MEMORY_TILES) this.memory.delete(this.memory.keys().next().value!);
  }

  private async total(db: IDBDatabase): Promise<number> {
    const rows = (await request(db.transaction(STORE).objectStore(STORE).getAll())) as Row[];
    return rows.reduce((sum, r) => sum + r.size, 0);
  }

  /** Drops the least recently used tiles until the store is at three quarters of its cap. */
  private async trim(db: IDBDatabase) {
    const tx = db.transaction(STORE, 'readwrite');
    const index = tx.objectStore(STORE).index('used');
    await new Promise<void>((resolve) => {
      const cursor = index.openCursor();
      cursor.onsuccess = () => {
        const c = cursor.result;
        if (!c || this.bytes <= MAX_BYTES * 0.75) return resolve();
        this.bytes -= (c.value as Row).size;
        c.delete();
        c.continue();
      };
      cursor.onerror = () => resolve();
    });
  }
}
