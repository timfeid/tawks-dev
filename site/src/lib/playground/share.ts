import lz from 'lz-string';

// lz-string is CommonJS, so named imports fail during server rendering.
const { compressToEncodedURIComponent, decompressFromEncodedURIComponent } = lz;

export type Shared = { definition: unknown; mocks: unknown };

export function encode(shared: Shared): string {
  return compressToEncodedURIComponent(JSON.stringify(shared));
}

export function decode(hash: string): Shared | null {
  const raw = new URLSearchParams(hash.replace(/^#/, '')).get('p');
  if (!raw) return null;
  try {
    const parsed = JSON.parse(decompressFromEncodedURIComponent(raw) ?? 'null');
    return parsed && typeof parsed === 'object' && 'definition' in parsed ? (parsed as Shared) : null;
  } catch {
    return null;
  }
}
