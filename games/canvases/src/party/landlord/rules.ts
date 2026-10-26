import catalogJson from './catalog.json';

export type Group =
  | 'docks'
  | 'cannery'
  | 'old_town'
  | 'market'
  | 'foundry'
  | 'strip'
  | 'parkside'
  | 'hilltop'
  | 'rails'
  | 'works';

export type Kind =
  | { kind: 'cash' }
  | { kind: 'property'; group: Group }
  | { kind: 'wild'; groups: Group[] }
  | { kind: 'rent'; groups: Group[] }
  | { kind: 'takeover' }
  | { kind: 'lawyer_up' }
  | { kind: 'backroom_deal' }
  | { kind: 'trade_up' }
  | { kind: 'repo_man' }
  | { kind: 'fundraiser' }
  | { kind: 'rent_hike' }
  | { kind: 'renovation' }
  | { kind: 'tower' }
  | { kind: 'payday' };

export type CardSpec = { id: number; kind: Kind; value: number; title: string };

export const CATALOG: CardSpec[] = catalogJson as CardSpec[];
export const card = (id: number) => CATALOG[id];

export const GROUPS: Group[] = [
  'docks',
  'cannery',
  'old_town',
  'market',
  'foundry',
  'strip',
  'parkside',
  'hilltop',
  'rails',
  'works',
];

export const GROUP_INFO: Record<
  Group,
  { title: string; color: string; size: number; rents: number[] }
> = {
  docks: { title: 'The Docks', color: '#8b5a2b', size: 2, rents: [1, 2] },
  cannery: { title: 'Cannery Row', color: '#7dd3fc', size: 3, rents: [1, 2, 3] },
  old_town: { title: 'Old Town', color: '#f472b6', size: 3, rents: [1, 2, 4] },
  market: { title: 'Market Square', color: '#fb923c', size: 3, rents: [1, 3, 5] },
  foundry: { title: 'Foundry District', color: '#ef4444', size: 3, rents: [2, 3, 6] },
  strip: { title: 'Sunset Strip', color: '#facc15', size: 3, rents: [2, 4, 6] },
  parkside: { title: 'Parkside', color: '#22c55e', size: 3, rents: [2, 4, 7] },
  hilltop: { title: 'Hilltop', color: '#3b82f6', size: 2, rents: [3, 8] },
  rails: { title: 'Rail Lines', color: '#1f2937', size: 4, rents: [1, 2, 3, 4] },
  works: { title: 'The Works', color: '#a3e635', size: 2, rents: [1, 2] },
};

export const ACTION_TEXT: Record<string, string> = {
  takeover: 'Take a complete district from a rival, buildings and all.',
  lawyer_up: 'Cancel an action played against you. Only playable as a response.',
  backroom_deal: 'Take one deed from a rival that is not in a complete district.',
  trade_up: 'Swap one of your deeds for a rival deed outside a complete district.',
  repo_man: 'One rival pays you $5.',
  fundraiser: 'Every rival pays you $2.',
  rent_hike: 'Play with a rent card to double it. Costs a play of its own.',
  renovation: 'Add $3 rent to a complete district of houses.',
  tower: 'Add $4 rent to a district that already has a renovation.',
  payday: 'Draw two cards.',
  cash: 'Money for the bank.',
};

export const isProperty = (spec: CardSpec) =>
  spec.kind.kind === 'property' || spec.kind.kind === 'wild';
export const groupsOf = (spec: CardSpec): Group[] => {
  const k = spec.kind;
  if (k.kind === 'property') return [k.group];
  if (k.kind === 'wild' || k.kind === 'rent') return k.groups.length ? k.groups : GROUPS;
  return [];
};

export function cardColor(spec: CardSpec): string {
  const k = spec.kind;
  if (k.kind === 'cash') return '#166534';
  if (k.kind === 'property') return GROUP_INFO[k.group].color;
  if (k.kind === 'wild') return k.groups.length ? GROUP_INFO[k.groups[0]].color : '#6b7280';
  if (k.kind === 'rent') return '#7c3aed';
  return '#b45309';
}

/** Cash cards are titled by their value, which the card already shows. */
export const cardTitle = (spec: CardSpec) => (spec.kind.kind === 'cash' ? 'Cash' : spec.title);

/** The short line printed on a card on the 3D table. */
export const tableText = (spec: CardSpec) =>
  spec.kind.kind === 'property' ? GROUP_INFO[spec.kind.group].title : cardText(spec);

export function cardText(spec: CardSpec): string {
  const k = spec.kind;
  if (k.kind === 'property') {
    const info = GROUP_INFO[k.group];
    return `A deed in ${info.title}. ${info.size} complete it; rent ${info.rents.map((r) => `$${r}`).join(' / ')}.`;
  }
  if (k.kind === 'wild')
    return k.groups.length
      ? `Counts as ${k.groups.map((g) => GROUP_INFO[g].title).join(' or ')}.`
      : 'Counts as any district. Worth nothing as payment.';
  if (k.kind === 'rent')
    return k.groups.length
      ? `Everyone pays rent on ${k.groups.map((g) => GROUP_INFO[g].title).join(' or ')}.`
      : 'One rival pays rent on any of your districts.';
  return ACTION_TEXT[k.kind] ?? '';
}

