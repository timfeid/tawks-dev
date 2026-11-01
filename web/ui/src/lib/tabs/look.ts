/**
 * The one horizontal tab bar: a glass pill whose current tab is solid brand
 * color. The Svelte tabs here and the workflow renderer's tabs both draw
 * from these, so a bot's page and an app page look the same. The bar
 * scrolls sideways when it is wider than its row.
 */
export const tabsLook = {
  bar: 'inline-flex max-w-full min-w-0 shrink-0 items-center gap-1 overflow-x-auto rounded-full border border-foreground/10 bg-foreground/[0.03] p-1 backdrop-blur [scrollbar-width:none]',
  tab: 'inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-full px-3 text-xs font-semibold whitespace-nowrap transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:size-3.5 [&_svg]:shrink-0',
  on: 'bg-primary text-primary-foreground',
  off: 'text-muted-foreground hover:text-foreground',
  count: 'min-w-4 rounded-full bg-current/15 px-1 text-center text-[10px] leading-4 tabular-nums',
};

/** The same tab drawn by bits-ui, which marks the current one with data-state. */
export const triggerLook = `${tabsLook.tab} text-muted-foreground hover:text-foreground data-[state=active]:bg-primary data-[state=active]:text-primary-foreground`;
