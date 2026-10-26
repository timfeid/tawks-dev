<script lang="ts">
  import { mediaUrl } from '../../media.js';
  import { getNestedValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

  // Stored avatars are `default/<file>` or a file under the user's own folder;
  // anything already absolute is used as is.
  const avatarSrc = (user: { id?: string; avatar_url: string }) =>
    /^https?:\/\//.test(user.avatar_url)
      ? user.avatar_url
      : `https://cdn.tawks.app/avatars/${user.avatar_url.startsWith('default/') ? user.avatar_url : `${user.id}/${user.avatar_url}`}`;
</script>

{#if typeof display.type !== 'string' && 'UserAvatar' in display.type}
  {@const user = getNestedValue(inputs, display.type.UserAvatar.key)}
  <div class="inline-flex items-center justify-center h-8 w-8 rounded-full bg-primary/20 text-primary overflow-hidden border">
    {#if user?.avatar_url}
        <img src={mediaUrl(avatarSrc(user))} alt={user.username || 'User'} class="h-full w-full object-cover" />
    {:else}
        <span class="text-[10px] font-bold">{(user?.username || '?')[0].toUpperCase()}</span>
    {/if}
  </div>
{/if}