export type LandlordSet = {
  group: Group;
  cards: number[];
  house: boolean;
  hotel: boolean;
  complete: boolean;
  rent: number;
};
export type LandlordPlayer = {
  seat: number;
  hand_size: number;
  bank: number[];
  bank_total: number;
  sets: LandlordSet[];
  complete: number;
};
export type Effect =
  | { kind: 'charge'; from: number; to: number; amount: number; what: string }
  | { kind: 'steal'; from: number; to: number; card: number }
  | { kind: 'swap'; from: number; to: number; take: number; give: number }
  | { kind: 'takeover'; from: number; to: number; group: Group };
export type LandlordPhase =
  | { kind: 'turn'; seat: number; plays_left: number }
  | { kind: 'respond'; target: number; effect: Effect; cancelled: boolean; can_lawyer: boolean }
  | { kind: 'pay'; from: number; to: number; amount: number; options: number[] | null }
  | { kind: 'discard'; seat: number; over: number }
  | { kind: 'over'; winner: number };
export type LandlordSeat = {
  user_id: string | null;
  name: string;
  avatar_url: string | null;
  character: string | null;
  connected: boolean;
};
export type LandlordLog = Record<string, unknown> & { type: string };
export type LandlordView = {
  players: LandlordPlayer[];
  hand: number[] | null;
  phase: LandlordPhase;
  seconds_left: number;
  deck: number;
  discard_top: number | null;
  round: number;
  queued: number;
  log: LandlordLog[];
  seats: LandlordSeat[];
  me: number | null;
};
export type LandlordAction =
  | { type: 'bank'; card: number }
  | { type: 'place'; card: number; group?: Group }
  | { type: 'move'; card: number; group: Group }
  | { type: 'payday'; card: number }
  | { type: 'rent'; card: number; group: Group; target?: number; double?: number }
  | { type: 'repo'; card: number; target: number }
  | { type: 'fundraiser'; card: number }
  | { type: 'backroom'; card: number; target: number; take: number }
  | { type: 'trade'; card: number; target: number; take: number; give: number }
  | { type: 'takeover'; card: number; target: number; group: Group }
  | { type: 'build'; card: number; group: Group }
  | { type: 'end_turn' }
  | { type: 'discard'; cards: number[] }
  | { type: 'pay'; cards: number[] }
  | { type: 'lawyer'; card: number }
  | { type: 'accept' };

export function describeEffect(effect: Effect, seats: LandlordSeat[]): string {
  const who = (s: number) => seats[s]?.name ?? 'Someone';
  switch (effect.kind) {
    case 'charge':
      return `${who(effect.to)} charges ${who(effect.from)} $${effect.amount} for ${effect.what}.`;
    case 'steal':
      return `${who(effect.to)} takes ${card(effect.card).title} from ${who(effect.from)}.`;
    case 'swap':
      return `${who(effect.to)} swaps ${card(effect.give).title} for ${who(effect.from)}'s ${card(effect.take).title}.`;
    case 'takeover':
      return `${who(effect.to)} takes over ${who(effect.from)}'s ${GROUP_INFO[effect.group].title}.`;
  }
}

export function describe(entry: LandlordLog, seats: LandlordSeat[]): string {
  const who = (s: unknown) => (typeof s === 'number' ? (seats[s]?.name ?? 'Someone') : 'Someone');
  const title = (id: unknown) => (typeof id === 'number' ? card(id).title : '');
  const group = (g: unknown) => GROUP_INFO[g as Group]?.title ?? '';
  switch (entry.type) {
    case 'turn':
      return `${who(entry.seat)} drew ${entry.drew} cards.`;
    case 'bank':
      return `${who(entry.seat)} banked ${title(entry.card)}.`;
    case 'place':
      return `${who(entry.seat)} laid ${title(entry.card)} in ${group(entry.group)}.`;
    case 'move':
      return `${who(entry.seat)} moved a deed to ${group(entry.group)}.`;
    case 'payday':
      return `${who(entry.seat)} cashed a Payday.`;
    case 'rent':
      return `${who(entry.seat)} charged $${entry.amount} rent on ${group(entry.group)}${entry.everyone ? ' from everyone' : ''}.`;
    case 'repo':
      return `${who(entry.seat)} sent the Repo Man to ${who(entry.target)}.`;
    case 'fundraiser':
      return `${who(entry.seat)} held a fundraiser.`;
    case 'backroom':
      return `${who(entry.seat)} went after ${who(entry.target)}'s ${title(entry.card)}.`;
    case 'trade':
      return `${who(entry.seat)} offered ${title(entry.give)} for ${who(entry.target)}'s ${title(entry.take)}.`;
    case 'takeover':
      return `${who(entry.seat)} moved on ${who(entry.target)}'s ${group(entry.group)}.`;
    case 'build':
      return `${who(entry.seat)} built a ${entry.tower ? 'tower' : 'renovation'} in ${group(entry.group)}.`;
    case 'owes':
      return `${who(entry.from)} owes ${who(entry.to)} $${entry.amount}.`;
    case 'paid':
      return `${who(entry.from)} paid ${who(entry.to)} $${entry.amount}.`;
    case 'broke':
      return `${who(entry.seat)} had nothing to pay with.`;
    case 'lawyer':
      return `${who(entry.seat)} lawyered up.`;
    case 'blocked':
      return 'The lawyers won. Nothing happens.';
    case 'stolen':
      return `${who(entry.to)} took ${title(entry.card)} from ${who(entry.from)}.`;
    case 'swapped':
      return `${who(entry.to)} and ${who(entry.from)} swapped deeds.`;
    case 'taken_over':
      return `${who(entry.to)} took over ${group(entry.group)} from ${who(entry.from)}.`;
    case 'over':
      return `${who(entry.winner)} owns three districts and wins.`;
    default:
      return '';
  }
}
