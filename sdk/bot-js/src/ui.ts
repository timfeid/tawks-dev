import type {
  CreateWorkflowDefinition,
  InputType,
  JsonValue,
  NodeCondition,
  WorkflowAction,
  WorkflowDisplay,
  WorkflowNode,
  WorkflowVisual,
} from '@tawks/workflow-types';

/**
 * Shorthand for the displays most pages use. Each returns the plain JSON
 * tawks expects, so you can mix them with hand-written displays from the
 * Display reference.
 */

type Map = Partial<Record<string, JsonValue>>;

/** Text. `{key}` shows a response, such as `"Hello, {name}!"`. */
export const text = (value: string, cls = ''): WorkflowDisplay => ({ class: cls, type: { Text: { text: value } } });

/** A box that lays out its children, styled by Tailwind classes. */
export const div = (cls: string, content: WorkflowDisplay[]): WorkflowDisplay => ({ class: cls, type: { Div: { content } } });

export const image = (src: string, alt: string, cls = ''): WorkflowDisplay => ({
  class: cls,
  type: { Image: { src, alt_text: alt } },
});

export const badge = (value: string, variant = 'default', cls = ''): WorkflowDisplay => ({
  class: cls,
  type: { Badge: { variant, text: value } },
});

/** One copy of `content` per item of the list at `itemsKey`, with each item available as `{as.field}`. */
export const list = (itemsKey: string, as: string, content: WorkflowDisplay[], cls = ''): WorkflowDisplay => ({
  class: cls,
  type: { List: { items_key: itemsKey, as, content } },
});

/** Shows `content` only while the condition holds. */
export const when = (condition: NodeCondition, content: WorkflowDisplay[], cls = ''): WorkflowDisplay => ({
  class: cls,
  type: { Condition: { condition, content } },
});

type ButtonOptions = { variant?: string; size?: string; inputs?: Map; cls?: string };

/** Runs a workflow action. `inputs` values may use `{key}` templates. */
export const button = (label: string, actionId: string, options: ButtonOptions = {}): WorkflowDisplay => ({
  class: options.cls ?? '',
  type: {
    Button: {
      content: [text(label)],
      input_map: options.inputs ?? null,
      action_id: actionId,
      variant: options.variant ?? 'default',
      size: options.size ?? 'default',
      type: 'button',
    },
  },
});

/** Inside a form, sends the form. Typed values only reach an action through a form. */
export const submit = (label: string, actionId: string, options: ButtonOptions = {}): WorkflowDisplay => {
  const display = button(label, actionId, options);
  (display.type as { Button: { type: string } }).Button.type = 'submit';
  return display;
};

/** Sends every input inside it to `actionId`, when submitted or, with `onChange`, as people type. */
export const form = (actionId: string, content: WorkflowDisplay[], options: { cls?: string; onChange?: boolean } = {}): WorkflowDisplay => ({
  class: options.cls ?? 'flex flex-col gap-2',
  type: { Form: { server_action_id: actionId, content, submit_when: options.onChange ? 'OnChange' : 'OnSubmit' } },
});

type InputOptions = { required?: boolean; default?: JsonValue; cls?: string };

const input = (id: string, type: InputType, options: InputOptions): WorkflowDisplay => ({
  class: options.cls ?? '',
  type: {
    Input: {
      id,
      input: { readonly: null, type, default_value: options.default ?? null, required: options.required ?? false, on_change_action: null },
    },
  },
});

/** Saved under `id` in the responses. */
export const textInput = (id: string, placeholder: string | null = null, options: InputOptions = {}) =>
  input(id, { Text: { placeholder } }, options);

export const numberInput = (id: string, min: number | null = null, max: number | null = null, options: InputOptions = {}) =>
  input(id, { Number: { min, max } }, options);

export const switchInput = (id: string, options: InputOptions = {}) => input(id, 'Switch', options);

/** The outer box of a node's page. */
export const page = (content: WorkflowDisplay[], cls = 'mx-auto flex max-w-md flex-col gap-4 p-8', id = 'page'): WorkflowVisual => ({
  id,
  display: { Block: { class: cls, content } },
});

type NodeOptions = { parent?: string; condition?: NodeCondition; description?: string };

/** A screen. Children of a node show after it; conditions pick between them. */
export const node = (title: string, visuals: WorkflowVisual[], options: NodeOptions = {}): WorkflowNode => ({
  title,
  description: options.description ?? null,
  visuals,
  layout: null,
  condition: options.condition ?? null,
  parent_id: options.parent ?? null,
});

const action = (id: string, action_type: WorkflowAction['action_type'], target: string | null = null): WorkflowAction => ({
  id,
  action_type,
  allow_while_loading: false,
  target,
  progress_node: true,
});

export const actions = {
  /** Calls one of your server actions by the full id `serverAction()` returned. */
  run: (id: string, serverActionId: string) => action(id, 'RunServerAction', serverActionId),
  next: (id: string, target: string | null = null) => action(id, 'NextNode', target),
  back: (id: string) => action(id, 'PreviousNode'),
  submit: (id: string) => action(id, 'Submit'),
  cancel: (id: string) => action(id, 'Cancel'),
};

export const cond = {
  equals: (field: string, value: JsonValue): NodeCondition => ({ ResponseEquals: { field, value } }),
  exists: (field: string): NodeCondition => ({ ResponseExists: field }),
  atLeast: (field: string, value: JsonValue): NodeCondition => ({ Gte: { field, value } }),
  not: (condition: NodeCondition): NodeCondition => ({ Not: condition }),
  and: (...conditions: NodeCondition[]): NodeCondition => ({ And: conditions }),
  or: (...conditions: NodeCondition[]): NodeCondition => ({ Or: conditions }),
};

type DefinitionOptions = {
  description?: string;
  responses?: Map;
  context?: Map;
  actions?: WorkflowAction[];
  /** A server action run when someone opens the page, by its full id. */
  start?: string;
};

/** A whole workflow. `id` is short, such as `"hello"`; tawks scopes it to your bot. */
export const workflow = (
  id: string,
  name: string,
  nodes: Record<string, WorkflowNode>,
  options: DefinitionOptions = {},
): CreateWorkflowDefinition => ({
  id,
  name,
  description: options.description ?? null,
  nodes,
  responses: options.responses ?? {},
  context: options.context ?? {},
  actions: options.actions ?? [],
  starting_server_action: options.start ?? null,
  multi_instance_per_user: false,
});
