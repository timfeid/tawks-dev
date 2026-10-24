<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let text = $derived.by(() => {
    if (typeof display.type === 'string' || !('Time' in display.type)) return;
    const time = display.type.Time;
    const date = getNestedValue(inputs, time.key) ?? interpolate(inputs, time.key);
    return formatDate(date, time.opts);
  });
</script>

{#if text}
  <div class={display.class}>{text}</div>
{/if}
