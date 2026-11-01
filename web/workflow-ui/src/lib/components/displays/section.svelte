<script lang="ts">
  import { interpolate } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let section = $derived(typeof display.type !== 'string' && 'Section' in display.type ? display.type.Section : null);
</script>

{#if section}
  <section class="flex min-w-0 flex-col gap-5 border-t border-foreground/10 pt-6">
    <div class="flex flex-wrap items-end justify-between gap-x-4 gap-y-2">
      <div class="flex min-w-0 flex-col gap-1.5">
        {#if section.eyebrow}
          <div class={look.text('Eyebrow', 'Default')}>{interpolate(inputs, section.eyebrow)}</div>
        {/if}
        <h2 class={look.text('Heading', 'Default')}>{interpolate(inputs, section.title)}</h2>
        {#if section.lead}
          <p class={['max-w-2xl', look.text('Body', 'Default')]}>{interpolate(inputs, section.lead)}</p>
        {/if}
      </div>
      {#if section.aside.length}
        <div class="flex shrink-0 flex-wrap items-center gap-3">
          {#each section.aside as child}
            <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
          {/each}
        </div>
      {/if}
    </div>
    {#each section.content as child}
      <WorkflowDisplay {...rest} {inputs} {workflow} display={child} />
    {/each}
  </section>
{/if}
