<script lang="ts">
  import * as Popover from '@tawks/ui/popover/index.js';
  import { toast } from 'svelte-sonner';
  import { cn } from '../../utils.js';
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

  let triggerRef = $state<HTMLButtonElement>(null!);

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
    if (provider.processAction) {
        await provider.processAction(workflow.instance_id, action, {});
    }
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

    if (popover.closed_server_action_id) {
      await submitServerAction(popover.closed_server_action_id);
    }
  }
</script>

{#if popover}
  <Popover.Root bind:open onOpenChange={openChanged}>
    <Popover.Trigger
      class={display.class}
      bind:ref={triggerRef}
      onclick={(e) => {
        // Prevent the Popover from toggling closed if it's already open
        if (open) {
          e.preventDefault();
        }
      }}
    >
      {#each popover.trigger as childDisplay}
        <WorkflowDisplay {workflow} display={childDisplay} {inputs} />
      {/each}
    </Popover.Trigger>

    <Popover.Content
      class={cn('w-(--bits-floating-anchor-width) overflow-hidden p-0', popover.content_class)}
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