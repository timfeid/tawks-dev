import type { SpaceKind } from '../protocol';

/** What each kind of space does, for the tip shown when it is hovered. The
 * numbers follow the board's rules in open/games/party/src/board.rs. */
export const SPACE_HELP: Record<SpaceKind, { label: string; help: string }> = {
  start: { label: 'Start', help: 'Where everyone sets off. Aim your vacuum at pedestrians to rob cash, or collect nearby props.' },
  coin: { label: 'Sidewalk', help: 'A safe place to stand. Aim and hold the vacuum to rob nearby pedestrians or collect props, while walking or standing.' },
  loss: { label: 'Rough space', help: 'Land here and lose 3 cash.' },
  item: { label: 'Item space', help: 'Land here for a random item, if your bag has room.' },
  event: { label: 'Toll space', help: 'Land here and everyone else pays you up to 2 coins.' },
  shop: { label: 'Shop', help: 'Stop to buy an item, if your bag has room.' },
  hazard: { label: 'Spikes', help: 'Land here and lose 5 coins and a heart.' },
  teleport: { label: 'Teleport', help: 'Land here to warp to the space it links to.' },
  prize_spot: { label: 'Candy spot', help: 'Buy candy here when it is open; otherwise it pays coins.' },
  cannon: { label: 'Cannon', help: 'Climb in to be fired up to 12 spaces along the board.' },
  tide: { label: 'Tide space', help: 'Washes up 5 coins at low tide, and costs 3 when the tide is in.' },
  ice: { label: 'Ice', help: 'Slippery: you slide straight on to the next space.' },
};
