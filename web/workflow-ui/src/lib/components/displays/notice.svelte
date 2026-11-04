<script lang="ts">
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import WorkflowDisplay from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  let { display, inputs, workflow, ...rest }: Props = $props();
  let notice = $derived(typeof display.type !== 'string' && 'Notice' in display.type ? display.type.Notice : null);
  let tone = $derived(look.toneOf(notice?.tone, inputs));
  let Icon = $derived(notice?.icon ? LUCIDE_REGISTRY[shown(inputs, notice.icon)] : undefined);
  let body = $derived(notice ? shown(inputs, notice.body) : '');
</script>

{#if notice && (body || notice.title)}
  <div class={['flex items-start gap-3 rounded-xl border px-4 py-3', look.toneSoft(tone, 'Muted')]} role={tone === 'Danger' ? 'alert' : 'status'}>
    {#if Icon}<Icon class={['mt-0.5 size-4 shrink-0', look.toneText(tone, 'Muted')]} />{/if}
    <div class="flex min-w-0 flex-1 flex-col gap-0.5">
      {#if notice.title}<div class={look.text('Subheading', 'Default')}>{shown(inputs, notice.title)}</div>{/if}
      {#if body}<div class={look.text('Body', notice.title ? 'Default' : 'Strong')}>{body}</div>{/if}
    </div>
    {#if notice.actions.length}
      <div class="flex shrink-0 items-center gap-2 self-center">
        {#each notice.actions as child}<WorkflowDisplay {...rest} {inputs} {workflow} display={child} />{/each}
      </div>
    {/if}
  </div>
{/if}
