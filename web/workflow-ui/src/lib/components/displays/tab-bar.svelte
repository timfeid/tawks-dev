<script lang="ts">
  import { evaluateCondition, safeHref } from '../../utils';
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';
  import { getContext } from 'svelte';

  let { display, inputs, workflow }: Props = $props();
  let bar = $derived(typeof display.type !== 'string' && 'TabBar' in display.type ? display.type.TabBar : null);
  const docked = getContext<boolean | undefined>('dock-tabs');
  let style = $derived(docked ? look.dockedTabs : look.tabs[bar?.style ?? 'Underline']);
  const press = pressable(() => ({ workflow, inputs }));
  let active = $derived(bar ? bar.tabs.findIndex((tab) => evaluateCondition(tab.active, inputs)) : -1);

  // Arrow keys, Home and End move between tabs and open them, like every other Tawks tab list.
  function onkeydown(event: KeyboardEvent) {
    if (!bar) return;
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
    if (tabs[next] instanceof HTMLButtonElement) tabs[next].click();
  }
</script>

{#if bar}
  <nav class={style.bar} role="tablist" aria-orientation="horizontal" tabindex="-1" {onkeydown}>
    {#each bar.tabs as tab, index}
      {@const on = evaluateCondition(tab.active, inputs)}
      {@const Icon = tab.icon ? LUCIDE_REGISTRY[shown(inputs, tab.icon)] : undefined}
      {@const href = tab.href ? safeHref(shown(inputs, tab.href)) : null}
      {@const reachable = on || (active < 0 && index === 0)}
      {#if href}
        <a {href} role="tab" aria-selected={on} tabindex={reachable ? 0 : -1} class={[style.tab, on ? style.on : style.off]} aria-current={on ? 'page' : undefined}>
          {#if Icon}<Icon />{/if}
          {shown(inputs, tab.label)}
        </a>
      {:else}
        <button type="button" role="tab" aria-selected={on} tabindex={reachable ? 0 : -1} class={[style.tab, on ? style.on : style.off]} aria-current={on ? 'page' : undefined} onclick={() => press.runLatest(on ? null : tab.act)}>
          {#if Icon}<Icon />{/if}
          {shown(inputs, tab.label)}
        </button>
      {/if}
    {/each}
  </nav>
{/if}
