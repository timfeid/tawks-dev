<script lang="ts">
  import type { WorkflowResource, WorkflowVisual } from '@tawks/workflow-types';
  import type { WorkflowStateProvider } from '../../workflow';
  import WorkflowDisplay from '../workflow-display.svelte';
  import WorkflowShell from './workflow-shell.svelte';

  let {
    visual,
    workflow,
    provider,
  }: {
    visual: WorkflowVisual;
    workflow: WorkflowResource;
    provider: WorkflowStateProvider | undefined;
  } = $props();

  let injected = $derived(provider?.getActiveInjectedForTile?.(visual.id) ?? null);
</script>

{#if injected && provider}
  {#key injected.instance_id}
    <WorkflowShell workflow={injected} {provider} />
  {/key}
{:else}
  <div id={visual.id} class="workflow-tile">
    {#each visual.display.Tile?.content ?? [] as display}
      <WorkflowDisplay from="body-tile" {workflow} {display} inputs={workflow.responses} />
    {/each}
  </div>
{/if}
