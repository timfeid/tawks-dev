<script lang="ts">
  import { fallBackToDefaultAvatar } from '@tawks/ui/avatar/avatar-url.js';
  import { mediaUrl } from '../../media.js';
  import { interpolate, pictureUrl } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let person = $derived(typeof display.type !== 'string' && 'Person' in display.type ? display.type.Person : null);
  let name = $derived(person ? interpolate(inputs, person.name) : '');
  let src = $derived(person ? pictureUrl(interpolate(inputs, person.avatar)) : null);
  let size = $derived(look.personSize[person?.size ?? 'Medium']);
  let userId = $derived(person?.user ? interpolate(inputs, person.user).trim() : '');
</script>

{#snippet content()}
  <div class={['flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/15 font-bold text-primary', size.avatar]}>
    {#if src}<img src={mediaUrl(src)} alt="" onerror={fallBackToDefaultAvatar} class="size-full object-cover" />{:else}{(name || '?')[0].toUpperCase()}{/if}
  </div>
  <div class="flex min-w-0 flex-col">
    <span class={['truncate font-semibold text-foreground', size.name, userId && 'group-hover:underline']}>{name}</span>
    {#if person?.meta}<span class="truncate text-xs text-muted-foreground">{interpolate(inputs, person.meta)}</span>{/if}
  </div>
{/snippet}

{#if person}
  {#if userId}
    <button type="button" data-person-id={userId} class="group flex min-w-0 items-center gap-2.5 rounded-sm text-left">
      {@render content()}
    </button>
  {:else}
    <div class="flex min-w-0 items-center gap-2.5">{@render content()}</div>
  {/if}
{/if}
