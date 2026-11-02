<script lang="ts">
  import * as Popover from '@tawks/ui/popover/index.js';
  import { toast } from 'svelte-sonner';
  import { cn } from '../../utils.js';
  import * as look from '../../look';
  import WorkflowBody from '../ui/workflow-body.svelte';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext, tick } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let hasFocusWithin = $state(false);
  let open = $state(false);

  let popover = $derived.by(() => {
    if (typeof display.type === 'string') return;
    if ('Popover' in display.type) return display.type.Popover;
  });

  let triggerRef = $state<HTMLElement>(null!);

  // Action to listen for the bubbling close event
  function closeListener(node: HTMLElement) {
    const handler = () => {
      if (open) {
        openChanged(false);
      }
      open = false;

      // Optional: Refocus trigger for better keyboard UX
      tick().then(() => {
        triggerRef?.focus();
      });
    };
    node.addEventListener('close-popover', handler);
    return {
      destroy() {
        node.removeEventListener('close-popover', handler);
      },
    };
  }

  async function submitServerAction(action: string | null) {
    if (!action) {
      toast.error('no action');
      return;
    }
    // Opening and closing are best effort: a race with the search on its way
    // only loses a refresh, and the next keystroke makes up for it.
    await provider.processAction?.(workflow.instance_id, action, {}).catch((error) => console.warn(error));
  }

  /** Through its input event, so the form forgets the last search and the same words can search again. */
  function clearField() {
    const field = triggerRef?.querySelector('input');
    if (!field?.value) return;
    field.value = '';
    field.dispatchEvent(new Event('input', { bubbles: true }));
  }

  async function openChanged(v: boolean) {
    if (!popover) return;

    if (!v && hasFocusWithin) {
      open = true;
      return;
    }

    if (v) {
      if (popover.opened_server_action_id) {
        await submitServerAction(popover.opened_server_action_id);
      }
      return;
    }

    clearField();
    if (popover.closed_server_action_id) {
      await submitServerAction(popover.closed_server_action_id);
    }
  }
</script>

{#if popover}
  <Popover.Root bind:open onOpenChange={openChanged}>
    <Popover.Trigger
      bind:ref={triggerRef}
      onclick={(e) => {
        // Prevent the Popover from toggling closed if it's already open
        if (open) {
          e.preventDefault();
        }
      }}
    >
      {#snippet child({ props })}
        <!-- A div, because a field inside a button can't be focused in every browser. -->
        {@const { role: _role, tabindex: _tabindex, onkeydown: _onkeydown, type: _type, ...trigger } = props}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          {...trigger}
          class={popover?.width === 'Fill' ? 'min-w-0 flex-1' : ''}
          oninput={(event) => {
            // Typing into a field that took focus without a click still shows its results.
            if (!open && (event.target as HTMLInputElement).value) {
              open = true;
              openChanged(true);
            }
          }}
        >
          {#each popover.trigger as childDisplay}
            <WorkflowDisplay {workflow} display={childDisplay} {inputs} />
          {/each}
        </div>
      {/snippet}
    </Popover.Trigger>

    <Popover.Content
      class={cn(look.popoverWidth[popover.width ?? 'Medium'], 'overflow-hidden p-0')}
      trapFocus={false}
      sideOffset={7}
      align="center"
      onOpenAutoFocus={(e) => e.preventDefault()}
      onCloseAutoFocus={(e) => e.preventDefault()}
    >
      <div class="contents" use:closeListener>
        <WorkflowBody workflow={{ ...workflow, visuals: popover.content }}></WorkflowBody>
      </div>
    </Popover.Content>
  </Popover.Root>
{/if}