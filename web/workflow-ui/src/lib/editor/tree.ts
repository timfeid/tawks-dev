export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

export type Path = (string | number)[];

export type TreeItem = {
  path: Path;
  kind: 'node' | 'visual' | 'display';
  label: string;
  detail: string;
  /** Where new displays go when this item is selected. */
  container: Path | null;
  children: TreeItem[];
};

type Obj = { [key: string]: Json };

const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v);
const isDisplay = (v: unknown): v is Obj => isObj(v) && 'type' in v && Object.keys(v).every((key) => ['type', 'mark', 'class'].includes(key));

export function getAt(root: Json, path: Path): Json | undefined {
  let cur: Json | undefined = root;
  for (const key of path) {
    if (Array.isArray(cur) && typeof key === 'number') cur = cur[key];
    else if (isObj(cur) && typeof key === 'string') cur = cur[key];
    else return undefined;
  }
  return cur;
}

export function setAt(root: Json, path: Path, value: Json): void {
  const parent = getAt(root, path.slice(0, -1));
  const key = path[path.length - 1];
  if (Array.isArray(parent) && typeof key === 'number') parent[key] = value;
  else if (isObj(parent) && typeof key === 'string') parent[key] = value;
}

/** The display arrays inside one display, such as a Div's or a Card's content. */
function displayArrays(display: Obj, path: Path): { path: Path; items: Obj[] }[] {
  const type = display.type;
  if (!isObj(type)) return [];
  const [variant, body] = Object.entries(type)[0] ?? [];
  if (!isObj(body)) return [];
  return Object.entries(body).flatMap(([field, value]) =>
    Array.isArray(value) && value.length >= 0 && (value.length === 0 ? ['content', 'view_display', 'row', 'actions', 'aside', 'lead', 'trail', 'media', 'caption', 'badge'].includes(field) : value.every(isDisplay))
      ? [{ path: [...path, 'type', variant, field], items: value as Obj[] }]
      : [],
  );
}

function describe(display: Obj): { label: string; detail: string } {
  const type = display.type;
  if (typeof type === 'string') return { label: type, detail: '' };
  if (!isObj(type)) return { label: 'Display', detail: '' };
  const [variant, body] = Object.entries(type)[0] ?? ['Display', null];
  const b = isObj(body) ? body : {};
  const detail =
    (typeof b.text === 'string' && b.text) ||
    (typeof b.action_id === 'string' && `runs ${b.action_id}`) ||
    (typeof b.id === 'string' && b.id) ||
    (typeof b.href === 'string' && b.href) ||
    (typeof b.items_key === 'string' && `each ${b.items_key}`) ||
    '';
  return { label: variant, detail: detail.length > 40 ? `${detail.slice(0, 40)}...` : detail };
}

function displayItem(display: Obj, path: Path): TreeItem {
  const arrays = displayArrays(display, path);
  return {
    path,
    kind: 'display',
    ...describe(display),
    container: arrays[0]?.path ?? null,
    children: arrays.flatMap((a) => a.items.map((child, i) => displayItem(child, [...a.path, i]))),
  };
}

function visualContainer(visual: Obj, path: Path): Path | null {
  const display = visual.display;
  if (!isObj(display)) return null;
  const [variant, body] = Object.entries(display)[0] ?? [];
  if (isObj(body) && Array.isArray(body.content)) return [...path, 'display', variant, 'content'];
  return null;
}

export function tree(def: Json): TreeItem[] {
  const nodes = getAt(def, ['nodes']);
  if (!isObj(nodes)) return [];
  return Object.keys(nodes)
    .sort()
    .map((nodeId) => {
      const node = nodes[nodeId] as Obj;
      const visuals = Array.isArray(node.visuals) ? (node.visuals as Obj[]) : [];
      return {
        path: ['nodes', nodeId],
        kind: 'node' as const,
        label: nodeId,
        detail: typeof node.title === 'string' ? node.title : '',
        container: null,
        children: visuals.map((visual, vi) => {
          const vpath: Path = ['nodes', nodeId, 'visuals', vi];
          const container = visualContainer(visual, vpath);
          const items = container ? ((getAt(def, container) as Obj[]) ?? []) : [];
          const variant = isObj(visual.display) ? Object.keys(visual.display)[0] : 'Visual';
          return {
            path: vpath,
            kind: 'visual' as const,
            label: variant,
            detail: typeof visual.id === 'string' ? visual.id : '',
            container,
            children: items.map((d, i) => displayItem(d, [...(container as Path), i])),
          };
        }),
      };
    });
}

