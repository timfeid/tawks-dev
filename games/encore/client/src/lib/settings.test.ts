import { describe, expect, it } from 'vitest';
import { readSettings } from './settings';

describe('readSettings', () => {
  it('mutes the lobby mic while singing unless the player turned it off', () => {
    expect(readSettings(null).muteLobbyWhileSinging).toBe(true);
    expect(readSettings({ muteLobbyWhileSinging: false }).muteLobbyWhileSinging).toBe(false);
    expect(readSettings({ muteLobbyWhileSinging: 'no' }).muteLobbyWhileSinging).toBe(true);
  });
});
