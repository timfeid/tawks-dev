<script lang="ts">
  import { WorkflowShell, type ActivityInfo, type WorkflowStateProvider } from '@tawks/workflow-ui';
  import type { WorkflowResource } from '@tawks/workflow-types';

  let {
    resource,
    onAction,
    findActivity,
  }: {
    resource: WorkflowResource;
    onAction: (actionId: string, inputs: Record<string, unknown>) => void | Promise<void>;
    findActivity?: (workflowId: string) => Promise<ActivityInfo | null>;
  } = $props();

  const provider: WorkflowStateProvider = {
    get byInstance() {
      return { [resource.instance_id]: resource };
    },
    activeChildForParent: {},
    returnedToParent: {},
    playAnimations() {},
    async processAction(_instanceId: string, actionId: string, inputs: Record<string, unknown>) {
      await onAction(actionId, inputs);
    },
    findActivity: (workflowId: string) => findActivity?.(workflowId) ?? Promise.resolve(null),
  };
</script>

{#key resource.instance_id}
  <WorkflowShell workflow={resource} {provider} />
{/key}
