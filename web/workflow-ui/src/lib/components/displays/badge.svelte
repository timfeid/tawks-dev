<script lang="ts">
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import { Badge } from '@tawks/ui/badge/index.js';
  import type { Props } from '../workflow-display.svelte';

  let { workflow, display, inputs }: Props = $props();

  let badge = $derived.by(() => {
    if (!('Badge' in display.type)) return;
    return display.type.Badge;
  });
</script>

{#if badge}
  <Badge variant={badge.variant} class={display.class}>{interpolate(inputs, badge.text)}</Badge>
{/if}
