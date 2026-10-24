/**
 * A small in-browser stand-in for the tawks workflow engine, so a page can be
 * clicked through without a bot. It follows the engine's rules for choosing
 * nodes, checking conditions and applying transforms; server actions answer
 * from mocks you write, or from a bot running against devhost.
 */
import type {
  CreateWorkflowDefinition,
  NodeCondition,
  TransformCond,
  TransformOp,
  WorkflowAction,
  WorkflowNode,
} from '@tawks/workflow-types';

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type Map = Record<string, Json>;

export type Status = 'active' | 'completed' | 'cancelled';

export type State = {
  nodeId: string | null;
  history: string[];
  responses: Map;
  context: Map;
  status: Status;
  message: string | null;
};

export type LogKind = 'action' | 'server' | 'change' | 'warn' | 'error';
export type LogEntry = { kind: LogKind; text: string };

export type Mocks = Record<string, Json>;

/** Answers server actions. `run` resolves to undefined when there is no answer. */
export type ServerActions = {
  source: string;
  missing: (actionId: string) => string;
  run: (actionId: string, state: State) => Promise<Json | undefined>;
};

/**
 * A mock is a result, or a list of `{ if, then }` rules checked in order
 * against the instance, where `if` is a node condition and may be left out.
 */
export function fromMocks(mocks: Mocks): ServerActions {
  return {
    source: 'its mock',
    missing: (id) => `No mock for server action ${id}, or none of its rules matched. Add one under Mocks to see its result.`,
    run: async (id, state) => {
      const mock = mocks[id];
      if (mock === undefined) return undefined;
      if (!Array.isArray(mock)) return clone(mock);
      for (const rule of mock) {
        if (!rule || typeof rule !== 'object' || Array.isArray(rule) || !('then' in rule)) continue;
        const cond = (rule as Map).if as NodeCondition | undefined;
        if (!cond || evalCondition(cond, state.responses, state.context)) return clone((rule as Map).then);
      }
      return undefined;
    },
  };
}

type Effect = { kind: 'run'; actionId: string; inputMap: Record<string, string> | null } | { kind: 'cancel' } | { kind: 'next' } | { kind: 'previous' };

const clone = <T>(value: T): T => (value === undefined ? value : JSON.parse(JSON.stringify(value)));

const equal = (a: Json | undefined, b: Json | undefined) => JSON.stringify(a) === JSON.stringify(b);

// Paths: "a.b.0.c". A "$context." prefix reads and writes the context.

function split(path: string) {
  return path.split('.').filter(Boolean);
}

function isIndex(seg: string) {
  return /^\d+$/.test(seg);
}

export function getPath(root: Map, path: string): Json | undefined {
  const [first, ...rest] = split(path);
  if (first === undefined || !(first in root)) return undefined;
  let cur: Json | undefined = root[first];
  for (const seg of rest) {
    if (Array.isArray(cur) && isIndex(seg)) cur = cur[Number(seg)];
    else if (cur && typeof cur === 'object' && !Array.isArray(cur) && seg in cur) cur = (cur as Map)[seg];
    else return undefined;
  }
  return cur;
}

export function setPath(root: Map, path: string, value: Json): void {
  const parts = split(path);
  if (!parts.length) throw new Error('empty path');
  let cur: Json = root;
  for (let i = 0; i < parts.length; i++) {
    const seg = parts[i];
    const last = i === parts.length - 1;
    const nextIsIndex = !last && isIndex(parts[i + 1]);
    if (Array.isArray(cur)) {
      if (!isIndex(seg)) throw new Error(`expected array index at ${seg}`);
      const idx = Number(seg);
      while (cur.length <= idx) cur.push(null);
      if (last) cur[idx] = value;
      else {
        if (cur[idx] === null || typeof cur[idx] !== 'object') cur[idx] = nextIsIndex ? [] : {};
        cur = cur[idx];
      }
    } else if (cur && typeof cur === 'object') {
      const obj = cur as Map;
      if (last) obj[seg] = value;
      else {
        if (obj[seg] === null || typeof obj[seg] !== 'object' || obj[seg] === undefined) obj[seg] = nextIsIndex ? [] : {};
        cur = obj[seg];
      }
    } else {
      throw new Error(`cannot set into non-container at segment ${seg}`);
    }
  }
}

