<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { cn } from '../../utils';
  import WorkflowDisplay from '../workflow-display.svelte';
  import WorkflowNotification from './workflow-notification.svelte';
  import type { WorkflowStateProvider } from '../../workflow';
  import WorkflowBodyTile from './workflow-body-tile.svelte';
  import { getContext } from 'svelte';

  let { workflow }: { workflow: WorkflowResource } = $props();

  const provider = getContext<WorkflowStateProvider | undefined>('workflow-provider');

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === 'string'
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join('+');
    return `${index}:${typeKey}:${display?.class ?? ''}`;
  }
</script>

{#each workflow.visuals as visual (visual.id)}
  {#if 'Notification' in visual.display}
    <WorkflowNotification
        {workflow}
        variant={visual.display.Notification.variant}
        content={visual.display.Notification.content}
    />
  {/if}

  {#if 'Chat' in visual.display && provider?.customComponents?.Chat}
    {@const Chat = provider.customComponents.Chat}
    <Chat display={{ class: '', type: { Chat: visual.display.Chat } }} inputs={workflow.responses} />
  {/if}

  {#if 'Block' in visual.display}
    <div id={visual.id} class={cn('flex w-full flex-col', visual.display.Block.class)}>
      {#each visual.display.Block.content as display, index (displayKey(display, index))}
        <WorkflowDisplay from="body-block" {workflow} {display} inputs={workflow.responses} />
      {/each}
    </div>
  {/if}

  {#if 'Tile' in visual.display}
    <WorkflowBodyTile {visual} {workflow} {provider} />
  {/if}

  {#if 'Modal' in visual.display}
    <!-- Modals are usually handled by the shell/manager, but we can show inline for preview if needed -->
    <div class="p-4 border-2 border-primary rounded-xl my-4 bg-background shadow-lg">
        <div class="text-xs font-bold uppercase text-primary mb-2">Modal Content</div>
        {#each visual.display.Modal.content as display, index (displayKey(display, index))}
            <WorkflowDisplay from="body-modal" {workflow} {display} inputs={workflow.responses} />
        {/each}
    </div>
  {/if}
{/each}
