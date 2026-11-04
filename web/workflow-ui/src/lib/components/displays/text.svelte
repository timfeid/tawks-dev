<script lang="ts">
  import { getNestedValue, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils';
  import { shown } from '../../member-names.svelte';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let text = $derived.by(() => {
    if (typeof display.type === 'string' || !('Text' in display.type)) return;
    let template = display.type.Text.text;
    if (typeof template !== 'string') return;
    return shown(inputs, template);
  });
</script>

{#if text}
  <div>{text}</div>
{/if}