export function removePath(root: Map, path: string): void {
  const parts = split(path);
  const last = parts.pop();
  if (last === undefined) throw new Error('empty path');
  const parent = parts.length ? getPath(root, parts.join('.')) : root;
  if (Array.isArray(parent) && isIndex(last)) parent.splice(Number(last), 1);
  else if (parent && typeof parent === 'object') delete (parent as Map)[last];
}

function route(state: { responses: Map; context: Map }, path: string): [Map, string] {
  return path.startsWith('$context.') ? [state.context, path.slice('$context.'.length)] : [state.responses, path];
}

const getAny = (s: { responses: Map; context: Map }, path: string) => {
  const [root, p] = route(s, path);
  return getPath(root, p);
};
const setAny = (s: { responses: Map; context: Map }, path: string, value: Json) => {
  const [root, p] = route(s, path);
  setPath(root, p, value);
};
const removeAny = (s: { responses: Map; context: Map }, path: string) => {
  const [root, p] = route(s, path);
  removePath(root, p);
};

/** Node conditions, as the engine checks them. Paths here are dotted object keys only. */
function nested(map: Map, dotted: string): Json | undefined {
  const [first, ...rest] = dotted.split('.');
  if (!(first in map)) return undefined;
  let cur: Json | undefined = map[first];
  for (const part of rest) {
    if (cur && typeof cur === 'object' && !Array.isArray(cur) && part in cur) cur = (cur as Map)[part];
    else return undefined;
  }
  return cur;
}

export function evalCondition(cond: NodeCondition, responses: Map, context: Map): boolean {
  if (cond === 'Always') return true;
  if ('ResponseExists' in cond) return nested(responses, cond.ResponseExists) !== undefined;
  if ('ResponseEquals' in cond) return equal(nested(responses, cond.ResponseEquals.field), cond.ResponseEquals.value as Json);
  if ('ContextEquals' in cond) return equal(nested(context, cond.ContextEquals.field), cond.ContextEquals.value as Json);
  if ('ResponseListEmpty' in cond) {
    const v = nested(responses, cond.ResponseListEmpty);
    return Array.isArray(v) ? v.length === 0 : true;
  }
  if ('Gte' in cond) {
    const a = nested(responses, cond.Gte.field);
    const b = cond.Gte.value as Json;
    if (typeof a === 'number' && typeof b === 'number') return a >= b;
    if (typeof a === 'string' && typeof b === 'string') return a >= b;
    if (typeof a === 'boolean' && typeof b === 'boolean') return a || !b;
    return false;
  }
  if ('SomeInList' in cond) {
    const items = nested(responses, cond.SomeInList.list);
    if (!Array.isArray(items)) return false;
    return items.some((item) => evalCondition(cond.SomeInList.condition, { ...responses, [cond.SomeInList.as]: item }, context));
  }
  if ('And' in cond) return cond.And.every((c) => evalCondition(c, responses, context));
  if ('Or' in cond) return cond.Or.some((c) => evalCondition(c, responses, context));
  if ('Not' in cond) return !evalCondition(cond.Not, responses, context);
  return false;
}

function evalTransformCond(cond: TransformCond, s: { responses: Map; context: Map }): boolean {
  if (cond === 'Always') return true;
  if ('Not' in cond) return !evalTransformCond(cond.Not, s);
  if ('And' in cond) return cond.And.every((c) => evalTransformCond(c, s));
  if ('Or' in cond) return cond.Or.some((c) => evalTransformCond(c, s));
  if ('Exists' in cond) return getAny(s, cond.Exists.path) !== undefined;
  if ('Equals' in cond) return equal(getAny(s, cond.Equals.path), cond.Equals.value as Json);
  if ('ListNotEmpty' in cond) {
    const v = getAny(s, cond.ListNotEmpty.path);
    return Array.isArray(v) && v.length > 0;
  }
  if ('EqualsPath' in cond) {
    const a = getAny(s, cond.EqualsPath.left);
    const b = getAny(s, cond.EqualsPath.right);
    return a !== undefined && b !== undefined && equal(a, b);
  }
  return false;
}

function interpolate(template: Json, s: { responses: Map; context: Map }): Json {
  if (typeof template === 'string') {
    return template.replace(/\{([^}]*)\}/g, (_, key: string) => {
      const k = key.trim();
      if (!k) return '{}';
      const v = getAny(s, k);
      if (v === undefined || v === null) return '';
      return typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean' ? String(v) : JSON.stringify(v);
    });
  }
  if (Array.isArray(template)) return template.map((t) => interpolate(t, s));
  if (template && typeof template === 'object') {
    return Object.fromEntries(Object.entries(template).map(([k, v]) => [k, interpolate(v, s)]));
  }
  return template;
}

