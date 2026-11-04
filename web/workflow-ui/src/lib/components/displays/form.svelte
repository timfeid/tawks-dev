<script lang="ts">
  import { submitServerAction } from '../workflow';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { getContext, onDestroy, setContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, display, inputs }: Props = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  // Values typed into this form's inputs, keyed by input id. Inputs reset
  // themselves when `version` moves, which happens after each submit.
  let values: Record<string, any> = {};
  const formState = $state<{ version: number; scope: Element | null }>({ version: 0, scope: null });
  setContext('form-state', formState);
  const closeDialog = getContext<(() => void) | undefined>('dialog-close');
  setContext(
    'form-submit',
    (sent: { action: string; inputs: Record<string, any> } | null, from: Element | null) => submitAll(sent, from),
  );

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
  // always report changes up through props. A list repeats an input's id per
  // row, and then the one that counts is the one in the pressed button's row.
  function domValues(from: Element | null) {
    const out: Record<string, any> = {};
    const repeated = new Set<string>();
    const seen = new Map<string, HTMLElement[]>();
    for (const wrapper of formEl?.querySelectorAll<HTMLElement>('[data-input-id]') ?? []) {
      const id = wrapper.dataset.inputId;
      if (!id) continue;
      seen.set(id, [...(seen.get(id) ?? []), wrapper]);
    }
    const row = rowOf(from);
    for (const [id, wrappers] of seen) {
      let wrapper = wrappers[0];
      if (wrappers.length > 1) {
        repeated.add(id);
        const inRow = row && wrappers.find((candidate) => row.contains(candidate));
        wrapper = inRow || wrappers[wrappers.length - 1];
      }
      const field = wrapper.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>('input, textarea, select');
      if (!field) continue;
      if (field instanceof HTMLInputElement && field.type === 'number') {
        if (field.value !== '') out[id] = Number(field.value);
      } else {
        out[id] = field instanceof HTMLInputElement && field.type === 'checkbox' ? field.checked : field.value;
      }
    }
    return { out, repeated };
  }

  /** The list row a button sits in, when the form runs across a list. */
  function rowOf(from: Element | null): Element | null {
    const row = from?.closest('[data-workflow-row]') ?? null;
    return row && formEl?.contains(row) ? row : null;
  }

  let sending = false;

  async function submitAll(sent: { action: string; inputs: Record<string, any> } | null = null, from: Element | null = null) {
    if (!form?.server_action_id || sending) return;
    const { out: typed, repeated } = domValues(from);
    const kept = { ...values };
    for (const id of repeated) delete kept[id];
    const fields = { ...kept, ...typed };
    // The button's own inputs first: a dotted form field then lands inside an
    // object it carries, and a blank field never hides a value it carries.
    const payload: Record<string, any> = { ...(sent?.inputs ?? {}) };
    for (const [key, value] of Object.entries(fields)) {
      if (value === '' && key in payload) continue;
      payload[key] = value;
    }
    // Read now: a live update can take the row off the list before the press returns.
    const pressedRow = rowOf(from);
    sending = true;
    try {
      const went = await submitServerAction(provider, workflow.instance_id, payload, sent?.action || form.server_action_id);
      // A refusal leaves what they typed, to fix and send again.
      if (went !== false) {
        clearTyped(pressedRow);
        closeDialog?.();
      }
    } catch (e) {
      console.error('Submission failed', e);
    } finally {
      sending = false;
    }
  }

  // Back to each input's starting value, which is blank unless it has a default.
  // Only the pressed row's: what someone is typing in another row stays theirs.
  function clearTyped(row: Element | null) {
    const scope = row ?? formEl;
    for (const field of scope?.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>(
      '[data-input-id] input[type="text"], [data-input-id] input[type="number"], [data-input-id] textarea',
    ) ?? []) {
      field.value = '';
      field.dispatchEvent(new Event('input', { bubbles: true }));
    }
    for (const field of scope?.querySelectorAll('[data-input-id] input[data-input-clears]') ?? []) {
      field.dispatchEvent(new Event('input', { bubbles: true }));
    }
    values = {};
    formState.scope = row;
    formState.version += 1;
  }

  // Enter belongs to the row it was typed in, not the form's first button.
  function onkeydown(event: KeyboardEvent) {
    const target = event.target as Element | null;
    if (event.key !== 'Enter' || !(target instanceof HTMLInputElement)) return;
    const row = target.closest('[data-workflow-row]');
    const button = row?.querySelector<HTMLButtonElement>('button[type="submit"]');
    if (!button) return;
    event.preventDefault();
    button.click();
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

  function cancelPending() {
    if (pendingTimeout) clearTimeout(pendingTimeout);
    pendingTimeout = null;
  }

  let seenNode: string | undefined;
  $effect(() => {
    const node = workflow.current_node_id;
    if (seenNode !== undefined && node !== seenNode) {
      cancelPending();
      values = {};
      lastSubmittedValues = new Map();
    }
    seenNode = node;
  });

  onDestroy(cancelPending);
</script>

{#if form}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <form
    bind:this={formEl}
    {onkeydown}
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
