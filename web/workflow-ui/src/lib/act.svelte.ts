import type { Act, JsonValue, WorkflowResource } from '@tawks/workflow-types';
import { getContext, untrack } from 'svelte';
import { evaluateCondition, resolveInputValue } from './utils';
import { submitServerAction } from './components/workflow';
import type { WorkflowStateProvider } from './workflow';

const PENDING_TIMEOUT_MS = 15_000;

type Sent = { action: string; inputs: Record<string, JsonValue> };

/**
 * A choice such as a tab tapped while the previous tap is still on its way,
 * per workflow instance. It outlives the tab bar that took it, since the
 * update for the earlier tap can swap the whole page and its tabs.
 */
const queuedChoice = new Map<string, Sent>();

/**
 * Runs an `Act` the way every pressable block does: resolves its inputs
 * against what is in scope, sends it, and stays pending until the workflow
 * next changes or a timeout passes.
 */
export function pressable(get: () => { workflow: WorkflowResource; inputs: Record<string, unknown> }) {
  const provider = getContext<WorkflowStateProvider>('workflow-provider');
  let pending = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function clear() {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    pending = false;
    const id = get().workflow.instance_id;
    const next = queuedChoice.get(id);
    if (next) {
      queuedChoice.delete(id);
      send(next);
    }
  }

  $effect(() => {
    get().workflow;
    untrack(clear);
  });
  $effect(() => () => {
    if (timer !== null) clearTimeout(timer);
  });

  function resolve(act: Act | null | undefined): Sent | null {
    if (!act) return null;
    const { inputs } = get();
    if (act.disabled && evaluateCondition(act.disabled, inputs)) return null;
    const resolved: Record<string, JsonValue> = {};
    for (const [key, value] of Object.entries(act.inputs ?? {})) {
      resolved[key] = resolveInputValue(inputs, value as JsonValue);
    }
    return { action: act.action, inputs: resolved };
  }

  function send(sent: Sent) {
    const { workflow } = get();
    showAtOnce(workflow, sent.inputs);
    pending = true;
    timer = setTimeout(clear, PENDING_TIMEOUT_MS);
    submitServerAction(provider, workflow.instance_id, sent.inputs, sent.action);
  }

  return {
    get pending() {
      return pending;
    },
    disabled(act: Act | null | undefined) {
      return !!act?.disabled && evaluateCondition(act.disabled, get().inputs);
    },
    run(act: Act | null | undefined) {
      if (pending) return;
      const sent = resolve(act);
      if (sent) send(sent);
    },
    /**
     * For a choice where only the last tap counts, such as a tab: a tap while
     * the previous one is on its way is sent once that one lands, instead of
     * being dropped and leaving the page on the earlier choice. A tap on the
     * choice already showing (`null`) cancels a queued one.
     */
    runLatest(act: Act | null | undefined) {
      const id = get().workflow.instance_id;
      const sent = resolve(act);
      if (!pending) {
        queuedChoice.delete(id);
        if (sent) send(sent);
      } else if (sent) queuedChoice.set(id, sent);
      else queuedChoice.delete(id);
    },
  };
}

/**
 * The server stores an act's inputs as responses before it runs the action,
 * so they are shown right away: a tab or a picker switches without waiting.
 * The server's update replaces them either way.
 */
function showAtOnce(workflow: WorkflowResource, inputs: Record<string, JsonValue>) {
  const responses = workflow.responses as Record<string, JsonValue> | undefined;
  if (!responses) return;
  for (const [key, value] of Object.entries(inputs)) {
    if (!key.includes('.') && key !== '__proto__' && key !== 'constructor' && key !== 'prototype') {
      responses[key] = value;
    }
  }
}

/** Scrolls the nearest scrolling ancestor of the element with `id`, or of the
 * display marked `id`, so it sits at the top. */
export function scrollToId(id: string) {
  const target = document.getElementById(id) ?? document.querySelector(`[data-mark~="${CSS.escape(id)}"]`);
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
