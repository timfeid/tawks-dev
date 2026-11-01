<script lang="ts">
  import { avatarSrc, fallBackToDefaultAvatar } from '@tawks/ui/avatar/avatar-url.js';
  import { mediaUrl } from '../../media.js';
  import { getNestedValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

</script>

{#if typeof display.type !== 'string' && 'UserAvatar' in display.type}
  {@const user = getNestedValue(inputs, display.type.UserAvatar.key)}
  <div class="inline-flex items-center justify-center h-8 w-8 rounded-full bg-primary/20 text-primary overflow-hidden border">
    {#if user?.avatar_url}
        <img src={mediaUrl(avatarSrc(user))} alt={user.username || 'User'} onerror={fallBackToDefaultAvatar} class="h-full w-full object-cover" />
    {:else}
        <span class="text-[10px] font-bold">{(user?.username || '?')[0].toUpperCase()}</span>
    {/if}
  </div>
{/if}