const MAX_MODELS = 8;
const downloads = new Map<string, Promise<ArrayBuffer>>();

/** Rigs wearing the same model share one download; each still parses its own
 * scene, since cloning skinned hierarchies is not worth the risk here. The
 * bytes are read but never transferred or detached by the parser. */
export async function sharedModelBytes(url: string, download: (url: string) => Promise<ArrayBuffer>, signal?: AbortSignal): Promise<ArrayBuffer> {
  const pending = downloads.get(url) ?? download(url);
  downloads.delete(url);
  downloads.set(url, pending);
  pending.catch(() => { if (downloads.get(url) === pending) downloads.delete(url); });
  while (downloads.size > MAX_MODELS) downloads.delete(downloads.keys().next().value!);
  const bytes = await pending;
  if (signal?.aborted) throw new DOMException('Aborted', 'AbortError');
  return bytes;
}
