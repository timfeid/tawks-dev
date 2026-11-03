<script lang="ts">
  import { interpolate, safeHref } from '../../utils';
  import * as look from '../../look';
  import { pressable } from '../../act.svelte';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let row = $derived(typeof display.type !== 'string' && 'Row' in display.type ? display.type.Row : null);
  let state = $derived(look.stateOf(row?.state, inputs));
  let accent = $derived(look.accentOf(row?.accent, inputs));
  let tint = $derived(accent ? `background-color: ${look.accentFill(accent)}; box-shadow: ${look.accentEdge(accent)}` : undefined);
  let href = $derived(row?.href ? safeHref(interpolate(inputs, row.href)) : null);
  let external = $derived(!!href && /^https?:\/\//i.test(href));
  const press = pressable(() => ({ workflow, inputs }));
  let interactive = $derived(!!href || !!row?.act);
  let cls = $derived([
    'flex min-w-0 items-center gap-3 rounded-xl border px-3 py-2.5 text-left',
    look.rowState[state],
    interactive && 'transition-colors hover:border-foreground/25 hover:bg-foreground/[0.05]',
    press.pending && 'opacity-60',
  ]);
</script>

{#snippet inner()}
  {#if row!.lead.length}
    <div class="flex shrink-0 items-center gap-2">
      {#each row!.lead as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
    </div>
  {/if}
  <div class="flex min-w-0 flex-1 flex-col">
    <span class={['text-sm font-semibold [overflow-wrap:anywhere]', state === 'Muted' || state === 'Disabled' ? 'text-muted-foreground' : 'text-foreground']}>{interpolate(inputs, row!.title)}</span>
    {#if row!.meta}<span class="truncate text-xs text-muted-foreground">{interpolate(inputs, row!.meta)}</span>{/if}
  </div>
  {#if row!.trail.length}
    <!-- Buttons at the end do their own thing, not the row's. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="flex shrink-0 items-center gap-2" onclick={(event) => event.stopPropagation()}>
      {#each row!.trail as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
    </div>
  {/if}
{/snippet}

{#if row}
  {#if href}
    <a {href} class={cls} style={tint} target={external ? '_blank' : undefined} rel={external ? 'noopener noreferrer' : undefined}>{@render inner()}</a>
  {:else if row.act}
    {@const disabled = press.disabled(row.act)}
    <div
      role="button"
      tabindex={disabled ? -1 : 0}
      aria-disabled={disabled}
      class={[cls, 'w-full cursor-pointer', disabled && 'pointer-events-none']}
      style={tint}
      onclick={() => press.run(row.act)}
      onkeydown={(event) => {
        if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) {
          event.preventDefault();
          press.run(row.act);
        }
      }}
    >
      {@render inner()}
    </div>
  {:else}
    <div class={cls} style={tint}>{@render inner()}</div>
  {/if}
{/if}
