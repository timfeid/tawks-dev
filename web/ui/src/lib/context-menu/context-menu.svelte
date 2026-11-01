<script lang="ts">
  import { DropdownMenu } from "bits-ui";
  import { Check, ChevronRight } from "@lucide/svelte";
  import { cn } from "../utils.js";
  import type { ContextMenuItem, ContextMenuSection } from "./types.js";

  /** A menu at a point on screen, such as where the pointer was right-clicked. */
  let {
    at,
    sections,
    label = "Options",
    portal,
    restoreFocus,
    onclose,
  }: {
    at: { x: number; y: number };
    sections: ContextMenuSection[];
    label?: string;
    portal?: Element | null;
    /** Gets focus back when the menu closes, before the chosen item runs. */
    restoreFocus?: HTMLElement | null;
    onclose: () => void;
  } = $props();

  const anchor = $derived({
    getBoundingClientRect: () => new DOMRect(at.x, at.y, 0, 0),
  });
  let chosen: ContextMenuItem | null = null;

  function finish() {
    if (restoreFocus?.isConnected) restoreFocus.focus({ preventScroll: true });
    const item = chosen;
    chosen = null;
    onclose();
    // After the menu is gone, so an item that opens a popover or focuses a
    // field is not undone by the menu's own close.
    if (item?.run)
      setTimeout(() => Promise.resolve(item.run!()).catch(console.error));
  }

  const itemClass =
    "flex w-full cursor-default items-center gap-2 rounded-md px-2 py-2 text-left text-sm outline-none select-none transition-colors data-[highlighted]:bg-white/10 data-[disabled]:pointer-events-none data-[disabled]:opacity-50";
  const tone = (item: ContextMenuItem) =>
    item.danger
      ? "text-red-400 data-[highlighted]:text-red-300"
      : "text-foreground/80 data-[highlighted]:text-foreground data-[state=open]:bg-white/10";
  const contentClass =
    "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 z-50 w-56 rounded-md border p-1 shadow-md outline-hidden";
</script>

{#snippet face(item: ContextMenuItem)}
  {#if item.icon}
    <item.icon class="size-4 shrink-0 opacity-70" />
  {:else}
    <span class="size-4 shrink-0" aria-hidden="true"></span>
  {/if}
  <span class="min-w-0 flex-1 truncate">{item.label}</span>
  {#if item.checked}<Check
      class="size-4 shrink-0 opacity-70"
      aria-label="On"
    />{/if}
  {#if item.shortcut}
    <span class="text-muted-foreground shrink-0 text-xs tracking-wide"
      >{item.shortcut}</span
    >
  {/if}
{/snippet}

{#snippet list(groups: ContextMenuSection[])}
  {#each groups as section, s (s)}
    {#if s > 0}<DropdownMenu.Separator class="bg-border -mx-1 my-1 h-px" />{/if}
    {#each section as item, i (`${s}:${i}:${item.label}`)}
      {#if item.items?.length}
        <DropdownMenu.Sub>
          <DropdownMenu.SubTrigger
            class={cn(itemClass, tone(item))}
            disabled={item.disabled}
          >
            {@render face(item)}
            <ChevronRight class="size-4 shrink-0 opacity-50" />
          </DropdownMenu.SubTrigger>
          <DropdownMenu.Portal to={portal ?? undefined}>
            <DropdownMenu.SubContent
              class={contentClass}
              sideOffset={6}
              collisionPadding={8}
            >
              {@render list([item.items])}
            </DropdownMenu.SubContent>
          </DropdownMenu.Portal>
        </DropdownMenu.Sub>
      {:else}
        <DropdownMenu.Item
          class={cn(itemClass, tone(item))}
          disabled={item.disabled}
          onSelect={() => (chosen = item)}
        >
          {@render face(item)}
        </DropdownMenu.Item>
      {/if}
    {/each}
  {/each}
{/snippet}

<DropdownMenu.Root open onOpenChange={(open) => !open && finish()}>
  <DropdownMenu.Portal to={portal ?? undefined}>
    <DropdownMenu.Content
      customAnchor={anchor}
      side="bottom"
      align="start"
      sideOffset={4}
      collisionPadding={8}
      preventScroll={false}
      loop
      onCloseAutoFocus={(event) => event.preventDefault()}
      class={contentClass}
      aria-label={label}
      data-app-context-menu
    >
      {@render list(sections)}
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>
