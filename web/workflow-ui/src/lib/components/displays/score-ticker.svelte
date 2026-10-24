<script lang="ts">
  import ScoreTicker from '@tawks/ui/score-ticker/score-ticker.svelte';
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils.js';
  import WorkflowDisplay from '../workflow-display.svelte';
  import type { Props } from '../workflow-display.svelte';
  import Icon from './icon/icon.svelte';

  let { workflow, display, inputs }: Props = $props();

  let scoreTicker = $derived.by(() => {
    if (typeof display.type === 'string' || !('ScoreTicker' in display.type)) return;
    return display.type.ScoreTicker;
  });

  let value = $derived.by(() => {
    if (!scoreTicker) return;
    return getNestedValue(inputs, scoreTicker.value_key) as any as number;
  });
</script>

{#snippet icon(className: string)}
  {#if scoreTicker?.icon}
    <WorkflowDisplay {workflow} display={scoreTicker.icon} {inputs}></WorkflowDisplay>
  {/if}
{/snippet}

{#if scoreTicker && value !== undefined}
  <ScoreTicker
    containerClass={scoreTicker?.container_class || undefined}
    popContainerClass={scoreTicker.pop_container_class || undefined}
    popTextClass={scoreTicker.pop_text_class || undefined}
    class={display.class}
    {value}
    {icon}
  />
{/if}
