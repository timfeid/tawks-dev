import type { CardView, PlayerState } from './types';

export const derivePlayers = (
  gameState: any,
  viewerId?: string | null,
): PlayerState[] => {
  const players = (gameState?.players || []).map((player: any) => ({
    id: player.id,
    name: player.name,
    life: player.life,
    mana_pool: player.mana_pool,
    infrastructures: player.infrastructures,
    is_active: player.is_active,
    battlefield_units: player.battlefield_units || [],
    battlefield_infrastructure: player.battlefield_infrastructure || [],
    operator_zone: player.operator_zone || [],
    grid: player.grid || [],
    hand_size: player.hand_size,
    library_size: 20,
    graveyard_size: 0,
  }));
  if (!viewerId) {
    return players;
  }
  const viewerIndex = players.findIndex((player: any) => player.id === viewerId);
  if (viewerIndex <= 0) {
    return players;
  }
  return [...players.slice(viewerIndex), ...players.slice(0, viewerIndex)];
};

export const deriveViewer = (gameState: any) => gameState?.viewer ?? null;

export const deriveHand = (viewer: any): CardView[] => viewer?.hand ?? [];

export const deriveScry = (viewer: any): CardView[] => viewer?.scry ?? [];

export const deriveScryKey = (viewer: any) =>
  (viewer?.scry || []).map((card: any) => card.id).join('|');
