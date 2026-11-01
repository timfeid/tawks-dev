<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import * as look from '../../look';
  import WorkflowDisplay from '../workflow-display.svelte';
  import DockTabs from './dock-tabs.svelte';

  let {
    workflow,
    tabs,
    actions,
  }: { workflow: WorkflowResource; tabs: any[]; actions: any[] } = $props();

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === 'string'
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join('+');
    return `${index}:${typeKey}`;
  }
</script>

{#if tabs.length || actions.length}
  <div class={[look.dock.bar, !actions.length && look.dock.phoneOnly]}>
    {#if tabs.length}
      <div class={look.dock.phoneTabs}>
        <DockTabs {workflow} visuals={tabs} layout="phone" />
      </div>
    {/if}
    {#if actions.length}
      <div class={look.dock.actions}>
        {#each actions as visual (visual.id)}
          {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
            <WorkflowDisplay from="dock-actions" {workflow} {display} inputs={workflow.responses} />
          {/each}
        {/each}
      </div>
    {/if}
  </div>
{/if}
