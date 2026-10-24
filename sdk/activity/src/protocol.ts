/** Messages between a game in an activity frame and the tawks page around it. */

export const PROTOCOL = 'tawks.activity';
export const VERSION = 1;

/** Largest move a game may send, as JSON. */
export const MAX_INPUT_BYTES = 16 * 1024;
/** Moves a game may send per second, with bursts up to twice that. */
export const SENDS_PER_SECOND = 20;

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

export type Me = { id: string };
export type Lobby = { id: string };

/** Posted by the game to window.parent until tawks answers. */
export type Hello = { protocol: typeof PROTOCOL; version: number; type: 'hello' };
/** Posted by tawks to the frame, carrying the port all later messages use. */
export type Welcome = { protocol: typeof PROTOCOL; version: number; type: 'welcome' };

export type HostMessage =
  | { type: 'init'; me: Me; lobby: Lobby | null; state: Json }
  /**
   * The player's push-to-talk key (`KeyboardEvent.key` in lower case) or
   * mouse button (`mouse3`..`mouse5`), or null when push-to-talk is off. Key
   * presses inside the frame never reach tawks, so the SDK reports this one.
   */
  | { type: 'push_to_talk'; binding: string | null }
  | { type: 'state'; state: Json }
  | { type: 'result'; id: number; ok: true }
  | { type: 'result'; id: number; ok: false; error: string };

export type GameMessage =
  | { type: 'send'; id: number; input: Json }
  | { type: 'push_to_talk'; down: boolean }
  | { type: 'call'; id: number; action: string; inputs: { [key: string]: Json } };

/** Action ids a game may call: one of its own workflow's actions. */
export const ACTION_ID = /^[A-Za-z0-9_.:-]{1,128}$/;

export function isHello(data: unknown): data is Hello {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as Hello).protocol === PROTOCOL &&
    (data as Hello).type === 'hello'
  );
}

export function isWelcome(data: unknown): data is Welcome {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as Welcome).protocol === PROTOCOL &&
    (data as Welcome).type === 'welcome'
  );
}

/** `MouseEvent.button` as a push-to-talk binding: middle is mouse3, back and forward mouse4 and mouse5. */
export function mouseBinding(button: number): string {
  return ['mouse1', 'mouse3', 'mouse2', 'mouse4', 'mouse5'][button] ?? `mouse${button + 1}`;
}
