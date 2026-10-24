import type { CreateWorkflowDefinition, WorkflowDisplay } from '@tawks/workflow-types';

/** A display from the Display reference, with the responses it reads. */
export type DisplayExample = {
  responses?: Record<string, unknown>;
  display: WorkflowDisplay;
};

/** The example as a one-page workflow, the way the playground and tawks see it. */
export function exampleDefinition(example: DisplayExample, visualId = 'page'): CreateWorkflowDefinition {
  return {
    id: 'example',
    name: 'Example',
    description: null,
    nodes: {
      home: {
        title: 'Example',
        description: null,
        visuals: [{ id: visualId, display: { Block: { class: 'flex flex-col gap-3 p-6', content: [example.display] } } }],
        layout: null,
        condition: null,
        parent_id: null,
      },
    },
    responses: (example.responses ?? {}) as CreateWorkflowDefinition['responses'],
    context: {},
    actions: [],
    starting_server_action: null,
    multi_instance_per_user: false,
  };
}
