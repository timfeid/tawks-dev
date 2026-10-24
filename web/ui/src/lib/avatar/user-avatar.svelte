<script lang="ts">
  import { getContext } from 'svelte';
  import type { UserResource } from '@tawks/workflow-types';
  import { cn } from '../utils.js';

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

  let url = $derived.by(() => {
    if (!u) {
      return `https://cdn.tawks.app/avatars/default-1.png`;
    }

    const ending = u.avatar_url.startsWith('default/')
      ? u.avatar_url
      : `${u.id}/${u.avatar_url}`;

    return `https://cdn.tawks.app/avatars/${ending}`;
  });
</script>

{#if u}
  <img
    src={url}
    alt={u.display_name}
    class={cn('rounded-lg object-cover', className)}
  />
{/if}
