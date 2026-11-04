<script lang="ts" module>
  import type { Path } from './tree';

  export type OutlineItem = {
    path: Path;
    label: string;
    detail?: string;
    kind?: string;
    /** The array of blocks this item holds, where a drop inside it lands. */
    holds?: Path | null;
    container?: Path | null;
    /** Holds blocks but never moves itself. */
    fixed?: boolean;
    children: OutlineItem[];
  };

  export type OutlineDrop = { array: Path; index: number; at: Path; place: 'before' | 'after' | 'inside' };

  /** Shared by every level of one outline, so a drag can cross levels. */
  export class OutlineDrag {
    from = $state<Path | null>(null);
    over = $state<OutlineDrop | null>(null);
  }
</script>

<script lang="ts">
  import Outline from './outline.svelte';
  import { isInside, samePath } from './tree';

  let {
    items,
    selected,
    onSelect,
    onDrop,
    disabled = false,
    depth = 0,
    drag = new OutlineDrag(),
  }: {
    items: OutlineItem[];
    selected: Path | null;
    onSelect: (path: Path) => void;
    /** Given, rows can be dragged before, after or into each other. */
    onDrop?: (from: Path, array: Path, index: number) => void;
    disabled?: boolean;
    depth?: number;
    drag?: OutlineDrag;
  } = $props();

  const holdsOf = (item: OutlineItem) => item.holds ?? item.container ?? null;
  const movable = (item: OutlineItem) => !!onDrop && !disabled && !item.fixed && typeof item.path.at(-1) === 'number';

  function placeFor(item: OutlineItem, event: DragEvent): OutlineDrop | null {
    const from = drag.from;
    if (!from) return null;
    const row = event.currentTarget as HTMLElement;
    const rect = row.getBoundingClientRect();
    const ratio = (event.clientY - rect.top) / Math.max(1, rect.height);
    const holds = holdsOf(item);
    const index = item.path.at(-1);
    let drop: OutlineDrop | null = null;
    if (holds && (item.fixed || (ratio > 0.3 && ratio < 0.7) || typeof index !== 'number')) {
      drop = { array: holds, index: item.children.length, at: item.path, place: 'inside' };
    } else if (typeof index === 'number') {
      const before = ratio < 0.5;
      drop = { array: item.path.slice(0, -1), index: before ? index : index + 1, at: item.path, place: before ? 'before' : 'after' };
    }
    if (!drop) return null;
    if (samePath(item.path, from) || isInside(item.path, from) || isInside(drop.array, from)) return null;
    return drop;
  }

  function over(item: OutlineItem, event: DragEvent) {
    if (!drag.from) return;
    const drop = placeFor(item, event);
    drag.over = drop;
    if (drop) {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
    }
  }

  function dropped(event: DragEvent) {
    event.preventDefault();
    const from = drag.from;
    const target = drag.over;
    drag.from = null;
    drag.over = null;
    if (from && target) onDrop?.(from, target.array, target.index);
  }

  const marked = (item: OutlineItem, place: OutlineDrop['place']) =>
    !!drag.over && drag.over.place === place && samePath(drag.over.at, item.path);
</script>

<ul class="flex flex-col" role={depth === 0 ? 'tree' : 'group'}>
  {#each items as item (JSON.stringify(item.path))}
    <li class="relative" role="treeitem" aria-selected={samePath(selected, item.path)}>
      {#if marked(item, 'before')}
        <div class="bg-primary pointer-events-none absolute inset-x-0 -top-px z-10 h-0.5 rounded-full" data-drop-line></div>
      {/if}
      <button
        type="button"
        class={[
          'flex w-full items-baseline gap-2 rounded px-2 py-1 text-left text-sm',
          samePath(selected, item.path) ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:bg-secondary/50 hover:text-foreground',
          marked(item, 'inside') && 'ring-primary ring-2 ring-inset',
          samePath(drag.from, item.path) && 'opacity-50',
        ]}
        style="padding-left: {0.5 + depth * 0.9}rem"
        data-outline-path={item.path.join('/')}
        draggable={movable(item)}
        aria-keyshortcuts={movable(item) ? 'Alt+ArrowUp Alt+ArrowDown' : undefined}
        onclick={() => onSelect(item.path)}
        ondragstart={(event) => {
          if (!movable(item)) return;
          drag.from = item.path;
          event.dataTransfer?.setData('text/plain', item.label);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
        }}
        ondragend={() => {
          drag.from = null;
          drag.over = null;
        }}
        ondragover={(event) => over(item, event)}
        ondrop={dropped}
      >
        <span class={item.kind === 'node' || item.fixed ? 'text-foreground font-semibold' : item.kind === 'visual' ? 'text-primary' : 'truncate'}>{item.label}</span>
        {#if item.detail}<span class="shrink-0 truncate text-xs opacity-70">{item.detail}</span>{/if}
      </button>
      {#if item.children.length}
        <Outline items={item.children} {selected} {onSelect} {onDrop} {disabled} depth={depth + 1} {drag} />
      {/if}
      {#if marked(item, 'after')}
        <div class="bg-primary pointer-events-none absolute inset-x-0 -bottom-px z-10 h-0.5 rounded-full" data-drop-line></div>
      {/if}
    </li>
  {/each}
</ul>