/** Where something new goes: inside the selection if it holds displays, else after it. */
export function insertTarget(def: Json, selected: Path | null, items: TreeItem[]): { array: Path; index: number } | null {
  const find = (list: TreeItem[]): TreeItem | null => {
    for (const it of list) {
      if (selected && JSON.stringify(it.path) === JSON.stringify(selected)) return it;
      const inner = find(it.children);
      if (inner) return inner;
    }
    return null;
  };
  const item = selected ? find(items) : null;
  if (item?.container) {
    const arr = getAt(def, item.container);
    return { array: item.container, index: Array.isArray(arr) ? arr.length : 0 };
  }
  if (item?.kind === 'display') {
    const index = item.path[item.path.length - 1] as number;
    return { array: item.path.slice(0, -1), index: index + 1 };
  }
  const firstVisual = items[0]?.children.find((v) => v.container);
  if (firstVisual?.container) {
    const arr = getAt(def, firstVisual.container);
    return { array: firstVisual.container, index: Array.isArray(arr) ? arr.length : 0 };
  }
  return null;
}

// ---- Editing a page by hand ------------------------------------------------

export const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

export const samePath = (a: Path | null | undefined, b: Path | null | undefined) =>
  !!a && !!b && a.length === b.length && a.every((key, i) => key === b[i]);

/** True when `path` lies inside `ancestor`. */
export const isInside = (path: Path, ancestor: Path) =>
  path.length > ancestor.length && ancestor.every((key, i) => path[i] === key);

export const pathKey = (path: Path) => path.join('/');

export const parsePathKey = (key: string): Path => key.split('/').map((part) => (/^\d+$/.test(part) ? Number(part) : part));

/** Blocks that arrange other blocks the builder can move by hand. */
const HOLDERS = new Set(['Section', 'Stack', 'Grid', 'Panel', 'Card', 'Condition', 'Div', 'Split']);

/** Keys that tie a block to data, actions or other pages. */
const BINDING_KEYS = new Set(['items_key', 'data_key', 'chart_data_key', 'server_action_id', 'act', 'action', 'on_change_action', 'href', 'src', 'condition']);

/** The words on a block a builder can change by hand; the server holds to the same list. */
export const TEXT_FIELDS = ['eyebrow', 'title', 'text', 'lead', 'body', 'label', 'hint', 'meta', 'caption', 'description', 'placeholder', 'alt', 'value'] as const;
export type TextField = (typeof TEXT_FIELDS)[number];

const FIELD_WORDS: Record<TextField, string> = {
  eyebrow: 'Eyebrow',
  title: 'Title',
  text: 'Text',
  lead: 'Lead',
  body: 'Body',
  label: 'Label',
  hint: 'Hint',
  meta: 'Details',
  caption: 'Caption',
  description: 'Description',
  placeholder: 'Placeholder',
  alt: 'Picture description',
  value: 'Value',
};

/** Fields that read as a paragraph rather than a line. */
const LONG_FIELDS = new Set<string>(['lead', 'body', 'text', 'description']);
/** Fields that may be left out, so clearing them takes them off the block. */
const OPTIONAL_FIELDS = new Set<string>(['eyebrow', 'lead', 'meta', 'caption', 'description', 'hint']);

