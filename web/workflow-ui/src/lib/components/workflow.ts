import { interpolate, setNestedValue } from '../utils';
import type { JsonValue } from '@tawks/workflow-types';
import type { WorkflowStateProvider } from '../workflow';

export const workflowInputs = new Map<string, Record<string, JsonValue>>();

export async function submitServerAction(
  provider: WorkflowStateProvider,
  instanceId: string,
  updates: Record<string, JsonValue>,
  actionId: string | null,
) {
  if (!actionId) {
    console.warn('submitServerAction called without actionId');
    return;
  }

  if (!provider?.processAction) {
    console.error('submitServerAction called without a valid provider');
    return;
  }

  const updatedInputs = workflowInputs.get(instanceId) || {};
  for (const [key, val] of Object.entries(updates)) {
    setNestedValue(updatedInputs, key, val);
  }

  const payload: Record<string, JsonValue> = {};
  for (const [key, val] of Object.entries(updatedInputs)) {
    setNestedValue(payload, key, val);
  }

  return provider.processAction(instanceId, actionId, payload);
}

export async function onChange(
  provider: WorkflowStateProvider,
  instanceId: string,
  inputs: Record<string, JsonValue>,
  updates: Record<string, JsonValue>,
  onChangeAction: [string, Record<string, JsonValue>] | null,
) {
  if (!onChangeAction) {
    return;
  }
  const [id, v] = onChangeAction;
  for (const [key, val] of Object.entries(v)) {
    updates[key] = interpolate(inputs, typeof val === 'string' ? val : String(val || ''));
  }

  const updatesToSend: Record<string, JsonValue> = {};
  const updated = { ...inputs };
  const keptValues = new Set<string>();
  for (const key of Object.keys(updates)) {
    if (key.includes('.')) {
      setNestedValue(updated, key, updates[key]);
      keptValues.add(key.split('.')[0]);
    } else {
      updatesToSend[key] = updates[key];
    }
  }

  for (const val of keptValues.values()) {
    updatesToSend[val] = inputs[val];
  }

  return submitServerAction(provider, instanceId, updatesToSend, id);
}