function ensureArray(s: { responses: Map; context: Map }, path: string): Json[] {
  if (getAny(s, path) === undefined) setAny(s, path, []);
  const v = getAny(s, path);
  if (!Array.isArray(v)) throw new Error(`expected array at ${path}`);
  return v;
}

/** Applies transform ops in place, returning what they asked the engine to do next. */
export function applyTransforms(s: { responses: Map; context: Map }, ops: TransformOp[]): Effect[] {
  const effects: Effect[] = [];
  const run = (op: TransformOp) => {
    if (op === 'EmitCancel') return void effects.push({ kind: 'cancel' });
    if (op === 'EmitNextNode') return void effects.push({ kind: 'next' });
    if (op === 'EmitPreviousNode') return void effects.push({ kind: 'previous' });
    if ('EmitRunServerAction' in op) {
      const e = op.EmitRunServerAction;
      return void effects.push({ kind: 'run', actionId: e.action_id, inputMap: (e.input_map as Record<string, string>) ?? null });
    }
    if ('When' in op) {
      for (const inner of evalTransformCond(op.When.cond, s) ? op.When.ops : op.When.else_ops) run(inner);
      return;
    }
    if ('Set' in op) return setAny(s, op.Set.path, clone(op.Set.value as Json));
    if ('Remove' in op) return removeAny(s, op.Remove.path);
    if ('MergeObject' in op) {
      const existing = getAny(s, op.MergeObject.path);
      if (existing && typeof existing === 'object' && !Array.isArray(existing)) Object.assign(existing, clone(op.MergeObject.value as Map));
      else setAny(s, op.MergeObject.path, clone(op.MergeObject.value as Map));
      return;
    }
    if ('Append' in op) return void ensureArray(s, op.Append.path).push(clone(op.Append.value as Json));
    if ('AppendUnique' in op) {
      const arr = ensureArray(s, op.AppendUnique.path);
      if (!arr.some((x) => equal(x, op.AppendUnique.value as Json))) arr.push(clone(op.AppendUnique.value as Json));
      return;
    }
    if ('FilterArray' in op) {
      const { path, as_key, predicate } = op.FilterArray;
      const arr = getAny(s, path) ?? [];
      if (!Array.isArray(arr)) throw new Error(`FilterArray target is not an array: ${path}`);
      const kept = arr.filter((item) => evalTransformCond(predicate, { responses: { ...s.responses, [as_key]: item }, context: s.context }));
      return setAny(s, path, kept);
    }
    if ('MapArray' in op) {
      const { path, as_key, template } = op.MapArray;
      const arr = getAny(s, path) ?? [];
      if (!Array.isArray(arr)) throw new Error(`MapArray target is not an array: ${path}`);
      const mapped = arr.map((item) => interpolate(template as Json, { responses: { ...s.responses, [as_key]: item }, context: s.context }));
      return setAny(s, path, mapped);
    }
    if ('Copy' in op) {
      const v = getAny(s, op.Copy.from);
      if (v !== undefined) setAny(s, op.Copy.to, clone(v));
      return;
    }
    if ('Move' in op) {
      const v = getAny(s, op.Move.from);
      if (v !== undefined) {
        setAny(s, op.Move.to, clone(v));
        removeAny(s, op.Move.from);
      }
    }
  };
  ops.forEach(run);
  return effects;
}

/** The engine's choice of next node: matching conditioned children, then plain children, then the same for root nodes. */
export function nextNode(def: CreateWorkflowDefinition, from: string, state: Pick<State, 'responses' | 'context'>): string | null {
  const ids = Object.keys(def.nodes).sort();
  const node = (id: string) => def.nodes[id] as WorkflowNode;
  const conditioned = (n: WorkflowNode) => n.condition !== null && n.condition !== undefined && n.condition !== 'Always';
  const passes = (n: WorkflowNode) => !n.condition || evalCondition(n.condition, state.responses, state.context);
  const child = (n: WorkflowNode) => n.parent_id === from;
  const root = (n: WorkflowNode) => !n.parent_id;
  return (
    ids.find((id) => child(node(id)) && conditioned(node(id)) && passes(node(id))) ??
    ids.find((id) => child(node(id)) && !conditioned(node(id))) ??
    ids.find((id) => root(node(id)) && conditioned(node(id)) && passes(node(id))) ??
    ids.find((id) => root(node(id)) && !conditioned(node(id))) ??
    null
  );
}

