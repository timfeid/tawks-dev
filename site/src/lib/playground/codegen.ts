/** Rust for the current definition: the template's own builder code while it is
 * unchanged, otherwise code that loads the exact JSON the playground shows. */
export function rustFor(definition: unknown, templateSource: string | null): string {
  if (templateSource) return templateSource;
  const json = JSON.stringify(definition, null, 2);
  let hashes = '#';
  while (json.includes(`"${hashes}`)) hashes += '#';
  return `use tawks_bot::workflow::CreateWorkflowDefinition;

/// Built in the tawks playground. Register it with \`bot.register_workflow(definition())\`.
pub fn definition() -> CreateWorkflowDefinition {
    serde_json::from_str(DEFINITION).expect("a valid workflow definition")
}

const DEFINITION: &str = r${hashes}"${json}"${hashes};
`;
}

type J = any;

const str = (s: string) => `'${s.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/\n/g, '\\n')}'`;
const pad = (depth: number) => '  '.repeat(depth);

/** A JSON value as a TypeScript literal, indented to sit at `depth`. */
function lit(value: J, depth: number): string {
  if (typeof value === 'string') return str(value);
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  const text = JSON.stringify(value, null, 2);
  if (!text.includes('\n')) return text;
  return text.replace(/\n/g, `\n${pad(depth)}`);
}

/** `[a, b]` with one item per line when there are any. */
function list(items: string[], depth: number): string {
  if (!items.length) return '[]';
  return `[\n${items.map((i) => `${pad(depth + 1)}${i},`).join('\n')}\n${pad(depth)}]`;
}

class Imports {
  used = new Set<string>();
  use(name: string) {
    this.used.add(name);
    return name;
  }
}

function variant(value: J): [string, J] {
  return typeof value === 'string' ? [value, null] : (Object.entries(value)[0] as [string, J]);
}

function condition(c: J, depth: number, im: Imports): string {
  const [kind, v] = variant(c);
  const C = () => im.use('Cond');
  switch (kind) {
    case 'Always':
      return `${C()}.always()`;
    case 'Not':
      return `${C()}.not(${condition(v, depth, im)})`;
    case 'And':
      return `${C()}.and(${list(v.map((x: J) => condition(x, depth + 1, im)), depth)})`;
    case 'Or':
      return `${C()}.or(${list(v.map((x: J) => condition(x, depth + 1, im)), depth)})`;
    case 'ResponseExists':
      return `${C()}.responseExists(${str(v)})`;
    case 'ResponseListEmpty':
      return `${C()}.responseListEmpty(${str(v)})`;
    case 'ResponseEquals':
      return `${C()}.responseEquals(${str(v.field)}, ${lit(v.value, depth)})`;
    case 'Gte':
      return `${C()}.gte(${str(v.field)}, ${lit(v.value, depth)})`;
    case 'SomeInList':
      return `${C()}.someInList(${str(v.list)}, ${str(v.as)}, ${condition(v.condition, depth, im)})`;
    default:
      return lit(c, depth);
  }
}

function inputType(t: J, depth: number, im: Imports): string {
  const [kind, v] = variant(t);
  const I = () => im.use('InputTypeBuilder');
  const opt = (x: J) => (x === null || x === undefined ? 'null' : lit(x, depth));
  switch (kind) {
    case 'Text':
      return `${I()}.text(${opt(v.placeholder)})`;
    case 'Number':
      return `${I()}.number(${opt(v.min)}, ${opt(v.max)})`;
    case 'TextArea':
      return `${I()}.textArea(${opt(v.placeholder)}, ${opt(v.rows)})`;
    case 'Checkbox':
      return `${I()}.checkbox(${str(v.label)})`;
    case 'Switch':
      return `${I()}.switch()`;
    case 'Wysiwyg':
      return `${I()}.wysiwyg()`;
    case 'List':
      return `${I()}.list(${str(v.from_responses_id)}, ${str(v.item_key)}, ${str(v.display_field)})`;
    default:
      return lit(t, depth);
  }
}

