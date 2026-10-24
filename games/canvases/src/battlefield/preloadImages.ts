/**
 * Preloads a list of image URLs into the browser cache.
 * Returns a promise that resolves when all images are loaded (or failed).
 * Calls onProgress with (loaded, total) as each image completes.
 */
export function preloadImages(
  urls: string[],
  onProgress?: (loaded: number, total: number) => void,
): Promise<void> {
  if (urls.length === 0) {
    onProgress?.(0, 0);
    return Promise.resolve();
  }

  let loaded = 0;
  const total = urls.length;

  return new Promise((resolve) => {
    const done = () => {
      loaded++;
      onProgress?.(loaded, total);
      if (loaded >= total) resolve();
    };

    for (const url of urls) {
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.onload = done;
      img.onerror = done; // don't block on failures
      img.src = url;
    }
  });
}

import { CDN_BASE } from './cdn';

/** Static assets used every game session. */
const STATIC_ASSETS = [
  `${CDN_BASE}/card_back.jpg`,
  `${CDN_BASE}/metal.jpg`,
  `${CDN_BASE}/table.jpg`,
  `${CDN_BASE}/window.png`,
  `${CDN_BASE}/window-left.png`,
  `${CDN_BASE}/window-right.png`,
  `${CDN_BASE}/rarities/public.png`,
  `${CDN_BASE}/rarities/private.png`,
  `${CDN_BASE}/rarities/restricted.png`,
  `${CDN_BASE}/rarities/classified.png`,
  `${CDN_BASE}/rarities/singularity.png`,
];

/**
 * Collects all unique non-null image_url values from cards,
 * plus static CDN assets.
 */
export function collectImageUrls(cards: { image_url?: string | null }[]): string[] {
  const seen = new Set<string>(STATIC_ASSETS);
  for (const card of cards) {
    if (card.image_url) seen.add(card.image_url);
  }
  return [...seen];
}
