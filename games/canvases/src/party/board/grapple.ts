import type { BoardLogEntry, Member } from '../protocol';

export type GrappleNotice = { heading: string; title: string; detail: string; tone: 'gold' | 'cyan' | 'muted'; delay: number };

const caughtCoins = (entry: BoardLogEntry) => entry.item === 'grapple' && entry.quality === 'hit' && Number(entry.coins) > 0;
const flightSeconds = (entry: BoardLogEntry) => Math.max(0.5, Number(entry.flight) || 1);
const CATCH_PAUSE = 0.75;

/** Only a successful hook slows locally, just before it catches. */
export function grappleFlightSeconds(entry: BoardLogEntry) {
  return flightSeconds(entry) + (caughtCoins(entry) ? CATCH_PAUSE : 0);
}

/** Preserve the initial throw speed, then ease through a readable slow catch. */
export function grappleTravelProgress(progress: number, originalFlight: number) {
  const flight = Math.max(0.5, originalFlight);
  const elapsed = Math.max(0, Math.min(1, progress)) * (flight + CATCH_PAUSE);
  const slowAt = flight * 0.72;
  return elapsed <= slowAt ? elapsed / flight : 0.72 + 0.28 * (elapsed - slowAt) / (flight * 0.28 + CATCH_PAUSE);
}

/** Explicit results, including a hook that lands but has nothing to steal. */
export function grappleResult(entry: BoardLogEntry, members: ReadonlyArray<Pick<Member, 'seat' | 'name'>>, viewer: number | null): GrappleNotice | null {
  if (entry.kind !== 'item' || entry.item !== 'grapple') return null;
  const name = (seat: unknown) => members.find(member => member.seat === seat)?.name ?? 'That player';
  const actor = entry.by === viewer ? 'You' : name(entry.by);
  const target = entry.target === viewer ? 'you' : name(entry.target);
  const delay = typeof entry.flight === 'number' ? grappleFlightSeconds(entry) : 0;
  if (caughtCoins(entry)) {
    const coins = Number(entry.coins);
    return { heading: 'HOOK LANDED', title: entry.by === viewer ? `+${coins} COINS` : entry.target === viewer ? `−${coins} COINS` : `${coins} COINS STOLEN`,
      detail: `${actor} ${entry.by === viewer ? 'took' : 'takes'} ${coins} coins from ${target}.`, tone: 'gold', delay };
  }
  if (entry.quality === 'empty' || entry.quality === 'hit') return {
    heading: 'HOOK LANDED', title: 'EMPTY POCKETS', detail: `${name(entry.target)} has no coins to take.`, tone: 'muted', delay,
  };
  if (entry.quality === 'blocked') return {
    heading: 'HOOK BLOCKED', title: 'SHIELD BROKEN', detail: `${name(entry.target)} kept their coins. Their shield absorbed the hook.`, tone: 'cyan', delay,
  };
  if (entry.quality === 'dropped') return {
    heading: 'HOOK DROPPED', title: 'TIME RAN OUT', detail: 'Release your throw before the timer runs out.', tone: 'muted', delay: 0,
  };
  return { heading: 'HOOK MISSED', title: 'NO PLAYER HOOKED', detail: 'Land the hook near a rival to steal up to 8 coins.', tone: 'muted', delay };
}
