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

const exactly = (v: J, fields: string[]) =>
  v !== null && typeof v === 'object' && Object.keys(v).length === fields.length && fields.every((f) => f in v);

const TEXT_STYLE = { numeric: false, clamp: 0, align: 'Start', struck: false, lines: false, themed: false };
const ROLE_HELPERS: Record<string, string> = {
  Display: 'displayTitle',
  Title: 'title',
  Heading: 'heading',
  Subheading: 'subheading',
  Lead: 'lead',
  Body: 'body',
  Fine: 'fine',
  Eyebrow: 'eyebrow',
  Code: 'code',
  Stat: 'statText',
};

/** `{ a: 1, b: [...] }` from the entries that are set, or nothing when none are. */
function options(entries: [string, string | null][], depth: number): string | null {
  const set = entries.filter((e): e is [string, string] => e[1] !== null);
  if (!set.length) return null;
  const inline = `{ ${set.map(([k, v]) => `${k}: ${v}`).join(', ')} }`;
  if (!inline.includes('\n') && inline.length <= 80) return inline;
  return `{\n${set.map(([k, v]) => `${pad(depth + 1)}${k}: ${v},`).join('\n')}\n${pad(depth)}}`;
}

/** A `ui` helper call, ending with its options when any are set. */
function helper(im: Imports, name: string, args: string[], opts: string | null = null): string {
  return `${im.use('ui')}.${name}(${[...args, ...(opts ? [opts] : [])].join(', ')})`;
}

const unless = <T>(value: T, fallback: T, code: (v: T) => string) => (value === fallback || value === undefined ? null : code(value));
const optStr = (value: J) => (value === null || value === undefined || value === '' ? null : str(value));

function act(a: J, depth: number, im: Imports): string | null {
  if (!exactly(a, ['action', 'inputs', 'disabled'])) return null;
  const opts = options(
    [
      ['inputs', a.inputs ? lit(a.inputs, depth + 1) : null],
      ['disabled', a.disabled ? condition(a.disabled, depth + 1, im) : null],
    ],
    depth,
  );
  return helper(im, 'act', [str(a.action)], opts);
}

