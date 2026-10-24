import type { Json } from './runtime';

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
const isDisplay = (v: unknown): v is Obj => isObj(v) && 'type' in v && 'class' in v;

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
    Array.isArray(value) && value.length >= 0 && (value.length === 0 ? ['content', 'view_display', 'row'].includes(field) : value.every(isDisplay))
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
