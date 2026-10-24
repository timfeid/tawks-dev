<script lang="ts">
  import * as Tooltip from '@tawks/ui/tooltip/index.js';
  import { onMount } from 'svelte';
  import { getNestedValue, interpolate, resolveInputValue, evaluateCondition, formatDate, cn, debounce, deepEqual, objectInterpolate } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { sanitizeInlineHtml } from '@tawks/editor';

  let { workflow, display, inputs }: Props = $props();

  let tooltip = $derived.by(() => {
    if (!('Tooltip' in display.type)) return;
    return display.type.Tooltip;
  });

  let scrollEl = $state();
  let ref = $state<HTMLElement>(null);

  onMount(() => {
    scrollEl = ref?.closest('[data-scroll-area-content]');
  });
</script>

{#if tooltip}
  {#snippet render()}
    {#each tooltip.view_display as display}
      <WorkflowDisplay from="wf-tooltip" {workflow} {display} {inputs}></WorkflowDisplay>
    {/each}
  {/snippet}

  <Tooltip.Provider>
    <Tooltip.Root>
      <Tooltip.Trigger bind:ref class={display.class}>
        {@render render()}
      </Tooltip.Trigger>

      <Tooltip.Content collisionBoundary={scrollEl} class={tooltip.class}>
        {@html sanitizeInlineHtml(interpolate(inputs, tooltip.tooltip_html))}
      </Tooltip.Content>
    </Tooltip.Root>
  </Tooltip.Provider>
{/if}
