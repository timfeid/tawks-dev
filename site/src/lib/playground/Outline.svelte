<script lang="ts">
  import type { Path, TreeItem } from './tree';
  import Outline from './Outline.svelte';

  let {
    items,
    selected,
    onSelect,
    depth = 0,
  }: { items: TreeItem[]; selected: Path | null; onSelect: (path: Path) => void; depth?: number } = $props();

  const same = (a: Path | null, b: Path) => !!a && a.length === b.length && a.every((k, i) => k === b[i]);
</script>

<ul class="flex flex-col">
  {#each items as item (JSON.stringify(item.path))}
    <li>
      <button
        type="button"
        class="flex w-full items-baseline gap-2 rounded px-2 py-1 text-left text-sm {same(selected, item.path)
          ? 'bg-secondary text-foreground'
          : 'text-muted-foreground hover:bg-secondary/50 hover:text-foreground'}"
        style="padding-left: {0.5 + depth * 0.9}rem"
        onclick={() => onSelect(item.path)}
      >
        <span class="{item.kind === 'node' ? 'font-semibold text-foreground' : item.kind === 'visual' ? 'text-primary' : ''}">{item.label}</span>
        {#if item.detail}<span class="truncate text-xs opacity-70">{item.detail}</span>{/if}
      </button>
      {#if item.children.length}
        <Outline items={item.children} {selected} {onSelect} depth={depth + 1} />
      {/if}
    </li>
  {/each}
</ul>
