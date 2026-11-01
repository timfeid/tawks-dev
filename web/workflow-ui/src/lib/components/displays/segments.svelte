<script lang="ts">
  import { getNestedValue, interpolate } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let segments = $derived(typeof display.type !== 'string' && 'Segments' in display.type ? display.type.Segments : null);
  let items = $derived.by(() => {
    const list = segments ? getNestedValue(inputs, segments.items_key) : null;
    return Array.isArray(list) ? list : [];
  });
</script>

{#if segments && items.length}
  <div class="flex min-w-0 flex-col gap-1.5">
    <div class="flex gap-1">
      {#each items as item}
        <div class={['h-1.5 flex-1 rounded-full', look.segmentState[look.stateOf(segments.state, { ...inputs, [segments.as]: item })]]}></div>
      {/each}
    </div>
    {#if segments.label}<div class={look.text('Fine', 'Default')}>{interpolate(inputs, segments.label)}</div>{/if}
  </div>
{/if}