const KIND_WORDS: Record<string, string> = {
  Section: 'Section',
  Hero: 'Header',
  Typography: 'Text',
  Text: 'Text',
  Stack: 'Group',
  Div: 'Group',
  Grid: 'Grid',
  Panel: 'Panel',
  Card: 'Card',
  Split: 'Split',
  Condition: 'Shown when',
  Table: 'Table',
  Items: 'List',
  List: 'List',
  Chart: 'Chart',
  Notice: 'Notice',
  Empty: 'Empty message',
  Divider: 'Divider',
  Stat: 'Stat',
  Meter: 'Meter',
  Feature: 'Feature',
  Chip: 'Chip',
  Badge: 'Badge',
  Row: 'Row',
  Media: 'Picture',
  Image: 'Picture',
  Link: 'Link',
  Icon: 'Icon',
  Action: 'Button',
  Form: 'Form',
  Input: 'Input',
  Choice: 'Choice',
  Dialog: 'Pop-up',
  TabBar: 'Tabs',
  Segments: 'Segments',
};

const TEMPLATE = /\{[^{}]*\}/;

export type PageBlockOf = { kind: string; body: Obj };

/** A block's kind, such as Section, and the fields it carries. */
export function blockOf(display: Json | undefined): PageBlockOf | null {
  if (!isObj(display)) return null;
  const type = display.type;
  if (typeof type === 'string') return { kind: type, body: {} };
  if (!isObj(type)) return null;
  const [kind, body] = Object.entries(type)[0] ?? [];
  if (!kind) return null;
  return { kind, body: isObj(body) ? body : {} };
}

export function kindWords(block: PageBlockOf): string {
  if (block.kind === 'Typography') {
    const role = block.body.role;
    if (role === 'Heading' || role === 'Title' || role === 'Subheading' || role === 'Display' || role === 'Headline') return 'Heading';
    if (role === 'Fine') return 'Small print';
  }
  return KIND_WORDS[block.kind] ?? block.kind;
}

function plainWords(text: string): string {
  return text.replace(/\{[^{}]*\}/g, ' ').replace(/\s+/g, ' ').trim();
}

/** Whether the block's own settings read data, run something or link somewhere. */
export function isBound(display: Json | undefined): boolean {
  const block = blockOf(display);
  if (!block) return false;
  if (['Table', 'Items', 'List', 'Chart', 'Segments', 'Form', 'Input', 'Choice', 'Calendar', 'Action'].includes(block.kind)) return true;
  const holds = HOLDERS.has(block.kind) ? 'content' : null;
  const walk = (value: Json, key: string): boolean => {
    if (BINDING_KEYS.has(key) && value !== null && value !== '') return true;
    if (typeof value === 'string') return TEMPLATE.test(value);
    if (Array.isArray(value)) return value.some((item) => walk(item, key));
    if (isObj(value)) return Object.entries(value).some(([k, v]) => walk(v, k));
    return false;
  };
  return Object.entries(block.body).some(([key, value]) => key !== holds && walk(value, key));
}

export type EditableField = { field: TextField; label: string; value: string; long: boolean; optional: boolean };

/** The words on a block that change by hand: plain text, never a template. */
export function editableFields(display: Json | undefined): EditableField[] {
  const block = blockOf(display);
  if (!block) return [];
  return TEXT_FIELDS.flatMap((field) => {
    if (!(field in block.body)) return [];
    const value = block.body[field];
    if (value !== null && typeof value !== 'string') return [];
    if (typeof value === 'string' && TEMPLATE.test(value)) return [];
    return [{ field, label: FIELD_WORDS[field], value: value ?? '', long: LONG_FIELDS.has(field), optional: value === null || OPTIONAL_FIELDS.has(field) }];
  });
}

/** What a block is called in the outline and in the card's history. */
export function blockName(display: Json | undefined): { label: string; kind: string } {
  const block = blockOf(display);
  if (!block) return { label: 'Block', kind: 'Block' };
  const kind = kindWords(block);
  for (const field of ['title', 'text', 'label', 'body', 'lead', 'value'] as const) {
    const value = block.body[field];
    if (typeof value !== 'string') continue;
    const words = plainWords(value);
    if (words) return { label: words.length > 48 ? `${words.slice(0, 47)}...` : words, kind };
  }
  if (typeof block.body.items_key === 'string') {
    const name = block.body.items_key.replace(/^(sys|data)\./, '').replace(/\.rows$/, '').replace(/[._]+/g, ' ').trim();
    if (name) return { label: `${kind} of ${name}`, kind };
  }
  return { label: kind, kind };
}

