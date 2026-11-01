<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import * as look from '../../look';
  import WorkflowDisplay from '../workflow-display.svelte';
  import WorkflowNotification from './workflow-notification.svelte';
  import type { WorkflowStateProvider } from '../../workflow';
  import WorkflowBodyTile from './workflow-body-tile.svelte';
  import WorkflowShell from './workflow-shell.svelte';
  import { getContext, type Snippet } from 'svelte';
  import { needsPageTitle } from '../../page-title';
  import DockTabs from './dock-tabs.svelte';

  let { workflow, title, tabs = [] }: { workflow: WorkflowResource; title?: Snippet; tabs?: any[] } = $props();

  const provider = getContext<WorkflowStateProvider | undefined>('workflow-provider');

  // A bottom tab bar sits at the foot of the screen even when the page is short.
  function endsWithBottomTabs(content: { type: unknown }[]) {
    const last = content.at(-1)?.type;
    return !!last && typeof last === 'object' && 'TabBar' in last && (last as { TabBar: { style: string } }).TabBar.style === 'Bottom';
  }

  let titledPage = $derived(
    title ? workflow.visuals.find((visual) => visual && 'Page' in visual.display)?.id : undefined,
  );

  // The first thing drawn below any sticky bars carries the tabs on a wide screen.
  let firstMain = $derived(
    tabs.length
      ? workflow.visuals.find((visual) => visual && !('Sticky' in visual.display) && !('Modal' in visual.display) && !('Notification' in visual.display))?.id
      : undefined,
  );

  function isTitle(display: any) {
    const role = display?.type?.Typography?.role;
    return role === 'Title' || role === 'Display';
  }

  /** A row that opens with the title, such as a title and its main button. */
  function titleRow(display: any): any[] | null {
    const stack = display?.type?.Stack;
    return stack?.direction === 'Row' && isTitle(stack.content?.[0]) ? stack.content : null;
  }

  // Only the page at the foot of the screen leaves room for the dock; a page above a chat would leave a gap.
  function atFoot(index: number) {
    return !workflow.visuals.slice(index + 1).some((visual) => visual && ('Chat' in visual.display || 'Page' in visual.display || 'Block' in visual.display));
  }

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === 'string'
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join('+');
    return `${index}:${typeKey}`;
  }
</script>

{#each workflow.visuals as visual, index (visual.id)}
  {#if 'Notification' in visual.display}
    <WorkflowNotification
        {workflow}
        variant={visual.display.Notification.variant}
        content={visual.display.Notification.content}
    />
  {/if}

  {#if visual.id === firstMain && !('Page' in visual.display)}
    <div class={look.dock.above}>
      <div class={look.dock.top}><DockTabs {workflow} visuals={tabs} layout="wide" /></div>
    </div>
  {/if}

  {#if 'Chat' in visual.display && provider?.customComponents?.Chat}
    {@const Chat = provider.customComponents.Chat}
    <Chat display={{ type: { Chat: visual.display.Chat } }} inputs={workflow.responses} />
  {/if}

  {#if 'Block' in visual.display}
    <div id={visual.id} class="flex w-full flex-col">
      {#each visual.display.Block.content as display, index (displayKey(display, index))}
        <WorkflowDisplay from="body-block" {workflow} {display} inputs={workflow.responses} />
      {/each}
    </div>
  {/if}

  {#if 'Page' in visual.display}
    {@const injected = provider?.getActiveInjectedForTile?.(visual.id) ?? null}
    {#if injected && provider}
      {#key injected.instance_id}
        <WorkflowShell workflow={injected} {provider} />
      {/key}
    {:else}
      {@const hostTitle = !!title && visual.id === titledPage && needsPageTitle(visual.display.Page.content, workflow.responses)}
      {@const headed = visual.id === firstMain}
      {@const ownTitle = headed && !hostTitle && (isTitle(visual.display.Page.content[0]) || !!titleRow(visual.display.Page.content[0]))}
      <div
        id={visual.id}
        class={[
          look.page[visual.display.Page.width],
          atFoot(index) && look.pageEnd,
          look.backdrop[visual.display.Page.backdrop ?? 'Default'],
          look.typeface[visual.display.Page.typeface ?? 'Default'],
          (visual.display.Page.center || endsWithBottomTabs(visual.display.Page.content)) && 'min-h-full',
          visual.display.Page.center && 'justify-center',
        ]}
      >
        {#if headed}
          <div class={[hostTitle || ownTitle ? look.dock.head : look.dock.headBare, hostTitle && look.pageTitle]}>
            {#if hostTitle && title}
              <div class="min-w-0">{@render title()}</div>
            {:else if ownTitle}
              {@const group = titleRow(visual.display.Page.content[0])}
              <div class={group ? look.dock.titleGroup : 'min-w-0'}>
                {#each group ?? [visual.display.Page.content[0]] as display, index (displayKey(display, index))}
                  <WorkflowDisplay from="body-page" {workflow} {display} inputs={workflow.responses} />
                {/each}
              </div>
            {/if}
            <div class={look.dock.top}><DockTabs {workflow} visuals={tabs} layout="wide" /></div>
          </div>
        {:else if hostTitle && title}
          <div class={look.pageTitle}>{@render title()}</div>
        {/if}
        {#each ownTitle ? visual.display.Page.content.slice(1) : visual.display.Page.content as display, index (displayKey(display, index))}
          <WorkflowDisplay from="body-page" {workflow} {display} inputs={workflow.responses} />
        {/each}
      </div>
    {/if}
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
