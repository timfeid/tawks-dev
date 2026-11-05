<script lang="ts">
  import { Plus, RotateCcw, X } from '@lucide/svelte';
  import { onDestroy } from 'svelte';
  import Calibrate from './Calibrate.svelte';
  import { DrumKit } from '../lib/drums';
  import {
    CONTROLS,
    CONTROL_LABELS,
    Controls,
    DEFAULT_BINDINGS,
    captureBinding,
    connectedPads,
    describeBinding,
    type Control,
    type Layout,
  } from '../lib/input';
  import type { RoomConnection } from '../lib/room.svelte';
  import type { Settings } from '../lib/settings';

  let { conn, settings, onClose }: { conn: RoomConnection; settings: Settings; onClose: () => void } = $props();

  let draft = $state<Settings>(structuredClone($state.snapshot(settings)));
  let layout = $state<Layout>(conn.me?.part?.instrument === 'drums' ? 'drums' : 'guitar');
  let capturing = $state<Control | null>(null);
  let held = $state<Set<Control>>(new Set());
  let pads = $state<string[]>(connectedPads());
  let step = $state<Control | null>(null);

  let abort: AbortController | null = null;
  let tester: Controls | null = null;
  let kit: DrumKit | null = null;
  let stepAbort: AbortController | null = null;
  let skipping = false;

  function listen() {
    tester?.stop();
    tester = new Controls($state.snapshot(draft.bindings[layout]), (control, down) => {
      if (layout === 'drums' && down && control.startsWith('lane')) {
        kit ??= new DrumKit();
        kit.setVolume(draft.drumVolume);
        kit.play(Number(control.slice(4)));
      }
      const next = new Set(held);
      if (down) next.add(control);
      else next.delete(control);
      held = next;
    });
    tester.start();
  }

  $effect(() => {
    void layout;
    void draft.bindings[layout];
    if (!capturing && !step) listen();
  });

  const padTimer = setInterval(() => (pads = connectedPads()), 1000);
  onDestroy(() => {
    clearInterval(padTimer);
    tester?.stop();
    kit?.dispose();
    abort?.abort();
    stepAbort?.abort();
  });

  async function add(control: Control) {
    abort?.abort();
    abort = new AbortController();
    tester?.stop();
    capturing = control;
    const binding = await captureBinding(abort.signal);
    capturing = null;
    if (binding) draft.bindings[layout][control] = [...draft.bindings[layout][control], binding];
    listen();
  }

  function remove(control: Control, index: number) {
    draft.bindings[layout][control] = draft.bindings[layout][control].filter((_, i) => i !== index);
  }

  function reset() {
    draft.bindings[layout] = structuredClone(DEFAULT_BINDINGS[layout]);
  }

  /** Clears the controller's buttons (keys stay) and asks for each control in turn. */
  async function walkThrough() {
    abort?.abort();
    tester?.stop();
    for (const control of CONTROLS[layout]) {
      draft.bindings[layout][control] = draft.bindings[layout][control].filter((b) => b.kind === 'key');
    }
    for (const control of CONTROLS[layout]) {
      step = control;
      skipping = false;
      stepAbort = new AbortController();
      const binding = await captureBinding(stepAbort.signal, true);
      if (binding) draft.bindings[layout][control] = [...draft.bindings[layout][control], binding];
      else if (!skipping) break;
    }
    step = null;
    listen();
  }

  function skipStep() {
    skipping = true;
    stepAbort?.abort();
  }

  function save() {
    conn.saveSettings($state.snapshot(draft));
    onClose();
  }
</script>

