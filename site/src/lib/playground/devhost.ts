import type { CreateWorkflowDefinition } from '@tawks/workflow-types';
import type { Json, ServerActions } from './runtime';

export const DEFAULT_DEVHOST = 'http://localhost:8787';

export type DevhostWorkflow = { id: string; definition: CreateWorkflowDefinition };

function base(url: string) {
  return url.trim().replace(/\/+$/, '');
}

async function call<T>(url: string, path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${base(url)}${path}`, {
      method: body === undefined ? 'GET' : 'POST',
      headers: body === undefined ? undefined : { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new Error(`devhost did not answer at ${base(url)}. Check that it is running. If your browser asked whether this site may reach apps on your device, allow it.`);
  }
  if (response.status === 403) throw new Error('devhost refused this page. Update devhost, or add this address to DEVHOST_PLAYGROUND_ORIGINS.');
  if (!response.ok) throw new Error(`devhost answered ${response.status}.`);
  return response.json();
}

export async function listWorkflows(url: string): Promise<DevhostWorkflow[]> {
  const { workflows } = await call<{ workflows: DevhostWorkflow[] }>(url, '/dev/workflows');
  return workflows;
}

export function devhostActions(url: string, workflowId: string): ServerActions {
  return {
    source: 'your bot',
    missing: (id) => `Your bot gave no answer for ${id}.`,
    run: async (actionId, state) => {
      const answer = await call<{ ok: boolean; result?: Json; error?: string }>(url, '/dev/run', {
        workflow_id: workflowId,
        action_id: actionId,
        responses: state.responses,
        context: state.context,
      });
      if (!answer.ok) throw new Error(answer.error ?? 'devhost could not run the action.');
      return answer.result ?? 'None';
    },
  };
}
