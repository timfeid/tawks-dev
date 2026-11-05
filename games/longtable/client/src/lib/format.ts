import type { Part, RollView, SheetView, TableView } from './types';

export const ABILITIES = ['STR', 'DEX', 'CON', 'INT', 'WIS', 'CHA'] as const;
export const QUICK_DICE = [4, 6, 8, 10, 12, 20, 100] as const;

export function modifier(score: number) {
  return Math.floor((score - 10) / 2);
}

export function signed(n: number) {
  return n >= 0 ? `+${n}` : `${n}`;
}

/** The formula a quick die button rolls, with the tray's modifier and edge. */
export function quickFormula(sides: number, mod: number, edge: 'advantage' | 'disadvantage' | null) {
  const base = `1d${sides}${mod ? signed(mod) : ''}`;
  if (sides === 20 && edge === 'advantage') return `${base} adv`;
  if (sides === 20 && edge === 'disadvantage') return `${base} dis`;
  return base;
}

/** A plain reading of a roll's parts, such as `(4, 15) + 5`. */
export function partsText(parts: Part[]) {
  return parts
    .map((part, i) => {
      const sign = part.negative ? '- ' : i === 0 ? '' : '+ ';
      if (part.kind === 'flat') return `${sign}${part.value}`;
      return `${sign}(${part.dice.map((d) => d.value).join(', ')})`;
    })
    .join(' ');
}

export function secretText(roll: Extract<RollView, { shown: 'secret' }>) {
  return roll.by_gm ? 'The GM rolled in secret' : `${roll.by_name} rolled for the GM`;
}

export function hpTone(hp: number, max: number) {
  if (hp <= 0) return 'down';
  const share = hp / Math.max(1, max);
  if (share <= 0.25) return 'low';
  if (share <= 0.5) return 'hurt';
  return 'well';
}

export function nameOf(table: TableView, userId: string | null) {
  if (!userId) return '';
  return table.people.find((p) => p.user_id === userId)?.name ?? 'Someone';
}

/** My sheets first, then the rest in table order. */
export function orderSheets(sheets: SheetView[], me: string) {
  return [...sheets.filter((s) => s.owner === me), ...sheets.filter((s) => s.owner !== me)];
}

export function clampInt(value: string, low: number, high: number, fallback: number) {
  const n = Number.parseInt(value, 10);
  if (Number.isNaN(n)) return fallback;
  return Math.min(high, Math.max(low, n));
}
