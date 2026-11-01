<script lang="ts">
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs, workflow }: Props = $props();
  let chip = $derived(typeof display.type !== 'string' && 'Chip' in display.type ? display.type.Chip : null);
  let Icon = $derived(chip?.icon ? LUCIDE_REGISTRY[interpolate(inputs, chip.icon)] : undefined);
  let state = $derived(look.stateOf(chip?.state, inputs));
  let on = $derived(state === 'Selected');
  const Cross = LUCIDE_REGISTRY['x'];
  const press = pressable(() => ({ workflow, inputs }));
  let disabled = $derived(state === 'Disabled' || press.disabled(chip?.act));
</script>

{#if chip?.act}
  <button
    type="button"
    class={[look.chipToggle[on ? 'on' : 'off'], press.pending && 'cursor-progress opacity-60', disabled && 'cursor-not-allowed opacity-50']}
    aria-pressed={on}
    {disabled}
    onclick={() => press.run(chip.act)}
  >
    {#if Icon}<Icon class="size-4 shrink-0" />{/if}
    <span>{interpolate(inputs, chip.text)}</span>
    {#if on && Cross}<Cross class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />{/if}
  </button>
{:else if chip}
  <div class={look.chip}>
    {#if Icon}<Icon class="size-4 shrink-0 text-primary" />{/if}
    <span>{interpolate(inputs, chip.text)}</span>
  </div>
{/if}
