<script lang="ts">
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs, workflow }: Props = $props();
  let group = $derived(typeof display.type !== 'string' && 'ButtonGroup' in display.type ? display.type.ButtonGroup : null);
  const press = pressable(() => ({ workflow, inputs }));
</script>

{#if group}
  <div class={look.buttonGroup.bar} role="group" aria-busy={press.pending}>
    {#each group.buttons as button}
      {@const disabled = press.pending || press.disabled(button.act)}
      <button type="button" class={[look.buttonGroup.button, disabled && 'cursor-not-allowed opacity-50']} {disabled} onclick={() => press.run(button.act)}>
        {interpolate(inputs, button.label)}
      </button>
    {/each}
  </div>
{/if}
