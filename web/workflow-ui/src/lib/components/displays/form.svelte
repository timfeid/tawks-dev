<script lang="ts">
  import { submitServerAction } from '../workflow';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext, setContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  // Values typed into this form's inputs, keyed by input id. Inputs reset
  // themselves when `version` moves, which happens after each submit.
  let values: Record<string, any> = {};
  const formState = $state({ version: 0 });
  setContext('form-state', formState);
  setContext('form-submit', () => submitAll());

  let form = $derived.by(() => {
    if (typeof display.type === 'string' || !('Form' in display.type)) return;
    return display.type.Form;
  });

  let pendingTimeout: ReturnType<typeof setTimeout> | null = null;
  // Use a map to track values per-key to prevent cross-talk between different inputs
  let lastSubmittedValues = new Map<string, any>();

  async function submit(key: string, value: any) {
    if (!form?.server_action_id) return;

    lastSubmittedValues.set(key, value);

    try {
      await submitServerAction(provider, workflow.instance_id, { [key]: value }, form.server_action_id);
    } catch (e) {
      console.error('Submission failed', e);
    }
  }

  let formEl = $state<HTMLFormElement | null>(null);

  // Typed values as the DOM has them; inputs nested in other displays do not
  // always report changes up through props.
  function domValues(): Record<string, any> {
    const out: Record<string, any> = {};
    for (const wrapper of formEl?.querySelectorAll<HTMLElement>('[data-input-id]') ?? []) {
      const id = wrapper.dataset.inputId;
      const field = wrapper.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>('input, textarea, select');
      if (!id || !field) continue;
      out[id] = field instanceof HTMLInputElement && field.type === 'checkbox' ? field.checked : field.value;
    }
    return out;
  }

  async function submitAll() {
    if (!form?.server_action_id) return;
    const payload = { ...domValues(), ...values };
    values = {};
    formState.version += 1;
    for (const field of formEl?.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
      '[data-input-id] input[type="text"], [data-input-id] textarea',
    ) ?? []) {
      field.value = '';
      field.dispatchEvent(new Event('input', { bubbles: true }));
    }
    try {
      await submitServerAction(provider, workflow.instance_id, payload, form.server_action_id);
    } catch (e) {
      console.error('Submission failed', e);
    }
  }

  function inputsChanged(key: string, value: any) {
    values[key] = value;
    if (form?.submit_when !== 'OnChange') return;

    // 1. Immediate local check: don't even start a timer if the value hasn't actually changed
    if (lastSubmittedValues.get(key) === value) return;

    // 2. Clear previous timer to "restart" the debounce window
    if (pendingTimeout) {
      clearTimeout(pendingTimeout);
    }

    // 3. Set a new timer. The 350ms window only starts AFTER the last keystroke.
    pendingTimeout = setTimeout(() => {
      submit(key, value);
      pendingTimeout = null;
    }, 350);
  }
</script>

{#if form}
  <form
    bind:this={formEl}
    class={display.class}
    onsubmit={(e) => {
      e.preventDefault();
      if (form?.submit_when === 'OnSubmit') submitAll();
    }}
  >
    {#each form.content as content}
      <!-- z{inputsChanged}
      bro -->
      <WorkflowDisplay from="wtf" {inputsChanged} {inputs} {workflow} display={content} />
    {/each}
  </form>
{/if}
