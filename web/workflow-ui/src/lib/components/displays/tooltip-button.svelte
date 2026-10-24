<script lang="ts">
  import type { JsonValue } from '@tawks/workflow-types';
  import * as Tooltip from '@tawks/ui/tooltip/index.js';
  import { cn, interpolate } from '../../utils';
  import { buttonVariants } from '@tawks/ui/button/index.js';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let tooltip = $derived.by(() => {
    if (typeof display.type === 'string' || !('TooltipButton' in display.type)) return;
    return display.type.TooltipButton;
  });

  async function onclick(e: MouseEvent) {
    if (!tooltip) {
      return;
    }

    const requestInputs: Record<string, JsonValue> = {};
    if (tooltip.input_map) {
      for (const [key, val] of Object.entries(tooltip.input_map)) {
        requestInputs[key] = interpolate(inputs, val || '');
      }
    }
    
    if (provider.processAction) {
        await provider.processAction(workflow.instance_id, tooltip.action_id, requestInputs);
    }
  }
</script>

{#if tooltip}
  {#snippet render()}
    {#each tooltip.content as display}
      <WorkflowDisplay
        from="wf-tooltip-button-content"
        {workflow}
        {display}
        {inputs}
      ></WorkflowDisplay>
    {/each}
  {/snippet}

  <Tooltip.Provider>
    <Tooltip.Root>
      <Tooltip.Trigger
        class={cn(
          buttonVariants({
            variant: tooltip.variant as any,
            size: tooltip.size as any,
          }),
          display.class,
        )}
        {onclick}
      >
        {@render render()}
      </Tooltip.Trigger>
      <Tooltip.Content>
        {#each tooltip.tooltip_content as display}
          <WorkflowDisplay
            from="wf-tooltip-button-tooltip"
            {workflow}
            {display}
            {inputs}
          ></WorkflowDisplay>
        {/each}
      </Tooltip.Content>
    </Tooltip.Root>
  </Tooltip.Provider>
{/if}