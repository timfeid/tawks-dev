<script lang="ts" module>
  export type { WorkflowStateProvider } from '../../workflow';
</script>

<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { onMount, type Snippet, setContext, getContext } from 'svelte';

  import WorkflowBody from './workflow-body.svelte';
  import WorkflowFooter from './workflow-footer.svelte';
  import WorkflowDock from './workflow-dock.svelte';
  import ContextBridge from './context-bridge.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let {
    workflow,
    titlebar,
    provider,
    onTitlebarUpdate,
    dockHost,
    followChildren = true,
    title,
  }: {
    workflow: WorkflowResource;
    titlebar?: Snippet;
    provider: WorkflowStateProvider;
    onTitlebarUpdate?: (snippet: Snippet) => void;
    /** Where the host floats the page's tabs and buttons; without one they sit at the foot of the shell. */
    dockHost?: { show: (dock: Snippet) => void; hide: (dock: Snippet) => void };
    followChildren?: boolean;
    /** The host's name for the page, shown above it unless the page opens with its own title. */
    title?: Snippet;
  } = $props();

  setContext('workflow-provider', provider);

  // current pointer
  let instanceId = $state(workflow.instance_id);

  onMount(() => {
    provider?.playAnimations(workflow);
    if (onTitlebarUpdate) {
      onTitlebarUpdate(finalTitlebar);
    }
  });

  // ✅ Derived from repo (fallback only if repo doesn't have it yet)
  let viewWorkflow = $derived(provider?.byInstance?.[instanceId] ?? workflow);

  // ✅ Replace handling: switch pointer only
  let replacing = $state(false);

  $effect(() => {
    if (!followChildren) return;
    const child = provider?.activeChildForParent?.[instanceId];
    if (child && child !== instanceId) {
      replacing = true;
      instanceId = child;
    } else {
      replacing = false;
    }
  });

  $effect(() => {
    const from = instanceId;
    const to = provider?.returnedToParent?.[from];
    if (to && to !== from) {
      instanceId = to;
    }
  });

  let toolbarVisuals = $derived(
    (viewWorkflow?.visuals || []).filter(
      (visual) =>
        visual &&
        'Sticky' in visual.display &&
        visual.display.Sticky.position === 'Toolbar',
    ),
  );

  let tabVisuals = $derived(
    (viewWorkflow?.visuals || []).filter(
      (visual) =>
        visual &&
        'Sticky' in visual.display &&
        visual.display.Sticky.position === 'Top',
    ),
  );

  let stickyFooterVisuals = $derived(
    (viewWorkflow?.visuals || []).filter(
      (visual) =>
        visual &&
        'Sticky' in visual.display &&
        visual.display.Sticky.position === 'Bottom',
    ),
  );

  let mainVisuals = $derived(
    (viewWorkflow?.visuals || []).filter(
      (visual) =>
        visual && !('Sticky' in visual.display) && !('Modal' in visual.display),
    ),
  );

  let hasDock = $derived(tabVisuals.length > 0 || toolbarVisuals.length > 0);

  $effect(() => {
    if (!dockHost || !hasDock) return;
    const host = dockHost;
    host.show(floatingDock);
    return () => host.hide(floatingDock);
  });
</script>

{#snippet floatingDock()}
  <ContextBridge {provider}>
    <WorkflowDock workflow={viewWorkflow} tabs={tabVisuals} actions={toolbarVisuals} />
  </ContextBridge>
{/snippet}

<!-- A page's own tabs and buttons go in its dock; the title bar keeps only
     what the host page adds. -->
{#snippet finalTitlebar()}
  <ContextBridge {provider}>
    {#if titlebar}
      {@render titlebar()}
    {/if}
  </ContextBridge>
{/snippet}

{#if viewWorkflow}
  {#key viewWorkflow.instance_id}
    <div class="workflow-shell-body-container flex-grow">
      {#if replacing}
        <WorkflowBody workflow={{ ...viewWorkflow, visuals: [] }} />
      {:else}
        <WorkflowBody workflow={viewWorkflow} {title} tabs={tabVisuals} />
      {/if}
    </div>

    <WorkflowFooter
      workflow={viewWorkflow}
      visuals={stickyFooterVisuals}
      tabs={dockHost ? [] : tabVisuals}
      actions={dockHost ? [] : toolbarVisuals}
    />
  {/key}
{/if}

<style>
  .workflow-shell-body-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
