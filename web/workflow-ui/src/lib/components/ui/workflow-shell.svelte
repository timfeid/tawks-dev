<script lang="ts" module>
  export type { WorkflowStateProvider } from '../../workflow';
</script>

<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { onMount, type Snippet, setContext, getContext } from 'svelte';

  import WorkflowBody from './workflow-body.svelte';
  import WorkflowFooter from './workflow-footer.svelte';
  import WorkflowDock from './workflow-dock.svelte';
  import DockTabs from './dock-tabs.svelte';
  import ContextBridge from './context-bridge.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let {
    workflow,
    titlebar,
    provider,
    onTitlebarUpdate,
    dockHost,
    tabsHost,
    header,
    followChildren = true,
    title,
    visuals,
  }: {
    workflow: WorkflowResource;
    titlebar?: Snippet;
    provider: WorkflowStateProvider;
    onTitlebarUpdate?: (snippet: Snippet) => void;
    /** Where the host floats the page's tabs and buttons; without one they sit at the foot of the shell. */
    dockHost?: { show: (dock: Snippet) => void; hide: (dock: Snippet) => void };
    /** Where the host draws the page's tabs, such as its own header row; without one they get a row above the page. */
    tabsHost?: { show: (tabs: Snippet) => void; hide: (tabs: Snippet) => void };
    /** The host's header row, drawn above the first view unless that view is a chat, whose own header carries it. */
    header?: Snippet;
    followChildren?: boolean;
    /** The host's name for the page, shown above it unless the page opens with its own title. */
    title?: Snippet;
    /** Drawn in place of the page's own, while it is edited by hand. */
    visuals?: WorkflowResource['visuals'];
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
  let viewWorkflow = $derived.by(() => {
    const shown = provider?.byInstance?.[instanceId] ?? workflow;
    return visuals && shown.instance_id === workflow.instance_id ? { ...shown, visuals } : shown;
  });

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

  let hasDock = $derived(toolbarVisuals.length > 0);
  let hasTabs = $derived(tabVisuals.length > 0);

  $effect(() => {
    if (!dockHost || !hasDock) return;
    const host = dockHost;
    host.show(floatingDock);
    return () => host.hide(floatingDock);
  });

  $effect(() => {
    if (!tabsHost || !hasTabs) return;
    const host = tabsHost;
    host.show(hostedTabs);
    return () => host.hide(hostedTabs);
  });
</script>

{#snippet hostedTabs()}
  <ContextBridge {provider}>
    <DockTabs workflow={viewWorkflow} visuals={tabVisuals} />
  </ContextBridge>
{/snippet}

{#snippet floatingDock()}
  <ContextBridge {provider}>
    <WorkflowDock workflow={viewWorkflow} actions={toolbarVisuals} />
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
        <WorkflowBody workflow={viewWorkflow} {title} header={tabsHost ? header : undefined} tabs={tabsHost && !header ? [] : tabVisuals} />
      {/if}
    </div>

    <WorkflowFooter
      workflow={viewWorkflow}
      visuals={stickyFooterVisuals}
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
