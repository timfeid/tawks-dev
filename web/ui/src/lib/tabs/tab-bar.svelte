<script lang="ts" module>
  import type { Component } from 'svelte';

  export type TabBarItem = {
    value: string;
    label: string;
    href?: string;
    icon?: Component<{ class?: string }>;
    count?: number;
    countLabel?: string;
  };
</script>

<script lang="ts">
  import { tabsLook } from './look.js';
  import TabsCount from './tabs-count.svelte';

  /** A row of tabs that switch a page's view, by link or by callback. */
  let {
    items,
    value,
    label,
    onchange,
  }: {
    items: TabBarItem[];
    value: string;
    label: string;
    onchange?: (value: string) => void;
  } = $props();

  let bar = $state<HTMLElement | null>(null);

  // The bar scrolls sideways when it is wider than its row; the current tab stays in view.
  $effect(() => {
    void value;
    const on = bar?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (!bar || !on) return;
    const left = on.offsetLeft - bar.offsetLeft;
    if (left < bar.scrollLeft) bar.scrollLeft = left;
    else if (left + on.offsetWidth > bar.scrollLeft + bar.clientWidth) {
      bar.scrollLeft = left + on.offsetWidth - bar.clientWidth;
    }
  });

  function onkeydown(event: KeyboardEvent) {
    const tabs = [...(event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[role="tab"]')];
    const at = tabs.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    const last = tabs.length - 1;
    const next =
      event.key === 'ArrowRight' ? (at === last ? 0 : at + 1)
      : event.key === 'ArrowLeft' ? (at === 0 ? last : at - 1)
      : event.key === 'Home' ? 0
      : event.key === 'End' ? last
      : -1;
    if (next < 0) return;
    event.preventDefault();
    tabs[next].focus();
    tabs[next].click();
  }
</script>

<div bind:this={bar} class={tabsLook.bar} role="tablist" aria-label={label} aria-orientation="horizontal" tabindex="-1" {onkeydown}>
  {#each items as item (item.value)}
    {@const on = item.value === value}
    {#if item.href}
      <a
        href={item.href}
        role="tab"
        aria-selected={on}
        aria-current={on ? 'page' : undefined}
        tabindex={on ? 0 : -1}
        class={[tabsLook.tab, on ? tabsLook.on : tabsLook.off]}
        onclick={() => onchange?.(item.value)}
      >
        {#if item.icon}<item.icon />{/if}{item.label}{#if item.count !== undefined}<TabsCount value={item.count} label={item.countLabel} />{/if}
      </a>
    {:else}
      <button
        type="button"
        role="tab"
        aria-selected={on}
        tabindex={on ? 0 : -1}
        class={[tabsLook.tab, on ? tabsLook.on : tabsLook.off]}
        onclick={() => onchange?.(item.value)}
      >
        {#if item.icon}<item.icon />{/if}{item.label}{#if item.count !== undefined}<TabsCount value={item.count} label={item.countLabel} />{/if}
      </button>
    {/if}
  {/each}
</div>
