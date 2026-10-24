import type { WorkflowResource } from '@tawks/workflow-types';
import type { Component } from 'svelte';

export interface WorkflowStateProvider {
  byInstance: Record<string, WorkflowResource>;
  activeChildForParent: Record<string, string>;
  returnedToParent: Record<string, string>;
  playAnimations(wf: WorkflowResource): void;
  processAction(instanceId: string, actionId: string, inputs: Record<string, any>): Promise<void>;
  customComponents?: Record<string, Component<any>>;
  getActiveInjectedForTile?(tileId: string): WorkflowResource | null;
  /** A short web link for an app path, when the host can make one. */
  shortenLink?(path: string): Promise<string>;
  /** The activity registered for a workflow, when the host knows of one. */
  findActivity?(workflowId: string): Promise<ActivityInfo | null>;
}

export type ActivityInfo = {
  workflow_id: string;
  name: string;
  client_url: string;
};