export function start(def: CreateWorkflowDefinition, user: Map): State {
  const state: State = {
    nodeId: null,
    history: [],
    responses: clone((def.responses ?? {}) as Map),
    context: { ...clone((def.context ?? {}) as Map), $user: user, $anonymous: false },
    status: 'active',
    message: null,
  };
  state.nodeId = nextNode(def, '', state);
  return state;
}

/** Starts an instance and runs the definition's starting server action, if it has one. */
export async function begin(def: CreateWorkflowDefinition, user: Map, server: ServerActions): Promise<{ state: State; log: LogEntry[] }> {
  const state = start(def, user);
  const log: LogEntry[] = [];
  if (def.starting_server_action) {
    try {
      await runServerAction(def, state, def.starting_server_action, server, log, true);
    } catch (e) {
      log.push({ kind: 'error', text: e instanceof Error ? e.message : String(e) });
    }
  }
  return { state, log };
}

/** Applies a server action's result the way the engine would. */
export function applyResult(state: State, result: Json, log: LogEntry[]): void {
  if (result === 'None' || result === null) return;
  if (typeof result !== 'object' || Array.isArray(result)) {
    log.push({ kind: 'warn', text: `The playground does not act on the ${JSON.stringify(result)} result.` });
    return;
  }
  const [kind, body] = Object.entries(result)[0] ?? [];
  const asMap = (v: Json) => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Map) : {});
  switch (kind) {
    case 'UpdateResponses':
      Object.assign(state.responses, clone(asMap(body)));
      break;
    case 'UpdateContext':
      Object.assign(state.context, clone(asMap(body)));
      break;
    case 'UpdateContextAndResponses':
      Object.assign(state.responses, clone(asMap(asMap(body).responses)));
      Object.assign(state.context, clone(asMap(asMap(body).context)));
      break;
    case 'CompleteWorkflow':
      Object.assign(state.responses, clone(asMap(asMap(body).responses)));
      state.status = 'completed';
      state.message = (asMap(body).message as string) ?? null;
      break;
    case 'CancelWorkflow':
      state.status = 'cancelled';
      break;
    case 'Error':
      log.push({ kind: 'error', text: `The bot answered with an error: ${String(body)}` });
      break;
    default:
      log.push({ kind: 'warn', text: `The playground does not act on ${kind} results; tawks would.` });
  }
}

const ADVANCING = ['UpdateResponses', 'UpdateContext', 'UpdateContextAndResponses'];

/**
 * The engine's re-pick after a server action updates an instance: within the
 * current node's group, a matching conditioned child, then a plain child, then
 * the group's root, then the starting node.
 */
export function advance(def: CreateWorkflowDefinition, state: State): void {
  const current = state.nodeId;
  const node = (id: string) => def.nodes[id] as WorkflowNode | undefined;
  if (current === null) {
    state.nodeId = nextNode(def, '', state);
    return;
  }
  let root = current;
  for (let i = 0; i < 64; i++) {
    const parent = node(root)?.parent_id;
    if (!parent) break;
    root = parent;
  }
  const ids = Object.keys(def.nodes).sort();
  const plain = (n: WorkflowNode) => !n.condition || n.condition === 'Always';
  const passes = (n: WorkflowNode) => !n.condition || evalCondition(n.condition, state.responses, state.context);
  const childOfRoot = (id: string) => node(id)?.parent_id === root;
  const moveTo = (id: string) => {
    if (id !== current) {
      state.history.push(current);
      state.nodeId = id;
    }
  };
  const conditioned = ids.find((id) => childOfRoot(id) && !plain(node(id)!) && passes(node(id)!));
  if (conditioned) return moveTo(conditioned);
  const unconditioned = ids.find((id) => childOfRoot(id) && plain(node(id)!));
  if (unconditioned) return moveTo(unconditioned);
  if (node(root) && passes(node(root)!)) return moveTo(root);
  const first = nextNode(def, '', state);
  if (first) moveTo(first);
}

