<script lang="ts">
  import { getContext } from 'svelte';
  import type { UserResource } from '@tawks/workflow-types';
  import { cn } from '../utils.js';
  import { avatarSrc, fallBackToDefaultAvatar } from './avatar-url.js';

  let {
    user,
    class: className = 'size-7',
  }: { user: UserResource | string; class?: string } = $props();

  const getUser = getContext<(id: string) => UserResource | undefined>('getUser');

  let u = $derived.by(() => {
    if (typeof user === 'string') {
      return getUser?.(user);
    }

    return user;
  });

  let url = $derived(avatarSrc(u));
</script>

{#if u}
  <img
    src={url}
    alt={u.display_name}
    data-person-id={u.id}
    onerror={fallBackToDefaultAvatar}
    class={cn('rounded-lg object-cover', className)}
  />
{/if}
