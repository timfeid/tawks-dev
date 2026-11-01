export const AVATAR_CDN = 'https://cdn.tawks.app/avatars';
export const DEFAULT_AVATAR = `${AVATAR_CDN}/default/default-1.png`;

/** Stored avatars are `default/<file>`, a file in the user's own folder, or
 * already a full URL. Older uploads kept their raw file names, so the name is
 * encoded to keep characters like `#`, `?` and `%` from breaking the URL. */
export function avatarSrc(user: { id?: string | null; avatar_url?: string | null } | null | undefined): string {
  const stored = user?.avatar_url?.trim();
  if (!stored) return DEFAULT_AVATAR;
  if (/^https?:\/\//i.test(stored)) return stored;
  if (stored.startsWith('default/')) return `${AVATAR_CDN}/${stored}`;
  if (!user?.id) return DEFAULT_AVATAR;
  return `${AVATAR_CDN}/${encodeURIComponent(user.id)}/${encodeURIComponent(stored)}`;
}

/** Swaps a failed avatar for the default once, so a missing default cannot loop. */
export function fallBackToDefaultAvatar(event: Event) {
  const img = event.currentTarget as HTMLImageElement | null;
  if (!img || img.src === DEFAULT_AVATAR) return;
  img.src = DEFAULT_AVATAR;
}
