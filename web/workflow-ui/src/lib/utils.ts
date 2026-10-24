import type { JsonValue } from '@tawks/workflow-types';

/** A compared value may name another response, as `{game.owner_user_id}` does. */
function expected(value: unknown, inputs: Record<string, any>): unknown {
  return typeof value === 'string' && /\{[^}]+\}/.test(value) ? interpolate(inputs, value) : value;
}

export function evaluateCondition(condition: any, inputs: Record<string, any>): boolean {
  if (condition === 'Always') return true;
  if (!condition) return true;

  if (typeof condition === 'object') {
    if ('Not' in condition) return !evaluateCondition(condition.Not, inputs);
    if ('And' in condition) return condition.And.every((c: any) => evaluateCondition(c, inputs));
    if ('Or' in condition) return condition.Or.some((c: any) => evaluateCondition(c, inputs));
    
    if ('ResponseExists' in condition) {
      const val = getNestedValue(inputs, condition.ResponseExists);
      return val !== undefined && val !== null && val !== '';
    }
    
    if ('ResponseEquals' in condition) {
      const val = getNestedValue(inputs, condition.ResponseEquals.field);
      return deepEqual(val, expected(condition.ResponseEquals.value, inputs));
    }

    if ('ContextEquals' in condition) {
        // For preview, we might not have context, but let's try to find it in inputs
        const val = getNestedValue(inputs, condition.ContextEquals.field);
        return deepEqual(val, expected(condition.ContextEquals.value, inputs));
    }
    
    if ('Gte' in condition) {
      const val = getNestedValue(inputs, condition.Gte.field);
      const min = expected(condition.Gte.value, inputs);
      const number = (v: unknown) => (typeof v === 'number' ? v : typeof v === 'string' && v.trim() !== '' ? Number(v) : NaN);
      if (!Number.isNaN(number(val)) && !Number.isNaN(number(min))) return number(val) >= number(min);
      if (typeof val === 'string' && typeof min === 'string') return val >= min;
      if (typeof val === 'boolean' && typeof min === 'boolean') return val || !min;
      return false;
    }

    if ('ResponseListEmpty' in condition) {
      const val = getNestedValue(inputs, condition.ResponseListEmpty);
      return Array.isArray(val) && val.length === 0;
    }

    if ('SomeInList' in condition) {
        const list = getNestedValue(inputs, condition.SomeInList.list);
        if (!Array.isArray(list)) return false;
        return list.some(item => evaluateCondition(condition.SomeInList.condition, { ...inputs, [condition.SomeInList.as]: item }));
    }
  }

  return false;
}

export function cn(...inputs: any[]) {
  return inputs.filter(Boolean).join(' ');
}

const UNSAFE_PATH_KEYS = new Set(['__proto__', 'constructor', 'prototype']);

function pathSegments(path: string): string[] {
  return path.split('.').filter(Boolean);
}

function isUnsafePath(keys: string[]): boolean {
  return keys.some((key) => UNSAFE_PATH_KEYS.has(key));
}

export function getNestedValue(obj: any, path: string): any {
  if (!path) return obj;
  const keys = pathSegments(path);
  if (isUnsafePath(keys)) return undefined;
  return keys.reduce((acc, key) => {
    if (acc == null || typeof acc !== 'object') return undefined;
    return Object.prototype.hasOwnProperty.call(acc, key) ? acc[key] : undefined;
  }, obj);
}

export function setNestedValue(obj: any, path: string, value: JsonValue): void {
  const keys = pathSegments(path);
  if (keys.length === 0 || isUnsafePath(keys)) return;
  let curr = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    if (curr[keys[i]] == null || typeof curr[keys[i]] !== 'object') {
      curr[keys[i]] = {};
    }
    curr = curr[keys[i]];
  }
  curr[keys[keys.length - 1]] = value;
}

export function deepEqual(a: any, b: any): boolean {
  if (a === b) return true;
  if (typeof a !== typeof b) return false;
  if (a && b && typeof a === 'object') {
    if (Array.isArray(a) !== Array.isArray(b)) return false;
    if (Array.isArray(a)) {
      if (a.length !== (b as any).length) return false;
      for (let i = 0; i < a.length; i++) {
        if (!deepEqual(a[i], (b as any)[i])) return false;
      }
      return true;
    } else {
      const keysA = Object.keys(a);
      const keysB = Object.keys(b);
      if (keysA.length !== keysB.length) return false;
      for (const key of keysA) {
        if (!deepEqual(a[key], b[key])) return false;
      }
      return true;
    }
  }
  return false;
}

