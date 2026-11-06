<script lang="ts">
  import { ChevronLeft, ChevronRight, Megaphone } from '@lucide/svelte';
  import type { Input } from '../lib/input';

  let { input, onhonk }: { input: Input; onhonk: () => void } = $props();

  type Key = 'left' | 'right' | 'gas' | 'brake' | 'drift';

  function hold(key: Key) {
    return {
      onpointerdown: (e: PointerEvent) => {
        input.touch[key] = true;
        try {
          (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
        } catch {
          /* Capture only keeps a sliding finger on the pedal; holding works without it. */
        }
      },
      onpointerup: () => (input.touch[key] = false),
      onpointercancel: () => (input.touch[key] = false),
      onlostpointercapture: () => (input.touch[key] = false),
    };
  }
</script>

<div class="touch">
  <div class="steer">
    <button class="pad" aria-label="Steer left" {...hold('left')}><ChevronLeft size={34} /></button>
    <button class="pad" aria-label="Steer right" {...hold('right')}><ChevronRight size={34} /></button>
  </div>
  <div class="pedals">
    <div class="extras">
      <button class="small" aria-label="Honk" onpointerdown={onhonk}><Megaphone size={20} /></button>
      <button class="small" aria-label="Drift" {...hold('drift')}>Drift</button>
    </div>
    <button class="pedal brake" aria-label="Brake" {...hold('brake')}>Brake</button>
    <button class="pedal gas" aria-label="Gas" {...hold('gas')}>Gas</button>
  </div>
</div>

<style>
  .touch {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(14px + env(safe-area-inset-bottom));
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    padding: 0 12px;
    pointer-events: none;
    z-index: 6;
  }
  .steer,
  .pedals {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    pointer-events: auto;
  }
  button {
    border: 1px solid var(--line-strong);
    background: rgba(14, 19, 26, 0.62);
    color: var(--text);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    touch-action: none;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-size: 13px;
  }
  button:active {
    background: rgba(255, 181, 71, 0.3);
    border-color: var(--signal);
  }
  .pad {
    width: min(72px, 17vw);
    height: min(72px, 17vw);
    border-radius: 50%;
    display: grid;
    place-items: center;
  }
  .extras {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .small {
    width: min(52px, 13vw);
    height: min(52px, 13vw);
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 11px;
  }
  .pedal {
    width: min(66px, 16vw);
    height: 104px;
    border-radius: 16px;
  }
  .gas {
    border-color: rgba(63, 208, 143, 0.6);
  }
  .brake {
    border-color: rgba(255, 91, 74, 0.6);
  }
</style>
