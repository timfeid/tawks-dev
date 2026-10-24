<script lang="ts" module>
  export type { WorkflowStateProvider } from '../../workflow';
</script>

<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { onMount, type Snippet, setContext, getContext } from 'svelte';

  import WorkflowDisplay from '../workflow-display.svelte';
  import WorkflowHeader from './workflow-header.svelte';
  import WorkflowBody from './workflow-body.svelte';
  import WorkflowFooter from './workflow-footer.svelte';
  import ContextBridge from './context-bridge.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let {
    workflow,
    titlebar,
    provider,
    onTitlebarUpdate,
    followChildren = true,
  }: {
    workflow: WorkflowResource;
    titlebar?: Snippet;
    provider: WorkflowStateProvider;
    onTitlebarUpdate?: (snippet: Snippet) => void;
    followChildren?: boolean;
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

  let stickyHeaderVisuals = $derived(
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

  function visualStructureKey(visuals: WorkflowResource["visuals"] = []) {
    return visuals
      .map((visual, index) => {
        const displayType =
          visual && typeof visual === "object"
            ? Object.keys(visual.display ?? {})
                .sort()
                .join("+")
            : "unknown";
        let contentShape = "";
        if (visual && typeof visual === "object") {
          const display: any = visual.display;
          if ("Block" in display) {
            contentShape = `${display.Block.content?.length ?? 0}`;
          } else if ("Sticky" in display) {
            contentShape = `${display.Sticky.position}:${display.Sticky.content?.length ?? 0}`;
          } else if ("Tile" in display) {
            contentShape = `${display.Tile?.content?.length ?? 0}`;
          } else if ("Modal" in display) {
            contentShape = `${display.Modal?.content?.length ?? 0}`;
          }
        }
        return `${index}:${visual?.id ?? ""}:${displayType}:${contentShape}`;
      })
      .join("|");
  }

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === "string"
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join("+");
    return `${index}:${typeKey}:${display?.class ?? ""}`;
  }

  let workflowRenderKey = $derived.by(
    () =>
      `${viewWorkflow?.instance_id ?? ""}:${viewWorkflow?.current_node_id ?? ""}:${visualStructureKey(viewWorkflow?.visuals || [])}`,
  );
</script>

{#snippet finalTitlebar()}
  <ContextBridge {provider}>
    <!-- The page's own buttons sit at the right, just before the channel's bell. -->
    {#if toolbarVisuals.length}
      <div class="order-last ml-auto flex shrink-0 items-center gap-1">
        {#each toolbarVisuals as visual (visual.id)}
          {@const sticky = (visual.display as any).Sticky}
          {#each sticky.content as display, index (displayKey(display, index))}
            <WorkflowDisplay
              from="shell-titlebar"
              workflow={viewWorkflow}
              {display}
              inputs={viewWorkflow.responses}
            />
          {/each}
        {/each}
      </div>
    {/if}
    {#if titlebar}
      {@render titlebar()}
    {/if}
  </ContextBridge>
{/snippet}

{#if viewWorkflow}
  {#key workflowRenderKey}
    <WorkflowHeader workflow={viewWorkflow} visuals={stickyHeaderVisuals} />

    <div class="workflow-shell-body-container flex-grow">
      {#if replacing}
        <WorkflowBody workflow={{ ...viewWorkflow, visuals: [] }} />
      {:else}
        <WorkflowBody workflow={viewWorkflow} />
      {/if}
    </div>

    <WorkflowFooter workflow={viewWorkflow} visuals={stickyFooterVisuals} />
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
