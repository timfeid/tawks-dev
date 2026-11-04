import { getContext, setContext } from "svelte";

const KEY = Symbol("workflow-editing");

/** Given while a page is edited by hand: which block of the page each drawn display is. */
export type PageEditing = { keyOf(display: object): string | null };

export const provideEditing = (editing: PageEditing) =>
  setContext(KEY, editing);

export const useEditing = () => getContext<PageEditing | undefined>(KEY);
