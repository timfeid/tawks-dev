import { describe, expect, it } from 'vitest';
import { clampInt, hpTone, modifier, orderSheets, partsText, quickFormula, signed } from './format';
import type { SheetView } from './types';

describe('format', () => {
  it('works out modifiers the way the server does', () => {
    expect([1, 8, 9, 10, 11, 12, 30].map(modifier)).toEqual([-5, -1, -1, 0, 0, 1, 10]);
    expect(signed(3)).toBe('+3');
    expect(signed(0)).toBe('+0');
    expect(signed(-2)).toBe('-2');
  });

  it('builds quick rolls with the tray settings', () => {
    expect(quickFormula(20, 5, 'advantage')).toBe('1d20+5 adv');
    expect(quickFormula(20, 0, 'disadvantage')).toBe('1d20 dis');
    expect(quickFormula(6, -1, 'advantage')).toBe('1d6-1');
    expect(quickFormula(100, 0, null)).toBe('1d100');
  });

  it('reads roll parts', () => {
    expect(
      partsText([
        { kind: 'dice', negative: false, count: 2, sides: 20, dice: [{ value: 4, kept: false }, { value: 15, kept: true }], subtotal: 15 },
        { kind: 'flat', negative: false, value: 5 },
        { kind: 'flat', negative: true, value: 1 },
      ]),
    ).toBe('(4, 15) + 5 - 1');
  });

  it('grades hit points and orders sheets', () => {
    expect([hpTone(0, 10), hpTone(2, 10), hpTone(5, 10), hpTone(9, 10)]).toEqual(['down', 'low', 'hurt', 'well']);
    const strip = (id: string, owner: string) =>
      ({ access: 'strip', id, owner, name: id, class_level: '', hp: 1, hp_max: 1, ac: 10, conditions: [], exhaustion: 0 }) as SheetView;
    expect(orderSheets([strip('a', 'x'), strip('b', 'me'), strip('c', 'y')], 'me').map((s) => s.id)).toEqual(['b', 'a', 'c']);
    expect(clampInt('abc', 0, 5, 2)).toBe(2);
    expect(clampInt('99', 0, 5, 2)).toBe(5);
  });
});
