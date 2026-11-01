<script lang="ts">
  import type { Component } from "svelte";
  import type { WorkflowStateProvider } from "../workflow";
  let {
    component: Custom,
    componentProps,
    provider,
    type,
  }: {
    component: Component<any>;
    componentProps: Record<string, any>;
    provider?: WorkflowStateProvider;
    type: string;
  } = $props();
  // The host owns activity policy; the guard follows the rendered component's lifetime.
  $effect(() => provider?.holdWhileDisplayed?.(type));
</script>

<Custom {...componentProps} />
