import { getContext, setContext, untrack } from 'svelte';

const CONTEXT = Symbol('ready-gate');

export type Release = () => void;

/**
 * Holds a page back until every part inside it says it is ready, or until `capMs`
 * passes, whichever comes first. Parts take a hold with `hold()` and release it when
 * their first data has landed, failed or all.
 */
export class ReadyGate {
  ready = $state(false);
  #holds = new Set<symbol>();
  #cap: ReturnType<typeof setTimeout> | undefined;
  #frame = 0;
  #armedFor: unknown = undefined;

  constructor(
    private readonly key: () => unknown,
    private readonly capMs = 2000,
  ) {
    this.arm();
  }

  /** Hides the page again for a new subject, such as another conversation. */
  arm() {
    this.#armedFor = this.key();
    this.ready = false;
    if (typeof window === 'undefined') return;
    clearTimeout(this.#cap);
    this.#cap = setTimeout(() => this.#open(), this.capMs);
    this.#check();
  }

  /** Arms again when the key has moved on. */
  sync() {
    if (this.key() !== this.#armedFor) this.arm();
  }

  /** Keeps the page hidden until the returned function is called. Once the page is
   * shown, holds are free and change nothing until the gate is armed again. */
  hold(): Release {
    // Untracked, so an effect that takes a hold does not run again when the page opens.
    return untrack(() => this.#take());
  }

  #take(): Release {
    this.sync();
    if (this.ready) return () => {};
    const token = Symbol();
    this.#holds.add(token);
    return () => {
      if (this.#holds.delete(token)) this.#check();
    };
  }

  destroy() {
    clearTimeout(this.#cap);
    cancelAnimationFrame(this.#frame);
  }

  // Two frames give parts mounted in the same update time to take their holds, and
  // let what released last paint before the page shows.
  #check() {
    cancelAnimationFrame(this.#frame);
    this.#frame = requestAnimationFrame(() => {
      this.#frame = requestAnimationFrame(() => {
        if (this.#holds.size === 0) this.#open();
      });
    });
  }

  #open() {
    clearTimeout(this.#cap);
    cancelAnimationFrame(this.#frame);
    this.#holds.clear();
    this.ready = true;
  }
}

export function setReadyGate(gate: ReadyGate) {
  return setContext(CONTEXT, gate);
}

/** The gate this part sits inside, if any. */
export function useReadyGate(): ReadyGate | undefined {
  return getContext<ReadyGate | undefined>(CONTEXT);
}

/** Resolves once every eager image inside `root` has loaded or failed. */
export function imagesSettled(root: ParentNode | null | undefined): Promise<void> {
  if (!root) return Promise.resolve();
  const pending = [...root.querySelectorAll('img')].filter(
    (img) => !img.complete && img.loading !== 'lazy',
  );
  return Promise.all(
    pending.map(
      (img) =>
        new Promise<void>((resolve) => {
          img.addEventListener('load', () => resolve(), { once: true });
          img.addEventListener('error', () => resolve(), { once: true });
        }),
    ),
  ).then(() => {});
}

/** Waits for the next two frames, so what was just rendered has painted. */
export function afterPaint(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
}
