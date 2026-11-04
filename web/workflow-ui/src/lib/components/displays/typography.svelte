<script lang="ts">
  import { toast } from 'svelte-sonner';
  import { Check, Copy } from '@lucide/svelte';
  import { shown } from '../../member-names.svelte';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let typography = $derived(typeof display.type !== 'string' && 'Typography' in display.type ? display.type.Typography : null);
  let value = $derived(typography ? shown(inputs, typography.text) : '');
  let tone = $derived(look.toneOf(typography?.tone, inputs));
  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      toast.error('Could not copy that');
    }
  }
</script>

{#if typography && value}
  {#if typography.role === 'Code'}
    <!-- Code is something to paste elsewhere, so it always comes with a copy button. -->
    <div class={['flex items-center gap-2', look.text('Code', tone, typography.style)]}>
      <span class="min-w-0 flex-1 break-all">{value}</span>
      <button
        type="button"
        class="text-muted-foreground hover:text-foreground -my-1 -mr-1.5 shrink-0 rounded-md p-1.5 transition-colors hover:bg-foreground/10"
        aria-label="Copy"
        title="Copy"
        onclick={copy}
      >
        {#if copied}<Check class="text-primary size-4" />{:else}<Copy class="size-4" />{/if}
      </button>
    </div>
  {:else}
    <div class={look.text(typography.role, tone, typography.style)}>{value}</div>
  {/if}
{/if}
