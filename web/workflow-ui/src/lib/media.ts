let resolve = (url: string) => url;

/** Lets the host app rewrite where display images and sounds load from, for
 * example through a privacy proxy. */
export function setMediaUrlResolver(resolver: (url: string) => string) {
  resolve = resolver;
}

export function mediaUrl<T extends string | null | undefined>(url: T): T | string {
  return url ? resolve(url) : url;
}
