import type { Component } from "svelte";

export type ContextMenuItem = {
  label: string;
  icon?: Component<any>;
  /** A key combo to show beside the label, such as "Ctrl C". */
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Shows a check mark, for an option that is on. */
  checked?: boolean;
  run?: () => unknown;
  /** Opens as a submenu instead of running. */
  items?: ContextMenuItem[];
};

/** Items between separators. */
export type ContextMenuSection = ContextMenuItem[];

/** Called with the element the items belong to, each time the menu draws. */
export type ContextMenuSource = (
  element: Element,
) => ContextMenuSection[] | ContextMenuItem[] | null | undefined;

export type ContextMenuOptions = {
  items: ContextMenuSource;
  /** People tagged inside this element open this menu instead of their own,
   * as on a conversation tile showing its member's avatar. */
  whole?: boolean;
};
