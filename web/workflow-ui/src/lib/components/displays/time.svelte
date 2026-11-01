<script lang="ts">
  import { getNestedValue, interpolate, formatDate } from '../../utils.js';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let text = $derived.by(() => {
    if (typeof display.type === 'string' || !('Time' in display.type)) return;
    const time = display.type.Time;
    const date = getNestedValue(inputs, time.key) ?? interpolate(inputs, time.key);
    return formatDate(date, time.opts);
  });
  let time = $derived(typeof display.type !== 'string' && 'Time' in display.type ? display.type.Time : null);
</script>

{#if text && time}
  <span class={look.text(time.role ?? 'Fine', look.toneOf(time.tone, inputs))}>{text}</span>
{/if}
