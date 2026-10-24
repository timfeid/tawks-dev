<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import type { Props } from '../workflow-input.svelte';

  let { workflow, input, inputs }: Props = $props();

  let checkbox = $derived.by(() => {
    if (!('Checkbox' in input.input.type)) return;
    return input.input.type.Checkbox;
  });
</script>

{#if checkbox}
  <input type="checkbox" />
{/if}
