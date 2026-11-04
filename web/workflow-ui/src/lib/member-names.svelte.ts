import { interpolate } from './utils';

/** What a hub's bot writes for a person: an id that means nothing to it. */
export type MemberName = { name: string; avatar_url: string | null };

const TOKEN = /@\[(m_[a-z0-9]{10,40})\]/g;
const REFRESH_MS = 60_000;

let known = $state<Record<string, MemberName>>({});
let lookup: ((ids: string[]) => Promise<Record<string, MemberName>>) | null = null;
const seen = new Set<string>();
const waiting = new Set<string>();
let timer: ReturnType<typeof setTimeout> | null = null;
let refresher: ReturnType<typeof setInterval> | null = null;

function fetchWaiting() {
  timer = null;
  const ids = [...waiting];
  waiting.clear();
  if (!ids.length || !lookup) return;
  void lookup(ids)
    .then((names) => {
      known = { ...known, ...names };
    })
    .catch(() => {});
}

function want(id: string) {
  if (seen.has(id)) return;
  seen.add(id);
  waiting.add(id);
  timer ??= setTimeout(fetchWaiting, 30);
}

/** Set once by the app: turns ids into names for whoever is looking now. */
export function setMemberLookup(next: (ids: string[]) => Promise<Record<string, MemberName>>) {
  lookup = next;
  // Names change and accounts go, and a page should show that.
  refresher ??= setInterval(() => {
    for (const id of seen) waiting.add(id);
    fetchWaiting();
  }, REFRESH_MS);
}

export function memberName(id: string): MemberName | null {
  if (!/^m_[a-z0-9]{10,40}$/.test(id)) return { name: 'Deleted user', avatar_url: null };
  want(id);
  return known[id] ?? null;
}

/**
 * Text as a person reads it, with each `@[m_...]` the current name. Only for what
 * goes on screen. Nothing sent back to a bot may pass through here, or a page
 * could read a name out of its own input.
 */
export function withNames(text: string): string {
  if (!text.includes('@[m_')) return text;
  return text.replace(TOKEN, (_, id: string) => {
    want(id);
    return known[id]?.name ?? 'a member';
  });
}

export function shown(values: object, text: string | undefined | null): string {
  return withNames(interpolate(values, text));
}
