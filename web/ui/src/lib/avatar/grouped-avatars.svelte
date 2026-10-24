<script lang="ts">
  import { Avatar as AvatarPrimitive } from 'bits-ui';
  import * as Avatar from './index.js';
  import { UserAvatar } from './index.js';

  const sizeClasses = {
    xs: 'h-4 w-4 text-xs',
    sm: 'h-6 w-6 text-sm',
    md: 'h-8 w-8 text-base',
    lg: 'h-9 w-9 text-base',
  };

  let {
    avatars,
    max = 5,
    size = 'sm',
  }: {
    avatars: { user_id?: string; username?: string }[];
    max?: number;
    size?: keyof typeof sizeClasses;
  } = $props();

  let visibleAvatars = $derived([...avatars].slice(0, max));
  let remaining = $derived(avatars.length - max);

  const colors = [
    ['oklch(58.6% 0.253 17.585)', '#000'],
    ['oklch(60% 0.118 184.704)', '#000'],
    ['oklch(64.6% 0.222 41.116)', '#000'],
    ['oklch(58.8% 0.158 241.966)', '#000'],
    ['oklch(62.7% 0.194 149.214)', '#000'],
  ];

  function getColor(firstName: string, lastName = '') {
    const initialF = firstName[0] || '';
    const initialL = lastName[0] || '';
    const seed = [initialF, initialL]
      .filter(Boolean)
      .map((c) => c.toLowerCase().charCodeAt(0) - 97)
      .reduce((a, b) => a + b, 0);

    return colors[seed % colors.length] || '#C7115A';
  }

  function getAvatarByName(options: {
    firstName: string;
    rounded?: boolean;
    color?: string;
    bold?: boolean;
    size?: number | string;
    background?: string;
    'font-size'?: string;
    format?: string;
  }) {
    const [background, color] = getColor(options.firstName);
    const name = (options.firstName || '').substring(0, 1);

    const params = {
      background: options.background || background,
      name,
      color: options.color || color,
      rounded: JSON.stringify(options.rounded || false),
      bold: JSON.stringify(options.bold || true),
      size: (options.size || 56).toString(),
      'font-size': options['font-size'] || '.6',
      format: options.format || 'svg',
    };

    return (
      `https://ui-avatars.com/api/?` + new URLSearchParams(params).toString()
    );
  }
</script>

<div class="flex -space-x-1">
  {#each visibleAvatars as avatar}
    <Avatar.Root class="{sizeClasses[size]} rounded-md font-mono">
      {#if avatar.avatar_url}
        <UserAvatar user={avatar} class={sizeClasses[size]}></UserAvatar>
      {:else if avatar.user_id}
        <UserAvatar user={avatar.user_id} class={sizeClasses[size]}
        ></UserAvatar>
      {/if}
    </Avatar.Root>
  {/each}

  {#if remaining > 0}
    <div
      class="{sizeClasses[
        size
      ]} bg-muted text-muted-foreground z-10 flex items-center justify-center rounded-full"
    >
      <span class="text-xs font-medium">+{remaining}</span>
    </div>
  {/if}
</div>
