/**
 * Pure helpers for anything people compete in over time: who plays whom, who
 * picks when, who ranks where. They hold no state and read no clock, so they can
 * be run again after a crash and give the same answer.
 */

export type Pairing<T> = { home: T; away: T | null };

/**
 * Round robin for any number of weeks, with every team playing once a week. An
 * odd number of teams gets one bye a week (`away: null`), passed around fairly.
 * Past one full cycle it starts again with home and away swapped.
 */
export function roundRobin<T>(teams: readonly T[], weeks: number): Pairing<T>[][] {
  if (teams.length < 2) return Array.from({ length: weeks }, () => []);
  const seats: (T | null)[] = [...teams];
  if (seats.length % 2 === 1) seats.push(null);
  const cycle = seats.length - 1;
  const out: Pairing<T>[][] = [];
  let order = seats.slice();
  for (let week = 0; week < weeks; week += 1) {
    if (week > 0 && week % cycle === 0) order = seats.slice();
    const swap = Math.floor(week / cycle) % 2 === 1;
    const pairs: Pairing<T>[] = [];
    for (let i = 0; i < order.length / 2; i += 1) {
      const a = order[i];
      const b = order[order.length - 1 - i];
      if (a === null || b === null) {
        pairs.push({ home: (a ?? b) as T, away: null });
      } else {
        const flip = (week + i) % 2 === 1 !== swap;
        pairs.push(flip ? { home: b, away: a } : { home: a, away: b });
      }
    }
    out.push(pairs);
    order = [order[0], order[order.length - 1], ...order.slice(1, -1)];
  }
  return out;
}

/** The team on the clock for each pick: order, then order reversed, and so on. */
export function snakeOrder<T>(order: readonly T[], rounds: number): T[] {
  const out: T[] = [];
  for (let round = 0; round < rounds; round += 1) {
    out.push(...(round % 2 === 0 ? order : [...order].reverse()));
  }
  return out;
}

/**
 * Sorts by each key in turn, highest first, then by `last` so the result never
 * depends on the order rows came in. Give a coin flip nobody can game, such as a
 * hash of the season and the team, as `last`.
 */
export function rankBy<T>(rows: readonly T[], keys: ((row: T) => number)[], last: (row: T) => string): T[] {
  return [...rows].sort((a, b) => {
    for (const key of keys) {
      const difference = key(b) - key(a);
      if (Math.abs(difference) > 1e-9) return difference;
    }
    return last(a) < last(b) ? -1 : last(a) > last(b) ? 1 : 0;
  });
}

export type Match<T> = { high: T | null; low: T | null };

/**
 * First round of a single elimination bracket from teams already in seed order.
 * Top seeds get the byes when the count isn't a power of two (`low: null`), and
 * the sides are laid out so seeds 1 and 2 can only meet in the final.
 */
export function bracket<T>(seeds: readonly T[]): Match<T>[] {
  let size = 1;
  while (size < seeds.length) size *= 2;
  let order = [1];
  while (order.length < size) {
    const next: number[] = [];
    const total = order.length * 2 + 1;
    for (const seed of order) next.push(seed, total - seed);
    order = next;
  }
  const out: Match<T>[] = [];
  for (let i = 0; i < order.length; i += 2) {
    const a = seeds[order[i] - 1] ?? null;
    const b = seeds[order[i + 1] - 1] ?? null;
    out.push(a === null ? { high: b, low: null } : { high: a, low: b });
  }
  return out;
}

/** The next round's matches from the winners of this one, in bracket order. */
export function nextRound<T>(winners: readonly T[]): Match<T>[] {
  const out: Match<T>[] = [];
  for (let i = 0; i < winners.length; i += 2) out.push({ high: winners[i], low: winners[i + 1] ?? null });
  return out;
}