<div class="absolute inset-0 z-20 flex items-center justify-center bg-black/70 p-4">
  <div class="flex max-h-full w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-white/10 bg-background shadow-2xl">
    <header class="flex items-center border-b border-white/10 px-5 py-3">
      <h2 class="font-semibold">Controller and calibration</h2>
      <button class="ml-auto rounded-md p-1 text-neutral-400 hover:bg-white/10 hover:text-white" onclick={onClose} aria-label="Close">
        <X class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 space-y-6 overflow-y-auto p-5">
      <section class="space-y-2">
        <div class="flex items-center gap-2">
          {#each ['guitar', 'drums'] as const as option (option)}
            <button
              class="rounded-full border px-3 py-1 text-xs capitalize {layout === option
                ? 'border-primary bg-primary/15'
                : 'border-white/10 text-neutral-400 hover:text-white'}"
              onclick={() => (layout = option)}
            >
              {option === 'guitar' ? 'Guitar, bass and keys' : 'Drums'}
            </button>
          {/each}
          <button
            class="ml-auto rounded-full bg-primary px-3 py-1 text-xs font-semibold text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            disabled={!!step}
            onclick={walkThrough}
          >
            Set up controller
          </button>
          <button class="flex items-center gap-1 text-xs text-neutral-400 hover:text-white" onclick={reset}>
            <RotateCcw class="size-3.5" /> Defaults
          </button>
        </div>
        {#if step}
          <div class="flex items-center gap-3 rounded-lg border border-primary/40 bg-primary/10 px-3 py-2.5">
            <span class="text-sm">
              Press the <span class="font-semibold text-primary">{CONTROL_LABELS[layout][step]?.toLowerCase()}</span>
              on your controller
            </span>
            <span class="text-xs text-neutral-500">
              {CONTROLS[layout].indexOf(step) + 1} of {CONTROLS[layout].length}
            </span>
            <button class="ml-auto text-xs text-neutral-300 hover:text-white" onclick={skipStep}>Skip</button>
            <button class="text-xs text-neutral-300 hover:text-white" onclick={() => stepAbort?.abort()}>Stop</button>
          </div>
        {/if}
        {#if layout === 'drums'}
          <label class="flex items-center gap-3 rounded-lg border border-white/10 px-3 py-2 text-sm">
            <input type="checkbox" bind:checked={draft.autoKick} class="accent-primary" />
            <span>Auto kick</span>
            <span class="text-xs text-neutral-500">Kick notes play themselves. Songs played this way keep no best score.</span>
          </label>
        {/if}
        <p class="text-xs text-neutral-500">
          {pads.length ? `Connected: ${pads.join(', ')}` : 'No controller found. Press a button on it to wake it up.'}
        </p>
        <div class="divide-y divide-white/5 rounded-lg border border-white/10">
          {#each CONTROLS[layout] as control (control)}
            <div class="flex items-center gap-3 px-3 py-2">
              <span class="size-2.5 rounded-full {held.has(control) ? 'bg-primary' : 'bg-white/10'}"></span>
              <span class="w-28 text-sm">{CONTROL_LABELS[layout][control]}</span>
              <div class="flex flex-1 flex-wrap gap-1">
                {#each draft.bindings[layout][control] as binding, i (i)}
                  <span class="flex items-center gap-1 rounded bg-white/10 px-2 py-0.5 font-mono text-xs">
                    {describeBinding(binding)}
                    <button class="text-neutral-400 hover:text-white" onclick={() => remove(control, i)} aria-label="Remove">
                      <X class="size-3" />
                    </button>
                  </span>
                {/each}
              </div>
              <button
                class="flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs {capturing === control
                  ? 'border-primary text-primary'
                  : 'border-white/10 text-neutral-400 hover:text-white'}"
                onclick={() => add(control)}
              >
                {#if capturing === control}Press it now{:else}<Plus class="size-3" /> Bind{/if}
              </button>
            </div>
          {/each}
        </div>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-semibold">Calibration</h3>
        <Calibrate
          {layout}
          bindings={draft.bindings[layout]}
          lookahead={draft.lookahead}
          bind:inputOffset={draft.inputOffset}
          bind:videoOffset={draft.videoOffset}
        />
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Input offset</span>
          <input type="range" min="-200" max="300" step="1" bind:value={draft.inputOffset} class="flex-1" />
          <span class="w-16 text-right font-mono text-xs">{draft.inputOffset} ms</span>
        </label>
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Video offset</span>
          <input type="range" min="-200" max="200" step="1" bind:value={draft.videoOffset} class="flex-1" />
          <span class="w-16 text-right font-mono text-xs">{draft.videoOffset} ms</span>
        </label>
        <p class="text-xs text-neutral-500">
          Input offset corrects when your hits register. Video offset moves the notes: raise it if they cross the line
          after you hear the click.
        </p>
      </section>

      <section class="space-y-2">
        <h3 class="text-sm font-semibold">Highway</h3>
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Note speed</span>
          <input type="range" min="500" max="2500" step="50" bind:value={draft.lookahead} class="flex-1" style="direction: rtl" />
          <span class="w-16 text-right font-mono text-xs">{(1100 / draft.lookahead).toFixed(1)}x</span>
        </label>
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Volume</span>
          <input type="range" min="0" max="1" step="0.05" bind:value={draft.volume} class="flex-1" />
          <span class="w-16 text-right font-mono text-xs">{Math.round(draft.volume * 100)}%</span>
        </label>
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Drum hit sounds</span>
          <input type="range" min="0" max="1" step="0.05" bind:value={draft.drumVolume} class="flex-1" />
          <span class="w-16 text-right font-mono text-xs">{draft.drumVolume ? `${Math.round(draft.drumVolume * 100)}%` : 'Off'}</span>
        </label>
        <label class="flex items-center gap-3 text-sm">
          <span class="w-36 text-neutral-400">Vocal guide</span>
          <input type="checkbox" bind:checked={draft.vocalGuide} class="accent-primary" />
          <span class="text-xs text-neutral-500">When you sing, hear the real vocals while you are on the note.</span>
        </label>
      </section>
    </div>

    <footer class="flex justify-end gap-2 border-t border-white/10 px-5 py-3">
      <button class="rounded-lg px-4 py-1.5 text-sm text-neutral-300 hover:bg-white/10" onclick={onClose}>Cancel</button>
      <button class="rounded-lg bg-white px-4 py-1.5 text-sm font-semibold text-black hover:bg-neutral-200" onclick={save}>Save</button>
    </footer>
  </div>
</div>
