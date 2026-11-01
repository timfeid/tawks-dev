<script lang="ts">
  import { onDestroy } from 'svelte';
  import { X } from '@lucide/svelte';
  import { ACTIONS, PAD_BUTTONS, gamepad, keyName, resetSettings, updateSettings, watchSettings, type Action, type Settings } from './settings';

  let { onclose }: { onclose: () => void } = $props();

  type Tab = 'controls' | 'controller' | 'sound' | 'camera';
  const TABS: { tab: Tab; label: string }[] = [
    { tab: 'controls', label: 'Controls' },
    { tab: 'controller', label: 'Controller' },
    { tab: 'sound', label: 'Sound' },
    { tab: 'camera', label: 'Camera' },
  ];
  let tab = $state<Tab>('controls');
  let current = $state<Settings | null>(null);
  const stop = watchSettings((next) => (current = next));
  onDestroy(() => {
    stop();
  });

  // Rebinding: the next key pressed takes the slot; Escape gives up.
  let waiting = $state<{ action: Action; slot: number } | null>(null);
  const onKey = (event: KeyboardEvent) => {
    if (!waiting) {
      if (event.code === 'Escape') onclose();
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    const { action, slot } = waiting;
    waiting = null;
    if (event.code === 'Escape') return;
    updateSettings((draft) => {
      const keys = [...draft.keys[action]];
      keys[slot] = event.code;
      draft.keys[action] = keys.filter(Boolean);
    });
  };
  const clear = (action: Action, slot: number) =>
    updateSettings((draft) => {
      draft.keys[action] = draft.keys[action].filter((_, index) => index !== slot);
    });

  // Rebinding a controller button: the next button pressed takes it.
  let padWaiting = $state<'action' | 'jump' | 'shoulder' | 'blow' | null>(null);
  let padName = $state<string | null>(null);
  let pressed = $state<number[]>([]);
  let frame = 0;
  const poll = () => {
    const pad = gamepad();
    padName = pad?.id ?? null;
    const down = pad ? pad.buttons.flatMap((button, index) => (button.pressed ? [index] : [])) : [];
    if (padWaiting && down.length && !pressed.length) {
      const which = padWaiting;
      padWaiting = null;
      updateSettings((draft) => {
        draft.pad[which] = down[0];
      });
    }
    pressed = down;
    frame = requestAnimationFrame(poll);
  };
  frame = requestAnimationFrame(poll);
  onDestroy(() => cancelAnimationFrame(frame));

  const groups = ['Move', 'Act'] as const;
  const buttonName = (index: number) => PAD_BUTTONS[index] ?? `Button ${index + 1}`;
</script>

<!-- Caught before the game's own keys, so a key being bound does not also fly the camera. -->
<svelte:window onkeydowncapture={onKey} />

<div class="pointer-events-auto absolute inset-0 z-40 flex items-center justify-center bg-black/50 p-4" role="presentation" onclick={onclose}>
  <div
    class="flex max-h-full w-[min(36rem,100%)] flex-col overflow-hidden rounded-2xl border border-white/10 bg-slate-900/95 text-white shadow-2xl"
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={() => {}}
  >
    <div class="flex items-center justify-between border-b border-white/10 px-4 py-3">
      <div class="text-sm font-black uppercase tracking-[0.16em]">Settings</div>
      <button class="rounded-lg p-1 text-white/60 hover:bg-white/10 hover:text-white" aria-label="Close" onclick={onclose}><X class="size-4" /></button>
    </div>
    <div class="flex gap-1 border-b border-white/10 px-3 py-2" role="tablist">
      {#each TABS as one (one.tab)}
        <button
          class="rounded-lg px-3 py-1.5 text-sm font-semibold {tab === one.tab ? 'bg-amber-400 text-slate-900' : 'text-white/70 hover:bg-white/10'}"
          role="tab"
          aria-selected={tab === one.tab}
          onclick={() => (tab = one.tab)}>{one.label}</button
        >
      {/each}
    </div>

    {#if current}
      <div class="min-h-0 flex-1 overflow-y-auto px-4 py-3 text-sm">
        {#if tab === 'controls'}
          {#each groups as group (group)}
            <div class="mb-1 mt-2 text-[11px] font-bold uppercase tracking-[0.16em] text-white/45">{group}</div>
            {#each ACTIONS.filter((one) => one.group === group) as one (one.action)}
              <div class="flex items-center justify-between gap-3 py-1">
                <span class="text-white/85">{one.label}</span>
                <div class="flex gap-1.5">
                  {#each [0, 1] as slot (slot)}
                    {@const code = current.keys[one.action][slot]}
                    {@const listening = waiting?.action === one.action && waiting.slot === slot}
                    <button
                      class="min-w-20 rounded-lg border px-2 py-1 text-xs font-semibold
                        {listening ? 'border-amber-300 bg-amber-300/20 text-amber-200' : 'border-white/10 bg-white/5 hover:bg-white/10'}"
                      title={code ? 'Click to change, right click to clear' : 'Click to set'}
                      onclick={() => (waiting = { action: one.action, slot })}
                      oncontextmenu={(event) => {
                        event.preventDefault();
                        if (code) clear(one.action, slot);
                      }}
                    >
                      {listening ? 'Press a key…' : code ? keyName(code) : '—'}
                    </button>
                  {/each}
                </div>
              </div>
            {/each}
          {/each}
          <p class="mt-3 text-xs text-white/45">Click a key to change it, then press the new one. Right click clears it. Escape gives up.</p>
        {:else if tab === 'controller'}
          <div class="rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-xs {padName ? 'text-emerald-200' : 'text-white/55'}">
            {padName ? `Connected: ${padName}` : 'No controller found. Plug one in and press any button.'}
          </div>
          <p class="mt-2 text-xs text-white/55">The left stick moves. While aiming, the right stick turns and tips the throw.</p>
          {#each [['action', 'Action, throw, swing, stop'], ['jump', 'Jump'], ['shoulder', 'Swap shoulder'], ['blow', 'Blow with the vacuum']] as const as [which, label] (which)}
            <div class="flex items-center justify-between gap-3 py-1.5">
              <span class="text-white/85">{label}</span>
              <button
                class="min-w-32 rounded-lg border px-2 py-1 text-xs font-semibold
                  {padWaiting === which ? 'border-amber-300 bg-amber-300/20 text-amber-200' : 'border-white/10 bg-white/5 hover:bg-white/10'}"
                onclick={() => (padWaiting = which)}
              >
                {padWaiting === which ? 'Press a button…' : buttonName(current.pad[which])}
              </button>
            </div>
          {/each}
          <label class="mt-2 block">
            <span class="flex justify-between text-white/85">Stick dead zone <span class="tabular-nums text-white/55">{current.pad.deadzone.toFixed(2)}</span></span>
            <input
              type="range"
              min="0.05"
              max="0.5"
              step="0.01"
              value={current.pad.deadzone}
              oninput={(event) => updateSettings((draft) => (draft.pad.deadzone = Number(event.currentTarget.value)))}
              class="mt-1 w-full accent-amber-400"
            />
          </label>
          {#if pressed.length}
            <p class="mt-2 text-xs text-white/55">Pressed: {pressed.map(buttonName).join(', ')}</p>
          {/if}
        {:else if tab === 'sound'}
          {#each [['master', 'Master'], ['effects', 'Effects'], ['jingles', 'Jingles'], ['ambience','City ambience']] as const as [which, label] (which)}
            <label class="block py-1.5">
              <span class="flex justify-between text-white/85">{label} <span class="tabular-nums text-white/55">{Math.round(current.sound[which] * 100)}%</span></span>
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={current.sound[which]}
                oninput={(event) => updateSettings((draft) => (draft.sound[which] = Number(event.currentTarget.value)))}
                class="mt-1 w-full accent-amber-400"
              />
            </label>
          {/each}
          <p class="mt-1 text-xs text-white/45">The speaker button mutes everything at once.</p>
        {:else}
          <label class="block py-1.5">
            <span class="flex justify-between text-white/85">Look sensitivity <span class="tabular-nums text-white/55">{current.camera.sensitivity.toFixed(1)}×</span></span>
            <input
              type="range"
              min="0.3"
              max="2.5"
              step="0.1"
              value={current.camera.sensitivity}
              oninput={(event) => updateSettings((draft) => (draft.camera.sensitivity = Number(event.currentTarget.value)))}
              class="mt-1 w-full accent-amber-400"
            />
          </label>
          <label class="block py-1.5">
            <span class="flex justify-between text-white/85">Throw camera width <span class="tabular-nums text-white/55">{current.camera.throwFov}°</span></span>
            <input
              type="range"
              min="50"
              max="100"
              step="1"
              value={current.camera.throwFov}
              oninput={(event) => updateSettings((draft) => (draft.camera.throwFov = Number(event.currentTarget.value)))}
              class="mt-1 w-full accent-amber-400"
            />
          </label>
          <label class="flex items-center justify-between py-2">
            <span class="text-white/85">Invert looking up and down</span>
            <input
              type="checkbox"
              checked={current.camera.invertY}
              onchange={(event) => updateSettings((draft) => (draft.camera.invertY = event.currentTarget.checked))}
              class="size-4 accent-amber-400"
            />
          </label>
        {/if}
      </div>
    {/if}

    <div class="flex justify-between border-t border-white/10 px-4 py-3">
      <button class="rounded-lg px-3 py-1.5 text-xs font-semibold text-white/60 hover:bg-white/10 hover:text-white" onclick={resetSettings}>Reset to defaults</button>
      <button class="rounded-lg bg-amber-400 px-4 py-1.5 text-sm font-bold text-slate-900 hover:bg-amber-300" onclick={onclose}>Done</button>
    </div>
  </div>
</div>