export type PageItem = {
  path: Path;
  key: string;
  kind: string;
  label: string;
  /** The array of blocks this one arranges, when it arranges any. */
  holds: Path | null;
  bound: boolean;
  /** A page's frame, which can hold blocks but not move. */
  fixed: boolean;
  children: PageItem[];
};

function holderArray(display: Obj, path: Path): Path | null {
  const block = blockOf(display);
  if (!block || !HOLDERS.has(block.kind) || !Array.isArray(block.body.content)) return null;
  return [...path, 'type', block.kind, 'content'];
}

function pageItem(display: Obj, path: Path): PageItem {
  const holds = holderArray(display, path);
  const name = blockName(display);
  const inner = holds ? ((getAt(display, holds.slice(path.length)) as Obj[] | undefined) ?? []) : [];
  return {
    path,
    key: pathKey(path),
    kind: name.kind,
    label: name.label,
    holds,
    bound: isBound(display),
    fixed: false,
    children: inner.map((child, i) => pageItem(child, [...holds!, i])),
  };
}

/** Each visual of each step that lays out blocks: a Page or a Block. */
function frames(def: Json): { path: Path; holds: Path; title: string }[] {
  const nodes = getAt(def, ['nodes']);
  if (!isObj(nodes)) return [];
  const ids = Object.keys(nodes).sort((a, b) => (a === 'main' ? -1 : b === 'main' ? 1 : a.localeCompare(b)));
  return ids.flatMap((id) => {
    const node = nodes[id];
    const visuals = isObj(node) && Array.isArray(node.visuals) ? node.visuals : [];
    return visuals.flatMap((visual, vi) => {
      if (!isObj(visual) || !isObj(visual.display)) return [];
      const [variant, body] = Object.entries(visual.display)[0] ?? [];
      if (!isObj(body) || !Array.isArray(body.content)) return [];
      const title = (isObj(node) && typeof node.title === 'string' && node.title) || id;
      return [{ path: ['nodes', id, 'visuals', vi], holds: ['nodes', id, 'visuals', vi, 'display', variant, 'content'], title }];
    });
  });
}

/** The outline of a page as a builder arranges it. A page with one frame lists its blocks directly. */
export function pageTree(def: Json): PageItem[] {
  const found = frames(def);
  const blocks = (frame: (typeof found)[number]) =>
    ((getAt(def, frame.holds) as Obj[] | undefined) ?? []).map((display, i) => pageItem(display, [...frame.holds, i]));
  if (found.length === 1) return blocks(found[0]);
  return found.map((frame) => ({
    path: frame.path,
    key: pathKey(frame.path),
    kind: 'Page',
    label: frame.title,
    holds: frame.holds,
    bound: false,
    fixed: true,
    children: blocks(frame),
  }));
}

export function flatten(items: PageItem[]): PageItem[] {
  return items.flatMap((item) => [item, ...flatten(item.children)]);
}

/** The array a top-level block goes in when nothing says where. */
export function firstFrame(def: Json): Path | null {
  return frames(def)[0]?.holds ?? null;
}

export function setField(def: Json, path: Path, field: string, value: Json): Json {
  const next = clone(def);
  const block = blockOf(getAt(next, path));
  if (block) block.body[field] = value;
  return next;
}

export function removeAt(def: Json, path: Path): Json {
  const next = clone(def);
  const array = getAt(next, path.slice(0, -1));
  const index = path[path.length - 1];
  if (Array.isArray(array) && typeof index === 'number') array.splice(index, 1);
  return next;
}

