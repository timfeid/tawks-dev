<script lang="ts">
  import { resolveInputValue } from '../../utils';
  import type { JsonValue } from '@tawks/workflow-types';
  import Button from '@tawks/ui/button/button.svelte';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { submitServerAction } from '../workflow';
  import { getContext, untrack } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  const PENDING_TIMEOUT_MS = 15_000;

  let { workflow, display, inputs, buttonClicked }: Props = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let pending = $state(false);
  let pendingTimer: ReturnType<typeof setTimeout> | null = null;

  function clearPending() {
    if (pendingTimer !== null) {
      clearTimeout(pendingTimer);
      pendingTimer = null;
    }
    pending = false;
  }

  function startPending() {
    if (pendingTimer !== null) clearTimeout(pendingTimer);
    pending = true;
    pendingTimer = setTimeout(clearPending, PENDING_TIMEOUT_MS);
  }

  $effect(() => {
    workflow;
    untrack(clearPending);
  });

  $effect(() => () => {
    if (pendingTimer !== null) clearTimeout(pendingTimer);
  });

  // A `scroll-to-<name>` class token on the button scrolls the nearest
  // scrollable ancestor so the element carrying `scroll-target-<name>` sits
  // at its top, where a sticky header would rest.
  function scrollToTarget(button: HTMLElement | null) {
    const name = /(?:^|\s)scroll-to-([\w-]+)/.exec(display.class ?? '')?.[1];
    if (!name || !button) return;
    const target = document.querySelector<HTMLElement>(`.scroll-target-${name}`);
    if (!target) return;
    let container = target.parentElement;
    while (container) {
      const overflowY = getComputedStyle(container).overflowY;
      if ((overflowY === 'auto' || overflowY === 'scroll') && container.scrollHeight > container.clientHeight) break;
      container = container.parentElement;
    }
    if (!container) {
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
      return;
    }
    const offset = target.getBoundingClientRect().top - container.getBoundingClientRect().top;
    container.scrollTo({ top: container.scrollTop + offset, behavior: 'smooth' });
  }

  const formSubmit = getContext<(() => void) | undefined>('form-submit');

  function onclick(e: MouseEvent) {
    if (pending) return;
    if (typeof display.type !== 'string' && 'Button' in display.type) {
      if (display.type.Button.type === 'submit' && formSubmit) {
        // The enclosing form sends the typed values along with this action.
        e.preventDefault();
        formSubmit();
        return;
      }
      const requestInputs: Record<string, JsonValue> = {};
      if (display.type.Button.input_map) {
        for (const [key, val] of Object.entries(
          display.type.Button.input_map,
        )) {
          requestInputs[key] = resolveInputValue(inputs, val);
        }
      }
      startPending();
      submitServerAction(
        provider,
        workflow.instance_id,
        requestInputs,
        display.type.Button.action_id,
      );
      scrollToTarget(e.currentTarget as HTMLElement | null);
      buttonClicked?.();
    }
  }
</script>

{#if typeof display.type !== 'string' && 'Button' in display.type}
  <Button
    type={display.type.Button.type}
    size={display.type.Button.size as 'default'}
    class={[display.class, pending && 'opacity-60 cursor-progress']}
    disabled={pending}
    aria-busy={pending}
    {onclick}
    variant={display.type.Button.variant as 'default'}
  >
    {#each display.type.Button.content as displayItem}
      <WorkflowDisplay
        from="button"
        {inputs}
        {workflow}
        display={displayItem}
      />
    {/each}
  </Button>
{/if}
