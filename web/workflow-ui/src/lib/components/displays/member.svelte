<script lang="ts">
  import { fallBackToDefaultAvatar } from '@tawks/ui/avatar/avatar-url.js';
  import { mediaUrl } from '../../media.js';
  import { memberName } from '../../member-names.svelte';
  import { interpolate, pictureUrl } from '../../utils';
  import * as look from '../../look';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();
  let member = $derived(typeof display.type !== 'string' && 'Member' in display.type ? display.type.Member : null);
  let found = $derived(member ? memberName(interpolate(inputs, member.member).trim()) : null);
  let src = $derived(found?.avatar_url ? pictureUrl(found.avatar_url) : null);
  let size = $derived(look.personSize[member?.size ?? 'Medium']);
</script>

{#if member}
  <div class="flex min-w-0 items-center gap-2.5">
    <div class={['bg-primary/15 flex shrink-0 items-center justify-center overflow-hidden rounded-full', size.avatar]}>
      {#if src}<img src={mediaUrl(src)} alt="" onerror={fallBackToDefaultAvatar} class="size-full object-cover" />{/if}
    </div>
    <span class={['text-foreground truncate font-semibold', size.name]}>{found?.name ?? 'A member'}</span>
  </div>
{/if}