function input(i: J, depth: number, im: Imports): string {
  const lines = [`new ${im.use('WorkflowInputBuilder')}()`, `.type(${inputType(i.type, depth + 1, im)})`];
  if (i.required) lines.push('.required(true)');
  if (i.default_value !== null && i.default_value !== undefined) lines.push(`.defaultValue(${lit(i.default_value, depth + 1)})`);
  if (i.readonly) lines.push(`.readonly(${condition(i.readonly, depth + 1, im)})`);
  if (i.on_change_action) lines.push(`.onChangeAction(${str(i.on_change_action[0])}, ${lit(i.on_change_action[1], depth + 1)})`);
  lines.push('.build()');
  return lines.join(`\n${pad(depth + 1)}`);
}

function display(d: J, depth: number, im: Imports): string {
  const [kind, v] = variant(d.type);
  const content = (items: J[]) => list(items.map((x) => display(x, depth + 2, im)), depth + 1);
  const call = (() => {
    switch (kind) {
      case 'Text':
        return `.text(${str(v.text)})`;
      case 'Div':
        return `.div(${content(v.content)})`;
      case 'Button':
        return `.${v.type === 'submit' ? 'submit' : 'button'}(${content(v.content)}, ${v.input_map ? lit(v.input_map, depth + 1) : 'null'}, ${str(v.action_id)}, ${str(v.variant)}, ${str(v.size)})`;
      case 'Form':
        return `.form(${str(v.server_action_id)}, ${content(v.content)}, ${str(v.submit_when)})`;
      case 'Input':
        return `.input(${str(v.id)}, ${input(v.input, depth + 1, im)})`;
      case 'Image':
        return `.image(${str(v.src)}, ${str(v.alt_text)})`;
      case 'Badge':
        return `.badge(${str(v.variant)}, ${str(v.text)})`;
      case 'Link':
        return `.link(${str(v.href)}, ${content(v.content)})`;
      case 'List':
        return `.list(${str(v.items_key)}, ${str(v.as)}, ${content(v.content)})`;
      case 'Condition':
        return `.condition(${condition(v.condition, depth + 1, im)}, ${content(v.content)})`;
      case 'Card':
        return `.card(${content(v.content)}, ${lit(v.layout, depth + 1)})`;
      case 'Progress':
        return `.progress(${str(v.progress_percentage_key)}, ${str(v.percentage_class)})`;
      case 'Activity':
        return `.activity(${str(v.state)}, ${str(v.action)})`;
      case 'Icon':
        return v.lib === 'Lucide' && !v.url ? `.lucideIcon(${str(v.name)}, ${v.aria_label ? str(v.aria_label) : 'null'})` : null;
      case 'BattlefieldCanvas':
        return '.battlefieldCanvas()';
      case 'TracklinesCanvas':
        return '.tracklinesCanvas()';
      default:
        return null;
    }
  })();
  // A display the builder has no shorthand for is written as the JSON it is.
  if (call === null) return lit(d, depth);
  return `new ${im.use('WorkflowDisplayBuilder')}(${str(d.class ?? '')})\n${pad(depth + 1)}${call}\n${pad(depth + 1)}.build()`;
}

function visual(v: J, depth: number, im: Imports): string {
  const [kind, d] = variant(v.display);
  const content = (items: J[]) => list(items.map((x) => display(x, depth + 2, im)), depth + 1);
  const call = (() => {
    switch (kind) {
      case 'Block':
        return `.block(${str(d.class)}, ${content(d.content)})`;
      case 'Tile':
        return `.tile(${content(d.content)})`;
      case 'Modal':
        return `.modal(${content(d.content)})`;
      case 'Sticky':
        return `.sticky(${lit(d.position, depth + 1)}, ${content(d.content)})`;
      case 'Notification':
        return `.notification(${lit(d.variant, depth + 1)}, ${content(d.content)})`;
      case 'Redirect':
        return `.redirect(${str(d.url_key)})`;
      case 'Loading':
        return `.loading(${str(d.action_id)}, ${lit(d.inputs, depth + 1)})`;
      case 'Chat':
        return `.chat(${str(d.chat_id)})`;
      default:
        return `.display(${lit(v.display, depth + 1)})`;
    }
  })();
  return `new ${im.use('WorkflowVisualBuilder')}(${str(v.id)})\n${pad(depth + 1)}${call}\n${pad(depth + 1)}.build()`;
}

