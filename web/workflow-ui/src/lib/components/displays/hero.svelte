<script lang="ts">
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let hero = $derived(typeof display.type !== 'string' && 'Hero' in display.type ? display.type.Hero : null);
</script>

{#if hero}
  <header class="flex min-w-0 flex-col gap-4 py-3">
    {#if hero.eyebrow}
      <div class={look.text('Eyebrow', 'Default')}>{interpolate(inputs, hero.eyebrow)}</div>
    {/if}
    <div class="flex flex-wrap items-start justify-between gap-4">
      <h1 class={['min-w-0 flex-1', look.text('Display', 'Default')]}>{interpolate(inputs, hero.title)}</h1>
      {#if hero.aside.length}
        <div class="flex shrink-0 flex-wrap items-center gap-3">
          {#each hero.aside as child}
            <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
          {/each}
        </div>
      {/if}
    </div>
    {#if hero.lead}
      <p class={look.text('Lead', 'Default')}>{interpolate(inputs, hero.lead)}</p>
    {/if}
    {#if hero.actions.length}
      <div class="flex flex-wrap items-center gap-3 pt-1">
        {#each hero.actions as child}
          <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
        {/each}
      </div>
    {/if}
  </header>
{/if}
