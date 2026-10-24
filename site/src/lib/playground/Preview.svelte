<script lang="ts">
  import type { Component } from 'svelte';
  import { WorkflowShell, type ActivityInfo, type WorkflowStateProvider } from '@tawks/workflow-ui';
  import type { WorkflowResource } from '@tawks/workflow-types';

  let {
    resource,
    onAction,
    findActivity,
    injected = [],
    customComponents,
  }: {
    resource: WorkflowResource;
    onAction: (actionId: string, inputs: Record<string, unknown>, instanceId: string) => void | Promise<void>;
    findActivity?: (workflowId: string) => Promise<ActivityInfo | null>;
    /** Workflows the game started inside this page, shown in the tiles they name. */
    injected?: WorkflowResource[];
    customComponents?: Record<string, Component<any>>;
  } = $props();

  function tileOf(wf: WorkflowResource): string | null {
    const inject = wf.inject as { Tile?: { name: string } } | null | undefined;
    return inject && typeof inject === 'object' && inject.Tile ? inject.Tile.name : null;
  }

  const provider: WorkflowStateProvider = {
    get byInstance() {
      return Object.fromEntries([resource, ...injected].map((wf) => [wf.instance_id, wf]));
    },
    activeChildForParent: {},
    returnedToParent: {},
    playAnimations() {},
    async processAction(instanceId: string, actionId: string, inputs: Record<string, unknown>) {
      await onAction(actionId, inputs, instanceId);
    },
    getActiveInjectedForTile(tileId: string) {
      const candidates = injected.filter((wf) => tileOf(wf) === tileId);
      return candidates.find((wf) => !wf.completed) ?? candidates.at(-1) ?? null;
    },
    get customComponents() {
      return customComponents;
    },
    findActivity: (workflowId: string) => findActivity?.(workflowId) ?? Promise.resolve(null),
  };
</script>

{#key resource.instance_id}
  <WorkflowShell workflow={resource} {provider} />
{/key}