function action(a: J, depth: number, im: Imports): string {
  const [kind, v] = variant(a.action_type);
  const kindCall = (() => {
    switch (kind) {
      case 'RunServerAction':
        return '.runServerAction()';
      case 'NextNode':
        return '.nextNode()';
      case 'PreviousNode':
        return '.previousNode()';
      case 'Submit':
        return '.submit()';
      case 'Cancel':
        return '.cancel()';
      case 'Noop':
        return '.noop()';
      case 'TransformResponses':
        return `.transformResponses(${lit(v.ops, depth + 1)}, ${v.reconcile_node})`;
      case 'TransformThenRun':
        return `.transformThenRun(${lit(v.ops, depth + 1)}, ${v.reconcile_node}, ${str(v.server_action_id)}, ${v.input_map ? lit(v.input_map, depth + 1) : 'null'})`;
      default:
        return null;
    }
  })();
  if (kindCall === null) return lit(a, depth);
  const lines = [`new ${im.use('WorkflowActionBuilder')}(${str(a.id)})`, kindCall];
  if (a.progress_node === false) lines.push('.progressNode(false)');
  if (a.target) lines.push(`.target(${str(a.target)})`);
  if (a.allow_while_loading) lines.push('.allowWhileLoading()');
  lines.push('.build()');
  return lines.join(`\n${pad(depth + 1)}`);
}

function node(n: J, depth: number, im: Imports): string {
  const lines = [`new ${im.use('WorkflowNodeBuilder')}(${str(n.title)})`];
  if (n.description) lines.push(`.description(${str(n.description)})`);
  if (n.layout) lines.push(`.layout(${str(n.layout)})`);
  if (n.parent_id) lines.push(`.parentId(${str(n.parent_id)})`);
  if (n.condition) lines.push(`.condition(${condition(n.condition, depth + 1, im)})`);
  for (const v of n.visuals ?? []) lines.push(`.visual(${visual(v, depth + 1, im)})`);
  lines.push('.build()');
  return lines.join(`\n${pad(depth + 1)}`);
}

/** TypeScript that builds the current definition with the builders in `@tawks/bot`. */
export function typescriptFor(definition: J): string {
  const im = new Imports();
  const lines = [`new ${im.use('WorkflowDefinitionBuilder')}(${str(definition.id)}, ${str(definition.name)})`];
  if (definition.description) lines.push(`.description(${str(definition.description)})`);
  if (definition.starting_server_action) lines.push(`.startingServerAction(${str(definition.starting_server_action)})`);
  for (const [key, value] of Object.entries(definition.responses ?? {})) lines.push(`.response(${str(key)}, ${lit(value, 1)})`);
  for (const [key, value] of Object.entries(definition.context ?? {})) lines.push(`.context(${str(key)}, ${lit(value, 1)})`);
  if (definition.multi_instance_per_user) lines.push('.multiInstancePerUser()');
  for (const [id, n] of Object.entries(definition.nodes ?? {})) lines.push(`.addNode(${str(id)}, ${node(n, 1, im)})`);
  for (const a of definition.actions ?? []) lines.push(`.addAction(${action(a, 1, im)})`);
  lines.push('.build()');
  return `import { ${[...im.used].sort().join(', ')} } from '@tawks/bot';

// Built in the tawks playground. Register it with \`await bot.workflow(definition)\`.
export const definition = ${lines.join(`\n${pad(1)}`)};
`;
}