async function runServerAction(
  def: CreateWorkflowDefinition,
  state: State,
  actionId: string,
  server: ServerActions,
  log: LogEntry[],
  progressNode: boolean,
) {
  const result = await server.run(actionId, clone(state));
  if (result === undefined) {
    log.push({ kind: 'warn', text: server.missing(actionId) });
    return;
  }
  log.push({ kind: 'server', text: `${actionId} answered from ${server.source}` });
  applyResult(state, result, log);
  const kind = result && typeof result === 'object' && !Array.isArray(result) ? Object.keys(result)[0] : null;
  if (progressNode && state.status === 'active' && kind && ADVANCING.includes(kind)) advance(def, state);
}

async function applyEffects(def: CreateWorkflowDefinition, state: State, effects: Effect[], server: ServerActions, log: LogEntry[]) {
  for (const e of effects) {
    if (e.kind === 'next') moveNext(def, state, null, log);
    else if (e.kind === 'previous') movePrevious(state, log);
    else if (e.kind === 'cancel') state.status = 'cancelled';
    else {
      if (e.inputMap) {
        for (const [key, path] of Object.entries(e.inputMap)) state.responses[key] = getAny(state, path) ?? null;
      }
      await runServerAction(def, state, e.actionId, server, log, true);
    }
  }
}

function moveNext(def: CreateWorkflowDefinition, state: State, target: string | null, log: LogEntry[]) {
  const to = target && def.nodes[target] ? target : target ? null : nextNode(def, state.nodeId ?? '', state);
  if (!to) {
    log.push({ kind: 'error', text: target ? `No node named ${target}.` : 'No node to move to.' });
    return;
  }
  if (state.nodeId) state.history.push(state.nodeId);
  state.nodeId = to;
}

function movePrevious(state: State, log: LogEntry[]) {
  const back = state.history.pop();
  if (back === undefined) log.push({ kind: 'error', text: 'There is no earlier node to go back to.' });
  else state.nodeId = back;
}

/** Runs one workflow action with the inputs the page sent. Returns the new state and what happened. */
export async function act(
  def: CreateWorkflowDefinition,
  current: State,
  actionId: string,
  inputs: Map,
  server: ServerActions,
): Promise<{ state: State; log: LogEntry[] }> {
  const state = clone(current);
  const log: LogEntry[] = [];
  if (state.status !== 'active') {
    log.push({ kind: 'warn', text: 'This instance is finished. Restart it to try again.' });
    return { state, log };
  }
  for (const key of Object.keys(inputs)) {
    if (key.startsWith('$')) {
      log.push({ kind: 'warn', text: `Ignored input ${key}: keys starting with $ are reserved.` });
      continue;
    }
    state.responses[key] = clone(inputs[key]);
  }
  const action = (def.actions ?? []).find((a: WorkflowAction) => a.id === actionId);
  if (!action) {
    log.push({ kind: 'error', text: `The page asked for action ${actionId}, but the workflow has no such action.` });
    return { state, log };
  }
  log.push({ kind: 'action', text: `${actionId}` });
  const t = action.action_type;
  try {
    if (t === 'NextNode') moveNext(def, state, action.target ?? null, log);
    else if (t === 'PreviousNode') movePrevious(state, log);
    else if (t === 'Submit') state.status = 'completed';
    else if (t === 'Cancel') state.status = 'cancelled';
    else if (t === 'Noop') {
      // Nothing to do.
    } else if (t === 'RunServerAction') await runServerAction(def, state, action.target ?? action.id, server, log, action.progress_node);
    else if (typeof t === 'object' && 'TransformResponses' in t) {
      await applyEffects(def, state, applyTransforms(state, t.TransformResponses.ops), server, log);
    } else if (typeof t === 'object' && 'TransformThenRun' in t) {
      const { ops, server_action_id, input_map } = t.TransformThenRun;
      const effects = applyTransforms(state, ops);
      if (input_map) for (const [key, path] of Object.entries(input_map)) state.responses[key] = getAny(state, path as string) ?? null;
      await runServerAction(def, state, server_action_id, server, log, action.progress_node);
      await applyEffects(def, state, effects, server, log);
    } else if (typeof t === 'object' && 'StartWorkflow' in t) {
      log.push({ kind: 'warn', text: 'Starting another workflow needs tawks; the playground skips it.' });
    } else {
      log.push({ kind: 'warn', text: `Unknown action type ${JSON.stringify(t)}.` });
    }
  } catch (e) {
    log.push({ kind: 'error', text: e instanceof Error ? e.message : String(e) });
  }
  return { state, log };
}
