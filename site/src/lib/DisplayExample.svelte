<script lang="ts">
  import { ExternalLink } from '@lucide/svelte';
  import type { WorkflowNode, WorkflowResource } from '@tawks/workflow-types';
  import Preview from './playground/Preview.svelte';
  import { exampleDefinition, type DisplayExample } from './examples';

  let { example, playground }: { example: DisplayExample; playground: string } = $props();

  const uid = Math.random().toString(36).slice(2);
  const definition = exampleDefinition(example, `example-${uid}`);
  // Buttons and forms in an example only fill in their inputs here; there is
  // no bot behind them. The playground link runs them against mocks.
  let responses = $state<Record<string, unknown>>({ ...(definition.responses as Record<string, unknown>) });

  const resource = $derived.by((): WorkflowResource => {
    const node = definition.nodes.home as WorkflowNode;
    return {
      inject: null,
      instance_id: `example-${uid}`,
      workflow_id: 'wf-example',
      stale_definition: false,
      name: definition.name,
      description: null,
      responses: responses as WorkflowResource['responses'],
      completed: false,
      status: 'active',
      complete_message: null,
      visuals: node.visuals,
      layout: null,
      user_id: 'you',
      current_node_id: 'home',
      waiting: false,
      attached_workflows: {},
      parent: null,
      created_at: '',
      updated_at: '',
      completed_at: null,
    } as WorkflowResource;
  });

  function onAction(_actionId: string, inputs: Record<string, unknown>) {
    responses = { ...responses, ...inputs };
  }
</script>

<figure class="my-6 overflow-hidden rounded-lg border bg-background">
  <div class="min-h-16">
    <Preview {resource} {onAction} />
  </div>
  <figcaption class="flex items-center justify-end border-t bg-card px-3 py-1.5 text-xs">
    <a class="flex items-center gap-1 text-muted-foreground hover:text-foreground" href={playground}>
      Open in playground <ExternalLink class="size-3" />
    </a>
  </figcaption>
</figure>
