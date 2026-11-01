<script lang="ts">
  import { onMount } from 'svelte';
  import type { Pet } from './catalog';
  import PetArt from './pet-art.svelte';

  /** An egg that wobbles, cracks and hatches into `pet`. `still` skips straight to the pet. */
  let { pet, still = false }: { pet: Pet; still?: boolean } = $props();

  let hatched = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    if (still || reduced) hatched = true;
    else timer = setTimeout(hatch, 1800);
    return () => clearTimeout(timer);
  });

  function hatch() {
    clearTimeout(timer);
    hatched = true;
  }
</script>

<div class="flex items-center gap-4 rounded-lg border border-white/10 p-3" aria-live="polite">
  <button
    type="button"
    class="relative grid size-24 shrink-0 place-items-center"
    onclick={hatch}
    disabled={hatched}
    aria-label={hatched ? pet.name : 'Hatch your pet'}
  >
    {#if hatched}
      <span class="burst absolute inset-0 rounded-full" style:--pet={pet.body}></span>
      <span class="pop"><PetArt {pet} /></span>
    {:else}
      <svg class="egg size-20" viewBox="0 0 100 100" aria-hidden="true">
        <path d="M50 8 C74 8 86 46 86 62 C86 82 70 94 50 94 C30 94 14 82 14 62 C14 46 26 8 50 8Z" fill="#fff8ec" stroke="rgba(0,0,0,0.25)" stroke-width="2" />
        <circle cx="36" cy="44" r="6" fill="#ffd23f" />
        <circle cx="62" cy="34" r="4" fill="#4d96ff" />
        <circle cx="64" cy="66" r="7" fill="#ff6f9f" />
        <circle cx="34" cy="74" r="4" fill="#62c370" />
        <path class="crack" d="M22 56 L34 50 L42 60 L52 50 L60 60 L70 52 L80 58" fill="none" stroke="#1b1b2b" stroke-width="2.5" stroke-linejoin="round" />
      </svg>
    {/if}
  </button>
  <div class="flex min-w-0 flex-col gap-0.5">
    {#if hatched}
      <span class="text-muted-foreground text-xs font-medium">Your pet</span>
      <span class="text-foreground text-sm font-semibold">{pet.name} <span class="text-muted-foreground font-normal">· {pet.kind} type</span></span>
      <span class="text-muted-foreground text-xs">{pet.blurb}</span>
    {:else}
      <span class="text-muted-foreground text-xs font-medium">Your pet</span>
      <span class="text-foreground text-sm font-semibold">Something is hatching</span>
      <span class="text-muted-foreground text-xs">Tap the egg if you can't wait.</span>
    {/if}
  </div>
</div>

<style>
  .egg {
    transform-origin: 50% 90%;
    animation: wobble 0.6s ease-in-out infinite;
  }
  .crack {
    stroke-dasharray: 90;
    stroke-dashoffset: 90;
    animation: crack 1.6s ease-in forwards;
  }
  .pop {
    animation: pop 0.55s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }
  .burst {
    background: radial-gradient(circle, var(--pet) 0%, transparent 70%);
    animation: burst 0.7s ease-out forwards;
  }
  @keyframes wobble {
    0%, 100% { transform: rotate(0deg); }
    25% { transform: rotate(-7deg); }
    75% { transform: rotate(7deg); }
  }
  @keyframes crack {
    60% { stroke-dashoffset: 90; }
    100% { stroke-dashoffset: 0; }
  }
  @keyframes pop {
    from { transform: scale(0.2) translateY(20%); opacity: 0; }
    to { transform: scale(1) translateY(0); opacity: 1; }
  }
  @keyframes burst {
    from { transform: scale(0.3); opacity: 0.9; }
    to { transform: scale(1.4); opacity: 0; }
  }
</style>
