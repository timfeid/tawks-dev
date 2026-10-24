<script lang="ts">
  import { browser } from '$app/environment';
  import { onMount, tick } from 'svelte';

  interface Props {
    load: () => Promise<boolean>;

    loading?: boolean;
    hasMore?: boolean;

    root?: Element | null;
    threshold?: number | number[];
    rootMargin?: string;

    enabled?: boolean;
    auto?: boolean;

    /** NEW: try to load immediately on mount if sentinel is visible */
    initialLoad?: boolean;

    /** NEW: keep loading until content overflows (useful when >9 items fit) */
    fillViewport?: boolean;
  }

  let {
    load,
    loading = $bindable(false),
    hasMore = $bindable(true),

    root = null,
    threshold = 0,
    rootMargin = '200px 0px',

    enabled = true,
    auto = true,

    initialLoad = true,
    fillViewport = true,
  }: Props = $props();

  let sentinel: HTMLElement | null = $state(null);
  let observer: IntersectionObserver | null = null;

  let isIntersecting = $state(false);

  // TEMP debug — remove once pagination is verified end-to-end.
  const TAG = '[InfiniteScroll]';

  /**
   * Wraps the consumer's `load` callback. Bails if disabled, mid-load, or
   * out of pages. After loading, if the sentinel is STILL in view (because
   * the new content didn't push it off-screen, e.g. when fewer items per
   * page than viewport height), it keeps draining until either the sentinel
   * leaves view or there are no more pages. Capped at 8 iterations as a
   * safety net for misbehaving consumers.
   */
  async function loadMore() {
    console.log(TAG, 'loadMore called', { enabled, loading, hasMore });
    if (!enabled || loading || !hasMore) return;

    loading = true;
    try {
      let iterations = 0;
      while (hasMore && iterations < 8) {
        iterations++;
        console.log(TAG, `iteration ${iterations}: calling load()`);
        try {
          hasMore = await load();
        } catch (e) {
          console.error(TAG, 'load() threw', e);
          throw e;
        }
        console.log(TAG, `iteration ${iterations} returned hasMore=${hasMore}`);
        if (!hasMore) break;

        await tick();
        await new Promise((r) => requestAnimationFrame(() => r(null)));
        console.log(TAG, `after paint isIntersecting=${isIntersecting}`);
        if (!isIntersecting) break;
      }
    } finally {
      loading = false;
      console.log(TAG, 'loadMore done', { hasMore });
    }
  }

  function handleIntersection(entries: IntersectionObserverEntry[]) {
    const entry = entries[0];
    const next = !!entry?.isIntersecting;
    if (next !== isIntersecting) {
      console.log(TAG, `intersection ${isIntersecting} -> ${next}`);
    }
    isIntersecting = next;
  }

  function connectObserver() {
    if (!browser) return;
    if (!sentinel) return;

    observer?.disconnect();
    observer = new IntersectionObserver(handleIntersection, {
      root,
      threshold,
      rootMargin,
    });
    observer.observe(sentinel);
  }

  function rootIsScrollable(): boolean {
    const r = root as HTMLElement | null;
    if (!r) return false;
    return r.scrollHeight > r.clientHeight + 1;
  }

  async function maybeInitialLoad(_reason: string) {
    if (!enabled || !auto || !initialLoad) return;
    if (loading || !hasMore) return;

    await tick();
    await new Promise((r) => requestAnimationFrame(() => r(null)));

    const shouldLoad =
      isIntersecting || (fillViewport && root && !rootIsScrollable());
    if (!shouldLoad) return;

    await loadMore();
  }

  // NOTE: previously had `export async function reset()` here, but in
  // Svelte 5 runes mode mixing `export function` with `$props()` breaks
  // the component's type codegen — svelte-check ends up treating the
  // exports object ({ reset?: ... }) as the prop type, so consumers
  // get confusing errors like "'load' does not exist in type
  // '{ reset?: ... }'". Removed since nothing actually called it.

  onMount(async () => {
    connectObserver();
    await maybeInitialLoad('mount');
  });

  onMount(() => {
    return () => observer?.disconnect();
  });

  // Reconnect observer when options change, and re-check initial load
  $effect(() => {
    root;
    threshold;
    rootMargin;
    enabled;
    auto;
    if (!browser) return;

    connectObserver();
    maybeInitialLoad('options');
  });

  // Normal auto-load on intersection.
  $effect(() => {
    if (!enabled || !auto) return;
    if (!isIntersecting) return;
    if (loading) return;
    loadMore();
  });
</script>

<div
  bind:this={sentinel}
  aria-hidden="true"
  style="height: 1px; visibility: hidden;"
></div>
