<script lang="ts">
  import type { WorkflowResource, WorkflowDisplay as WorkflowDisplayI, NotificationVariant } from '@tawks/workflow-types';
  import { cn } from '../../utils';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { 
    workflow, 
    variant, 
    content 
  }: { 
    workflow: WorkflowResource; 
    variant: NotificationVariant; 
    content: WorkflowDisplayI[];
  } = $props();

  const variantClasses = {
    Succes: 'bg-emerald-50 border-emerald-200 text-emerald-800',
    Info: 'bg-blue-50 border-blue-200 text-blue-800',
    Warning: 'bg-amber-50 border-amber-200 text-amber-800',
    Error: 'bg-destructive/10 border-destructive/20 text-destructive',
  };
</script>

<div class={cn('p-4 border rounded-lg mb-4 flex flex-col gap-2', variantClasses[variant])}>
  {#each content as display}
    <WorkflowDisplay 
        from="notification" 
        {workflow} 
        {display} 
        inputs={workflow.responses} 
    />
  {/each}
</div>
