<script lang="ts">
  import type { Pet } from './catalog';
  import PetHatch from './pet-hatch.svelte';

  /** A 3D egg that wobbles, cracks and hatches into `pet`, with its name beside it. `still` skips straight to the pet. */
  let { pet, still = false, onhatch }: { pet: Pet; still?: boolean; onhatch?: () => void } = $props();

  let hatched = $state(false);
  let hurry = $state(false);
</script>

<div class="flex items-center gap-4 rounded-lg border border-white/10 p-3" aria-live="polite">
  <button
    type="button"
    class="relative size-24 shrink-0 rounded-full"
    style:background="radial-gradient(circle at 50% 70%, color-mix(in srgb, {pet.body} 30%, transparent), transparent 70%)"
    onclick={() => (hurry = true)}
    disabled={hatched}
    aria-label={hatched ? pet.name : 'Hatch your pet'}
  >
    <PetHatch
      {pet}
      {still}
      hatchNow={hurry}
      onhatch={() => {
        hatched = true;
        onhatch?.();
      }}
    />
  </button>
  <div class="flex min-w-0 flex-col gap-0.5">
    <span class="text-muted-foreground text-xs font-medium">Your pet</span>
    {#if hatched}
      <span class="text-foreground text-sm font-semibold">{pet.name} <span class="text-muted-foreground font-normal">· {pet.kind} type</span></span>
      <span class="text-muted-foreground text-xs">{pet.blurb}</span>
    {:else}
      <span class="text-foreground text-sm font-semibold">Something is hatching</span>
      <span class="text-muted-foreground text-xs">Tap the egg if you can't wait.</span>
    {/if}
  </div>
</div>