export function objectInterpolate(values: object, text: string) {
  const val = interpolate(values, text);
  try {
    return JSON.parse(val);
  } catch (e) {
    return val;
  }
}

export function interpolate(values: object, text: string | undefined | null): string {
  if (!text) {
    return '';
  }
  return text.replace(/\{([^\{\}]+)\}/g, (_, key) => {
    let value = getNestedValue(values, key.trim());
    // Treat null and undefined identically — both indicate "no value", and
    // we don't want String(null) → "null" appearing in URLs/templates.
    if (value === undefined || value === null) return `{${key}}`;
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  });
}

export function resolveInputValue(inputs: any, val: any): any {
  if (val === null || val === undefined) {
    return val;
  }

  // String → interpolate
  if (typeof val === 'string') {
    return interpolate(inputs, val);
  }

  // Primitive passthrough
  if (typeof val === 'number' || typeof val === 'boolean') {
    return val;
  }

  // Array → recurse
  if (Array.isArray(val)) {
    return val.map((v) => resolveInputValue(inputs, v));
  }

  // Object → recurse
  if (typeof val === 'object') {
    const out: Record<string, any> = {};
    for (const [k, v] of Object.entries(val)) {
      out[k] = resolveInputValue(inputs, v);
    }
    return out;
  }

  return val;
}

export function debounce<T extends (...args: any[]) => any>(fn: T, delay: number): T {
  let timeoutId: ReturnType<typeof setTimeout>;
  return function (...args: any[]) {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  } as T;
}

type TimeOpts =
  | { timeago: true }
  | {
      timeago?: boolean | null;
      format?: string | null;
      showTimeForToday?: boolean | null;
      timeForTodayFormat?: string | null;
    }
  | string
  | undefined
  | null;

const RTF = typeof Intl !== 'undefined' && typeof Intl.RelativeTimeFormat === 'function'
  ? new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
  : null;

const MONTHS = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

/** "5 minutes ago", "2 hours ago", "yesterday", etc. */
function timeAgo(d: Date): string {
  const seconds = Math.round((Date.now() - d.getTime()) / 1000);
  if (seconds < 5) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return RTF ? RTF.format(-minutes, 'minute') : `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return RTF ? RTF.format(-hours, 'hour') : `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 7) return RTF ? RTF.format(-days, 'day') : `${days}d ago`;
  const weeks = Math.round(days / 7);
  if (weeks < 5) return RTF ? RTF.format(-weeks, 'week') : `${weeks}w ago`;
  const months = Math.round(days / 30);
  if (months < 12) return RTF ? RTF.format(-months, 'month') : `${months}mo ago`;
  const years = Math.round(days / 365);
  return RTF ? RTF.format(-years, 'year') : `${years}y ago`;
}

/** "Apr 5, 7pm" — month + day + bare hour with am/pm, no minutes. */
function shortDateWithHour(d: Date): string {
  const month = MONTHS[d.getMonth()];
  const day = d.getDate();
  const hours24 = d.getHours();
  const ampm = hours24 >= 12 ? 'pm' : 'am';
  const hours12 = hours24 % 12 || 12;
  return `${month} ${day}, ${hours12}${ampm}`;
}

/**
 * Smart time formatting:
 *   - < 1 day old  → relative ("5 minutes ago")
 *   - ≥ 1 day old  → "Apr 5, 7pm"
 *
 * Honors a few legacy opts shapes for backward compatibility:
 *   - { timeago: true }     → always relative
 *   - { format: '...' }     → ignored (we always use the smart format)
 */
export function formatDate(
  date: string | number | Date | null | undefined,
  opts?: TimeOpts,
): string {
  if (date == null || date === '') return '';
  const d = new Date(date as any);
  if (isNaN(d.getTime())) return String(date);

  // Force-relative override
  if (opts && typeof opts === 'object' && 'timeago' in opts && opts.timeago === true) {
    return timeAgo(d);
  }

  // Smart default
  const ageMs = Date.now() - d.getTime();
  const oneDayMs = 24 * 60 * 60 * 1000;
  return ageMs < oneDayMs ? timeAgo(d) : shortDateWithHour(d);
}
