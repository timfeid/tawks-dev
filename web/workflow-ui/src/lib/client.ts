export type ClientInfo = {
  /** Running inside the Tawks desktop app rather than a browser. */
  desktop: boolean;
  os: 'windows' | 'macos' | 'linux' | 'android' | 'ios' | 'other';
};

let info: ClientInfo = { desktop: false, os: 'other' };

/** Tells workflows where they are shown, as `$client` in conditions and text,
 * for example `$client.desktop`. */
export function setClientInfo(next: ClientInfo) {
  info = next;
}

export function clientInfo(): ClientInfo {
  return info;
}
