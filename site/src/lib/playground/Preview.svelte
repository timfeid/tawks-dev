<script lang="ts">
  import { WorkflowShell, type WorkflowStateProvider } from '@tawks/workflow-ui';
  import type { WorkflowResource } from '@tawks/workflow-types';

  let {
    resource,
    onAction,
  }: {
    resource: WorkflowResource;
    onAction: (actionId: string, inputs: Record<string, unknown>) => void;
  } = $props();

  const provider: WorkflowStateProvider = {
    get byInstance() {
      return { [resource.instance_id]: resource };
    },
    activeChildForParent: {},
    returnedToParent: {},
    playAnimations() {},
    async processAction(_instanceId: string, actionId: string, inputs: Record<string, unknown>) {
      onAction(actionId, inputs);
    },
  };
</script>

{#key resource.instance_id}
  <WorkflowShell workflow={resource} {provider} />
{/key}
