import type {
  ContextMenuOptions,
  ContextMenuSection,
  ContextMenuSource,
} from "./types.js";

const registered = new WeakMap<Element, ContextMenuOptions>();

function options(
  source: ContextMenuSource | ContextMenuOptions,
): ContextMenuOptions {
  return typeof source === "function" ? { items: source } : source;
}

/** Gives an element its own right-click menu items. */
export function contextMenu(
  node: Element,
  source: ContextMenuSource | ContextMenuOptions,
) {
  registered.set(node, options(source));
  return {
    update(next: ContextMenuSource | ContextMenuOptions) {
      registered.set(node, options(next));
    },
    destroy() {
      registered.delete(node);
    },
  };
}

/** The closest element, from `target` up, with its own menu items. */
export function contextMenuOwner(target: Element | null) {
  for (let element = target; element; element = element.parentElement) {
    const found = registered.get(element);
    if (found) return { element, ...found };
  }
  return null;
}

export function toSections(
  list: ContextMenuSection[] | ContextMenuSection | null | undefined,
): ContextMenuSection[] {
  if (!list?.length) return [];
  const sections = (
    Array.isArray(list[0]) ? list : [list]
  ) as ContextMenuSection[];
  return sections.filter((section) => section.length > 0);
}