/** A display as a `ui` building block call, or null when it is not one the helpers make exactly. */
function block(kind: string, v: J, mark: string | null, depth: number, im: Imports): string | null {
  const kids = (items: J[], at = depth) => list(items.map((x) => display(x, at + 1, im)), at);
  const kidsOpt = (items: J[]) => (items?.length ? kids(items, depth + 1) : null);
  const o = (entries: [string, string | null][]) => options([...entries, ['mark', mark === null ? null : str(mark)]], depth);
  const tone = (t: J): [string, string | null] => ['tone', optStr(t)];
  const bool = (name: string, value: J): [string, string | null] => [name, value ? 'true' : null];
  switch (kind) {
    case 'Typography': {
      if (!exactly(v, ['text', 'role', 'tone', 'style']) || !exactly(v.style, Object.keys(TEXT_STYLE)) || !ROLE_HELPERS[v.role]) return null;
      const st = v.style;
      return helper(im, ROLE_HELPERS[v.role], [str(v.text)], o([
        tone(v.tone),
        bool('numeric', st.numeric),
        ['clamp', unless(st.clamp, 0, String)],
        ['align', unless(st.align, 'Start', str)],
        bool('struck', st.struck),
        bool('lines', st.lines),
        bool('themed', st.themed),
      ]));
    }
    case 'Stack': {
      if (!exactly(v, ['direction', 'gap', 'align', 'justify', 'wrap', 'content'])) return null;
      const isRow = v.direction === 'Row';
      return helper(im, isRow ? 'row' : 'column', [str(v.gap), kids(v.content)], o([
        ['align', unless(v.align, isRow ? 'Center' : 'Stretch', str)],
        ['justify', unless(v.justify, 'Start', str)],
        ['wrap', v.wrap === isRow ? null : String(v.wrap)],
      ]));
    }
    case 'Grid':
      if (!exactly(v, ['columns', 'gap', 'content', 'fixed'])) return null;
      return helper(im, 'grid', [String(v.columns), kids(v.content)], o([['gap', unless(v.gap, 'Loose', str)], bool('fixed', v.fixed)]));
    case 'Panel':
      if (!exactly(v, ['tone', 'content'])) return null;
      return helper(im, 'panel', [kids(v.content)], o([tone(v.tone)]));
    case 'Hero':
      if (!exactly(v, ['eyebrow', 'title', 'lead', 'aside', 'actions'])) return null;
      return helper(im, 'hero', [str(v.title)], o([
        ['eyebrow', optStr(v.eyebrow)],
        ['lead', optStr(v.lead)],
        ['aside', kidsOpt(v.aside)],
        ['actions', kidsOpt(v.actions)],
      ]));
    case 'Section':
      if (!exactly(v, ['eyebrow', 'title', 'lead', 'aside', 'content'])) return null;
      return helper(im, 'section', [str(v.title), kids(v.content)], o([
        ['eyebrow', optStr(v.eyebrow)],
        ['lead', optStr(v.lead)],
        ['aside', kidsOpt(v.aside)],
      ]));
    case 'Feature':
      if (!exactly(v, ['icon', 'title', 'body', 'href'])) return null;
      return helper(im, 'feature', [str(v.title), str(v.body)], o([['icon', optStr(v.icon)], ['href', optStr(v.href)]]));
    case 'Chip':
      if (!exactly(v, ['icon', 'text'])) return null;
      return helper(im, 'chip', [str(v.text)], o([['icon', optStr(v.icon)]]));
    case 'Icon':
      if (!exactly(v, ['lib', 'name', 'aria_label', 'url', 'size', 'tone']) || v.lib !== 'Lucide' || v.url !== null) return null;
      return helper(im, 'icon', [str(v.name), ...(v.size === 'Inherit' && !v.tone && !v.aria_label && mark === null ? [] : [str(v.size)])], o([
        tone(v.tone),
        ['label', optStr(v.aria_label)],
      ]));
    case 'Link': {
      if (!exactly(v, ['href', 'content', 'style'])) return null;
      const only = v.content.length === 1 ? v.content[0] : null;
      const label = only && exactly(only, ['type']) ? only.type.Typography : null;
      const plainBody =
        label && exactly(label, ['text', 'role', 'tone', 'style']) && label.role === 'Body' && label.tone === '' && JSON.stringify(label.style) === JSON.stringify(TEXT_STYLE);
      const named: Record<string, string> = { Primary: 'linkPrimary', Secondary: 'linkSecondary', Inline: 'linkInline' };
      if (named[v.style] && plainBody) return helper(im, named[v.style], [str(v.href), str(label.text)], o([]));
      if (v.style === 'Card') return helper(im, 'linkCard', [str(v.href), kids(v.content)], o([]));
      return helper(im, 'link', [str(v.href), str(v.style), kids(v.content)], o([]));
    }
    case 'Image':
      if (!exactly(v, ['src', 'alt_text'])) return null;
      return helper(im, 'image', [str(v.src), str(v.alt_text)], o([]));
    case 'Badge':
      if (!exactly(v, ['variant', 'text', 'tone', 'icon']) || v.variant !== '') return null;
      return helper(im, 'badge', [str(v.text), str(v.tone)], o([['icon', optStr(v.icon)]]));
    case 'Progress':
      if (!exactly(v, ['progress_percentage_key', 'tone']) || v.tone === '') return null;
      return helper(im, 'progress', [str(v.progress_percentage_key), str(v.tone)], o([]));
    case 'Countdown':
      if (!exactly(v, ['until', 'as', 'text', 'shake_under_ms_left', 'role', 'tone'])) return null;
      return helper(im, 'countdown', [str(v.until), str(v.as), str(v.text)], o([
        ['role', unless(v.role, 'Body', str)],
        tone(v.tone),
        ['shakeUnderMsLeft', v.shake_under_ms_left === null ? null : String(v.shake_under_ms_left)],
      ]));
    case 'Time':
      if (!exactly(v, ['opts', 'key', 'role', 'tone'])) return null;
      return helper(im, 'time', [str(v.key), lit(v.opts, depth)], o([['role', unless(v.role, 'Fine', str)], tone(v.tone)]));
    case 'ScoreTicker':
      if (!exactly(v, ['value_key', 'icon'])) return null;
      return helper(im, 'scoreTicker', [str(v.value_key), display(v.icon, depth, im)], o([]));
    case 'Overlay':
      if (!exactly(v, ['corner', 'content'])) return null;
      return helper(im, 'overlay', [str(v.corner), kids(v.content)], o([]));
    case 'Condition':
      if (!exactly(v, ['condition', 'content'])) return null;
      return helper(im, 'when', [condition(v.condition, depth, im), kids(v.content)], o([]));
    case 'Action': {
      if (!exactly(v, ['label', 'icon', 'look', 'act', 'full_width', 'submit', 'scroll_to'])) return null;
      const run = act(v.act, depth, im);
      if (run === null) return null;
      const a = v.act;
      if (v.look === 'Primary' && v.submit && !v.icon && !v.full_width && !v.scroll_to && a.inputs === null && a.disabled === null)
        return helper(im, 'submit', [str(v.label), str(a.action)], o([]));
      const opts = (icon: boolean) =>
        o([
          ['icon', icon ? optStr(v.icon) : null],
          bool('fullWidth', v.full_width),
          bool('submit', v.submit),
          ['scrollTo', optStr(v.scroll_to)],
        ]);
      if (v.look === 'Icon' && v.icon) return helper(im, 'iconAction', [str(v.icon), str(v.label), run], opts(false));
      if (v.look === 'Primary') return helper(im, 'primaryAction', [str(v.label), run], opts(true));
      if (v.look === 'Secondary') return helper(im, 'secondaryAction', [str(v.label), run], opts(true));
      return helper(im, 'action', [str(v.label), str(v.look), run], opts(true));
    }
    case 'Notice':
      if (!exactly(v, ['tone', 'icon', 'title', 'body', 'actions'])) return null;
      return helper(im, 'notice', [str(v.tone), str(v.body)], o([['icon', optStr(v.icon)], ['title', optStr(v.title)], ['actions', kidsOpt(v.actions)]]));
    case 'TabBar': {
      if (!exactly(v, ['style', 'tabs'])) return null;
      const tabs = v.tabs.map((t: J) => {
        if (!exactly(t, ['label', 'icon', 'active', 'act', 'href'])) return null;
        const icon = options([['icon', optStr(t.icon)]], depth + 1);
        const active = condition(t.active, depth + 1, im);
        if (t.act && t.href === null) {
          const run = act(t.act, depth + 1, im);
          return run && helper(im, 'tab', [str(t.label), active, run], icon);
        }
        if (t.href !== null && t.act === null) return helper(im, 'tabLink', [str(t.label), active, str(t.href)], icon);
        return null;
      });
      if (tabs.includes(null)) return null;
      return helper(im, 'tabs', [str(v.style), list(tabs, depth)], o([]));
    }
    case 'ButtonGroup': {
      if (!exactly(v, ['buttons'])) return null;
      const buttons = v.buttons.map((b: J) => {
        if (!exactly(b, ['label', 'act'])) return null;
        const run = act(b.act, depth + 1, im);
        return run && helper(im, 'groupButton', [str(b.label), run]);
      });
      if (buttons.includes(null)) return null;
      return helper(im, 'buttonGroup', [list(buttons, depth)], o([]));
    }
    case 'Stepper': {
      if (!exactly(v, ['title', 'subtitle', 'previous', 'next'])) return null;
      const prev = v.previous === null ? 'null' : act(v.previous, depth, im);
      const next = v.next === null ? 'null' : act(v.next, depth, im);
      if (prev === null || next === null) return null;
      return helper(im, 'stepper', [str(v.title), prev, next], o([['subtitle', optStr(v.subtitle)]]));
    }
    case 'Row': {
      if (!exactly(v, ['lead', 'title', 'meta', 'trail', 'state', 'href', 'act'])) return null;
      const run = v.act === null ? null : act(v.act, depth + 1, im);
      if (v.act !== null && run === null) return null;
      return helper(im, 'listRow', [str(v.title)], o([
        ['lead', kidsOpt(v.lead)],
        ['meta', optStr(v.meta)],
        ['trail', kidsOpt(v.trail)],
        ['state', optStr(v.state)],
        ['href', optStr(v.href)],
        ['act', run],
      ]));
    }
    case 'Choice': {
      if (!exactly(v, ['content', 'state', 'act', 'direction'])) return null;
      const run = act(v.act, depth, im);
      if (run === null) return null;
      return helper(im, 'choice', [kids(v.content), run], o([['state', optStr(v.state)], bool('across', v.direction === 'Row')]));
    }
    case 'Stat':
      if (!exactly(v, ['value', 'label', 'tone'])) return null;
      return helper(im, 'stat', [str(v.value), str(v.label)], o([tone(v.tone)]));
    case 'Meter':
      if (!exactly(v, ['label', 'value', 'percent', 'tone'])) return null;
      return helper(im, 'meter', [str(v.percent)], o([['label', optStr(v.label)], ['value', optStr(v.value)], tone(v.tone)]));
    case 'Segments':
      if (!exactly(v, ['items_key', 'as', 'state', 'label'])) return null;
      return helper(im, 'segments', [str(v.items_key), str(v.as), str(v.state)], o([['label', optStr(v.label)]]));
    case 'Empty':
      if (!exactly(v, ['icon', 'title', 'body', 'loading', 'actions'])) return null;
      return helper(im, v.loading ? 'loading' : 'empty', [str(v.title)], o([
        ['icon', optStr(v.icon)],
        ['body', optStr(v.body)],
        ['actions', kidsOpt(v.actions)],
      ]));
    case 'Person':
      if (!exactly(v, ['avatar', 'name', 'meta', 'size'])) return null;
      return helper(im, 'person', [str(v.avatar), str(v.name)], o([['meta', optStr(v.meta)], ['size', unless(v.size, 'Medium', str)]]));
    case 'AvatarStack':
      if (!exactly(v, ['items_key', 'as', 'avatar', 'name', 'max'])) return null;
      return helper(im, 'avatarStack', [str(v.items_key), str(v.as), str(v.avatar), str(v.name)], o([['max', unless(v.max, 5, String)]]));
    case 'Divider':
      if (!exactly(v, ['label'])) return null;
      return v.label === null ? helper(im, 'divider', [], o([])) : helper(im, 'labelledDivider', [str(v.label)], o([]));
    case 'Log':
      if (!exactly(v, ['text'])) return null;
      return helper(im, 'log', [str(v.text)], o([]));
    case 'Split':
      if (!exactly(v, ['media', 'content'])) return null;
      return helper(im, 'split', [kids(v.media), kids(v.content)], o([]));
    case 'ActionBar':
      if (!exactly(v, ['content'])) return null;
      return helper(im, 'actionBar', [kids(v.content)], o([]));
    case 'Media':
      if (!exactly(v, ['src', 'alt', 'aspect', 'size', 'shape', 'fit', 'badge', 'caption', 'tint'])) return null;
      return helper(im, 'media', [str(v.src), str(v.alt), str(v.aspect), str(v.size)], o([
        ['shape', unless(v.shape, 'Rounded', str)],
        ['fit', unless(v.fit, 'Cover', str)],
        ['badge', kidsOpt(v.badge)],
        ['caption', kidsOpt(v.caption)],
        ['tint', optStr(v.tint)],
      ]));
    case 'Items':
      if (!exactly(v, ['items_key', 'as', 'layout', 'content'])) return null;
      return helper(im, 'items', [str(v.items_key), str(v.as), lit(v.layout, depth), kids(v.content)], o([]));
    case 'Form':
      if (!exactly(v, ['server_action_id', 'content', 'submit_when'])) return null;
      return helper(im, 'form', [str(v.server_action_id), kids(v.content)], o([bool('onChange', v.submit_when === 'OnChange')]));
    case 'Input': {
      const i = v.input;
      if (!exactly(v, ['id', 'input']) || !exactly(i, ['readonly', 'type', 'default_value', 'required', 'on_change_action'])) return null;
      if (i.readonly !== null || i.on_change_action !== null) return null;
      const opts = o([bool('required', i.required), ['default', i.default_value === null ? null : lit(i.default_value, depth + 1)]]);
      const [type, t] = variant(i.type);
      const opt = (x: J) => (x === null ? 'null' : lit(x, depth));
      if (type === 'Text' && exactly(t, ['placeholder'])) return helper(im, 'textInput', [str(v.id), opt(t.placeholder)], opts);
      if (type === 'Number' && exactly(t, ['min', 'max'])) return helper(im, 'numberInput', [str(v.id), opt(t.min), opt(t.max)], opts);
      if (type === 'Switch') return helper(im, 'switchInput', [str(v.id)], opts);
      return null;
    }
    default:
      return null;
  }
}

