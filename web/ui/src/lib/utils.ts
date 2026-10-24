import type { JsonValue } from '@tawks/workflow-types';
import { parseISO, format, formatDistanceToNow, isToday, isYesterday } from 'date-fns';
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, 'child'> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & {
  ref?: U | null;
};

export function getNestedValue(obj: any, path: string): JsonValue | undefined {
  return path.split('.').reduce((acc, key) => acc?.[key], obj);
}

export function setNestedValue(obj: any, path: string, value: JsonValue): void {
  const keys = path.split('.');
  let curr = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    if (curr[keys[i]] == null || typeof curr[keys[i]] !== 'object') {
      curr[keys[i]] = {};
    }
    curr = curr[keys[i]];
  }
  curr[keys[keys.length - 1]] = value;
}

export function objectInterpolate(values: object, text: string) {
  const val = interpolate(values, text);
  if (val === undefined) {
    return undefined;
  }
  try {
    return JSON.parse(val);
  } catch (e) {
    return val;
  }
}

export function resolveInputValue(
  inputs: any,
  val: JsonValue | undefined,
): JsonValue | undefined {
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
    return val.map((v: JsonValue) => resolveInputValue(inputs, v) ?? null);
  }

  // Object → recurse
  if (typeof val === 'object') {
    const out: Record<string, JsonValue> = {};
    for (const [k, v] of Object.entries(val)) {
      out[k] = resolveInputValue(inputs, v as JsonValue) ?? null;
    }
    return out;
  }

  // Fallback (symbol, function, etc.)
  return val;
}

export function interpolate(values: object, text: string) {
  if (!text) {
    return;
  }
  return text.replace(/\{([^\{\}]+)\}/g, (_, key) => {
    let value = getNestedValue(values, key.trim());
    if (value === undefined) return `{${key}}`;
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  });
}

export function timeAgo(timestamp: string) {
  const currentDate = new Date();
  const currentTimestamp = currentDate.getTime();
  const messageDate = new Date(+timestamp);
  const differenceInMilliseconds = currentTimestamp - +timestamp;

  const millisecondsInADay = 24 * 60 * 60 * 1000;

  if (differenceInMilliseconds < millisecondsInADay) {
    return messageDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  if (differenceInMilliseconds < 2 * millisecondsInADay) {
    return `yesterday @ ${messageDate.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
  }

  return messageDate.toLocaleDateString([], {
    month: 'numeric',
    day: 'numeric',
    year: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
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

export function debounce<T extends (...args: any[]) => void>(func: T, wait: number) {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  return function (this: ThisParameterType<T>, ...args: Parameters<T>) {
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => {
      func.apply(this, args);
    }, wait);
  };
}

export function parseDate(date: string) {
  return parseISO(date);
}

export function formatDate(
  date: string,
  options:
    | { timeago: true }
    | {
        timeago?: false | null;
        format?: string | null;
        showTimeForToday?: boolean | null;
        timeForTodayFormat?: string | null;
      } = {},
): string {
  try {
    const defaultFormat = 'PPpp';
    const defaultTimeFormat = 'p';
    const parsedDate = parseDate(date);
    if (isNaN(parsedDate.getTime())) return '';
    if (options?.timeago) {
      const distance = formatDistanceToNow(parsedDate, { addSuffix: true });
      return distance === 'less than a minute ago' ? 'just now' : distance;
    }
    // if options?.timeago = true ONLY timeago logic
    if (isToday(parsedDate)) {
      if (options.showTimeForToday) {
        const fmt = options.timeForTodayFormat || options.format || defaultTimeFormat;
        return format(parsedDate, fmt);
      } else {
        const distance = formatDistanceToNow(parsedDate, { addSuffix: true });
        return distance === 'less than a minute ago' ? 'just now' : distance;
      }
    }
    if (isYesterday(parsedDate)) {
      if (options.showTimeForToday) {
        const fmt = options.timeForTodayFormat || options.format || defaultTimeFormat;
        return `yesterday @ ${format(parsedDate, fmt)}`;
      } else {
        return 'yesterday';
      }
    }
    // For other days, just return default format or custom format if provided
    const fmt = options.format || defaultFormat;
    return format(parsedDate, fmt);
  } catch {
    return '';
  }
}
