<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import { cn } from '../utils.js';
  import PageLoader from './page-loader.svelte';
  import { ReadyGate, setReadyGate } from './ready-gate.svelte.js';

  // spin-delay's defaults: loads under half a second never show a loader, and one that shows doesn't flash.
  const SHOW_AFTER_MS = 500;
  const MIN_SHOWN_MS = 200;

  let {
    key = undefined,
    ready = true,
    capMs = 2000,
    label,
    shown = $bindable(false),
    class: className,
    children,
  }: {
    /** Changing it hides the page again until its parts are ready, as when switching conversations. */
    key?: unknown;
    /** False while the page itself is still fetching what decides its parts. */
    ready?: boolean;
    capMs?: number;
    label?: string;
    /** True once the page is on screen, for parts drawn outside the gate such as a side panel. */
    shown?: boolean;
    class?: string;
    children: Snippet;
  } = $props();

  const gate = setReadyGate(
    new ReadyGate(
      () => key,
      untrack(() => capMs),
    ),
  );

  $effect.pre(() => {
    void key;
    gate.sync();
  });

  $effect(() => {
    void key;
    if (ready) return;
    return gate.hold();
  });

  $effect(() => () => gate.destroy());

  let loaderShown = $state(false);
  let lingering = $state(false);
  let shownAt = 0;

  // A fast load never shows the loader; once it shows, it stays long enough not to flicker.
  $effect.pre(() => {
    if (!gate.ready) {
      lingering = false;
      const timer = setTimeout(() => {
        loaderShown = true;
        shownAt = performance.now();
      }, SHOW_AFTER_MS);
      return () => clearTimeout(timer);
    }
    if (!untrack(() => loaderShown)) return;
    const left = shownAt + MIN_SHOWN_MS - performance.now();
    if (left <= 0) {
      loaderShown = false;
      return;
    }
    lingering = true;
    const timer = setTimeout(() => {
      lingering = false;
      loaderShown = false;
    }, left);
    return () => clearTimeout(timer);
  });

  const visible = $derived(gate.ready && !lingering);
  $effect.pre(() => {
    shown = visible;
  });
</script>

<div class={cn('relative flex h-full min-h-0 w-full min-w-0 flex-1 flex-col', className)} aria-busy={!visible}>
  <div
    class={cn(
      'flex h-full min-h-0 w-full min-w-0 flex-1 flex-col',
      !visible && 'pointer-events-none opacity-0',
    )}
  >
    {@render children()}
  </div>
  {#if loaderShown && !visible}
    <div class="absolute inset-0 flex items-center justify-center">
      <PageLoader {label} />
    </div>
  {/if}
</div>
