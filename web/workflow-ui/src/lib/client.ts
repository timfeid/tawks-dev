export type ClientInfo = {
  /** Running inside the Tawks desktop app rather than a browser. */
  desktop: boolean;
  os: 'windows' | 'macos' | 'linux' | 'android' | 'ios' | 'other';
  /** Absent from older builds, so workflows can hold back what those can't draw. */
  renderer?: number;
};

const RENDERER = 2;

let info: ClientInfo = { desktop: false, os: 'other', renderer: RENDERER };

/** Tells workflows where they are shown, as `$client` in conditions and text,
 * for example `$client.desktop`. */
export function setClientInfo(next: ClientInfo) {
  info = { ...next, renderer: RENDERER };
}

export function clientInfo(): ClientInfo {
  return info;
}
