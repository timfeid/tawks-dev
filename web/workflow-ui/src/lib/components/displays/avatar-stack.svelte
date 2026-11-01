<script lang="ts">
  import { fallBackToDefaultAvatar } from '@tawks/ui/avatar/avatar-url.js';
  import { mediaUrl } from '../../media.js';
  import { getNestedValue, interpolate, pictureUrl } from '../../utils';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let stack = $derived(typeof display.type !== 'string' && 'AvatarStack' in display.type ? display.type.AvatarStack : null);
  let people = $derived.by(() => {
    if (!stack) return [];
    const list = getNestedValue(inputs, stack.items_key);
    if (!Array.isArray(list)) return [];
    return list.map((item) => {
      const scope = { ...inputs, [stack.as]: item };
      return { name: interpolate(scope, stack.name), src: pictureUrl(interpolate(scope, stack.avatar)) };
    });
  });
  let shown = $derived(people.slice(0, Math.max(1, stack?.max ?? 5)));
</script>

{#if stack && people.length}
  <div class="flex items-center -space-x-2">
    {#each shown as person}
      <div class="flex size-7 shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-2 ring-background" title={person.name}>
        {#if person.src}<img src={mediaUrl(person.src)} alt={person.name} onerror={fallBackToDefaultAvatar} class="size-full object-cover" />{:else}{(person.name || '?')[0].toUpperCase()}{/if}
      </div>
    {/each}
    {#if people.length > shown.length}
      <div class="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-[10px] font-bold text-muted-foreground ring-2 ring-background">+{people.length - shown.length}</div>
    {/if}
  </div>
{/if}
