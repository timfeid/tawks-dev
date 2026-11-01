import type { Item } from '../protocol';

export type ItemInfo = {
  label: string;
  color: string;
  help: string;
  /** Thrown or swung in first person; the rest act on the user straight away.
   * Nothing is ever aimed by picking a rival. */
  handheld: boolean;
};

export const ITEMS: Record<Item, ItemInfo> = {
  bat: { label: 'Bat', color: '#f59e0b', help: 'Swing it in first person: face them and stop the needle in the middle. Everyone in the arc is knocked back and loses a heart.', handheld: true },
  drive_by: { label: 'Drive-by', color: '#fb923c', help: 'Move on foot with 10 rounds for up to 10 seconds. Hold fire for one shot every 0.8 seconds; three hits take one heart and up to 3 coins. Shields block the completed hit chain.', handheld: true },
  slapshot: { label: 'Slapshot', color: '#67e8f9', help: 'Wind up a puck at a rival. A clean hit costs 4 coins and one heart, and knocks them back one space. A shield stops it.', handheld: true },
  drive_car: {label:'Rental car',color:'#facc15',help:'A car pulls up for a ten-second drive. Steer on the marked roads, honk at close calls and avoid traffic: a collision is an instant knockout. Then return to your space.',handheld:false},
  slash_rush: { label: 'Slash rush', color: '#38bdf8', help: 'Leave your space for a short hockey-stick rush, even on another player’s turn. Close in and swing; then return home.', handheld: true },
  bomb: {
    label: 'Bomb',
    color: '#ef4444',
    help: 'Throw it in first person, judging the power and the wind. A direct hit costs 8 coins and a heart; close by, 5. Mind your own feet.',
    handheld: true,
  },
  grapple: { label: 'Grapple', color: '#22d3ee', help: 'Hook a rival to reel in up to 8 of their coins. Aim where they stand; the hook steals coins and leaves both players in place. Shields block it.', handheld: true },
  trap: { label: 'Trap', color: '#a78bfa', help: 'Throw it: it sets, hidden, on whatever space it lands on.', handheld: true },
  double_dice: { label: 'Double dice', color: '#fbbf24', help: 'Stop the needle well for two dice this turn; perfectly for 3 coins too.', handheld: false },
  golden_die: { label: 'Golden die', color: '#facc15', help: 'Pick a number, then stop the needle: perfect rolls it exactly.', handheld: false },
  swap: { label: 'Swap', color: '#38bdf8', help: 'Throw the swap orb: trade places with whoever it lands by.', handheld: true },
  shield: { label: 'Shield', color: '#93c5fd', help: 'Stop the needle well to raise a shield against the next hit.', handheld: false },
  medkit: { label: 'Medkit', color: '#4ade80', help: 'A hard, fast needle: a perfect stop heals you to full.', handheld: false },
  compass: { label: 'Compass', color: '#f9a8d4', help: 'Stop the needle to travel toward the chest: Perfect drops you 1 space short, Good 3, Weak 6. A miss leaves you here.', handheld: false },
  curse: { label: 'Curse', color: '#c084fc', help: 'Throw the hex: whoever it lands by rolls no higher than 3 next turn.', handheld: true },
  pickpocket: { label: 'Pickpocket', color: '#fb923c', help: 'Throw the sticky hand: it takes an item from whoever it lands by.', handheld: true },
  snowball: { label: 'Snowball', color: '#e0f2fe', help: 'Throw it: whoever it lands by is frozen solid and misses their next turn.', handheld: true },
  crate: { label: 'Crate', color: '#b45309', help: 'Throw it to put up a barricade: whoever walks into it stops there.', handheld: true },
  heist: { label: 'Heist', color: '#f472b6', help: 'Crack the safe on a fast needle to take candy off whoever has the most.', handheld: false },
};