export function insertAt(def: Json, array: Path, index: number, value: Json): { def: Json; path: Path } | null {
  const next = clone(def);
  const target = getAt(next, array);
  if (!Array.isArray(target)) return null;
  const at = Math.max(0, Math.min(index, target.length));
  target.splice(at, 0, clone(value));
  return { def: next, path: [...array, at] };
}

export function duplicateAt(def: Json, path: Path): { def: Json; path: Path } | null {
  const value = getAt(def, path);
  const index = path[path.length - 1];
  if (value === undefined || typeof index !== 'number') return null;
  return insertAt(def, path.slice(0, -1), index + 1, value);
}

/** Moves the block at `from` to `index` of `array`, as the array reads before the move. */
export function moveTo(def: Json, from: Path, array: Path, index: number): { def: Json; path: Path } | null {
  const fromArray = from.slice(0, -1);
  const fromIndex = from[from.length - 1];
  if (typeof fromIndex !== 'number') return null;
  if (samePath(array, from) || isInside(array, from)) return null;
  let target = [...array];
  let at = index;
  if (samePath(target, fromArray)) {
    if (at > fromIndex) at -= 1;
    if (at === fromIndex) return null;
  } else if (isInside(target, fromArray)) {
    const step = target[fromArray.length];
    if (typeof step === 'number' && step > fromIndex) target[fromArray.length] = step - 1;
  }
  const value = getAt(def, from);
  if (value === undefined) return null;
  return insertAt(removeAt(def, from), target, at, value);
}

/** The block a holder array belongs to, when it belongs to one. */
export function ownerOf(array: Path): Path | null {
  const n = array.length;
  if (n >= 4 && array[n - 3] === 'type' && typeof array[n - 4] === 'number') return array.slice(0, -3);
  return null;
}

/** One step up or down among its neighbours, or out past the block holding it at either end. */
export function moveBy(def: Json, path: Path, delta: -1 | 1): { def: Json; path: Path } | null {
  const array = path.slice(0, -1);
  const index = path[path.length - 1];
  const items = getAt(def, array);
  if (typeof index !== 'number' || !Array.isArray(items)) return null;
  const next = index + delta;
  if (next >= 0 && next < items.length) return moveTo(def, path, array, delta > 0 ? next + 1 : next);
  const owner = ownerOf(array);
  if (!owner) return null;
  const ownerIndex = owner[owner.length - 1] as number;
  return moveTo(def, path, owner.slice(0, -1), delta > 0 ? ownerIndex + 1 : ownerIndex);
}

function nameAt(def: Json, path: Path): string {
  const { label, kind } = blockName(getAt(def, path));
  return label === kind ? `the ${kind.toLowerCase()}` : label;
}

/** Where a moved block landed, in words: "below NFC West", "into Playoff picture". */
export function placeWords(def: Json, path: Path): string {
  const array = path.slice(0, -1);
  const index = path[path.length - 1] as number;
  const items = getAt(def, array);
  const length = Array.isArray(items) ? items.length : 0;
  const owner = ownerOf(array);
  if (index > 0) return `below ${nameAt(def, [...array, index - 1])}`;
  if (owner && length === 1) return `into ${nameAt(def, owner)}`;
  if (index + 1 < length) return `above ${nameAt(def, [...array, index + 1])}`;
  return 'to the top of the page';
}

export function moveSummary(after: Json, path: Path): string {
  const array = path.slice(0, -1);
  const index = path[path.length - 1] as number;
  const items = getAt(after, array);
  const last = Array.isArray(items) && index === items.length - 1 && !ownerOf(array);
  const where = last && index > 0 ? 'to the bottom of the page' : placeWords(after, path);
  return `Moved ${nameAt(after, path)} ${where}.`;
}

export function blockLabel(def: Json, path: Path): string {
  return nameAt(def, path);
}

/** Every block of the page by the object it is, for the preview to know what it drew. */
export function blockKeys(def: Json): Map<object, string> {
  const keys = new Map<object, string>();
  for (const item of flatten(pageTree(def))) {
    const value = getAt(def, item.path);
    if (value && typeof value === 'object' && !item.fixed) keys.set(value, item.key);
  }
  return keys;
}
