<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let text = $derived.by(() => {
    if (typeof display.type === 'string' || !('Text' in display.type)) return;
    let template = display.type.Text.text;
    if (typeof template !== 'string') return;
    return interpolate(inputs, template);
  });
</script>

{#if text}
  <div class={display.class}>{text}</div>
{/if}