function display(raw: J, depth: number, im: Imports): string {
  // Classes from before the building blocks style nothing any more.
  const { class: _class, ...d } = raw;
  if (d.mark === null) delete d.mark;
  if (!exactly(d, d.mark === undefined ? ['type'] : ['type', 'mark'])) return lit(d, depth);
  const [kind, v] = variant(d.type);
  const made = block(kind, v, d.mark ?? null, depth, im);
  if (made !== null) return made;
  if (d.mark !== undefined) return lit(d, depth);
  const content = (items: J[]) => list(items.map((x) => display(x, depth + 2, im)), depth + 1);
  const call = (() => {
    switch (kind) {
      case 'Text':
        return `.text(${str(v.text)})`;
      case 'Div':
        return `.div(${content(v.content)})`;
      case 'Button':
        return `.${v.type === 'submit' ? 'submit' : 'button'}(${content(v.content)}, ${v.input_map ? lit(v.input_map, depth + 1) : 'null'}, ${str(v.action_id)}, ${str(v.variant)}, ${str(v.size)})`;
      case 'Input':
        return `.input(${str(v.id)}, ${input(v.input, depth + 1, im)})`;
      case 'Badge':
        return exactly(v, ['variant', 'text', 'tone', 'icon']) && v.tone === '' && v.icon === null ? `.badge(${str(v.variant)}, ${str(v.text)})` : null;
      case 'List':
        return `.list(${str(v.items_key)}, ${str(v.as)}, ${content(v.content)})`;
      case 'Card':
        return `.card(${content(v.content)}, ${lit(v.layout, depth + 1)})`;
      case 'Progress':
        return exactly(v, ['progress_percentage_key', 'tone']) ? `.progress(${str(v.progress_percentage_key)})` : null;
      case 'Activity':
        return `.activity(${str(v.state)}, ${str(v.action)})`;
      case 'BattlefieldCanvas':
        return '.battlefieldCanvas()';
      case 'TracklinesCanvas':
        return '.tracklinesCanvas()';
      default:
        return null;
    }
  })();
  // A display the builders have no shorthand for is written as the JSON it is.
  if (call === null) return lit(d, depth);
  return `new ${im.use('WorkflowDisplayBuilder')}()\n${pad(depth + 1)}${call}\n${pad(depth + 1)}.build()`;
}

function visual(v: J, depth: number, im: Imports): string {
  const [kind, d] = variant(v.display);
  if (kind === 'Page' && exactly(d, ['width', 'content', 'center', 'backdrop', 'typeface'])) {
    const opts = options(
      [
        ['id', unless(v.id, 'page', str)],
        ['center', d.center ? 'true' : null],
        ['backdrop', unless(d.backdrop, 'Default', str)],
        ['typeface', unless(d.typeface, 'Default', str)],
      ],
      depth,
    );
    return helper(im, 'page', [str(d.width), list(d.content.map((x: J) => display(x, depth + 1, im)), depth)], opts);
  }
  const content = (items: J[]) => list(items.map((x) => display(x, depth + 2, im)), depth + 1);
  const call = (() => {
    switch (kind) {
      case 'Block':
        return Array.isArray(d.content) && Object.keys(d).every((k) => k === 'content' || k === 'class')
          ? `.block(${content(d.content)})`
          : `.display(${lit(v.display, depth + 1)})`;
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
