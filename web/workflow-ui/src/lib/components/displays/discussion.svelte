<script lang="ts">
  import { getNestedValue } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';
  import { LUCIDE_REGISTRY } from './icon/registry.js';

  type Reaction = { emoji: string; count: number };

  // Hosts with chat draw this themselves; elsewhere it shows the counts the page knew.
  let { display, inputs }: Props = $props();
  let spec = $derived(typeof display.type !== 'string' && 'Discussion' in display.type ? display.type.Discussion : null);
  let reactions = $derived.by(() => {
    const list = spec ? getNestedValue(inputs, spec.reactions) : null;
    return Array.isArray(list) ? (list as Reaction[]).filter((r) => r?.emoji && r.count > 0) : [];
  });
  let comments = $derived(Number((spec && getNestedValue(inputs, spec.comments)) ?? 0) || 0);
  const Comments = LUCIDE_REGISTRY['message-square'];
</script>

{#if spec}
  <div class="flex flex-wrap items-center gap-1.5">
    {#each reactions as reaction (reaction.emoji)}
      <span class={look.chip}>
        <span>{reaction.emoji}</span>
        <span class="tabular-nums">{reaction.count}</span>
      </span>
    {/each}
    <span class={look.chip}>
      {#if Comments}<Comments class="size-4 shrink-0" />{/if}
      <span class="tabular-nums">{comments} {comments === 1 ? 'comment' : 'comments'}</span>
    </span>
  </div>
{/if}
