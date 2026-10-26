import type { CovenCard, CovenCharacter, CovenLog, CovenSeat, CovenVerdict } from '../protocol';

export type CardColor = 'red' | 'green' | 'blue' | 'black';

export const CARDS: Record<
  CovenCard,
  { title: string; text: string; color: CardColor; accusations: number }
> = {
  rumor: {
    title: 'Rumor',
    text: 'One accusation against a player.',
    color: 'red',
    accusations: 1,
  },
  testimony: {
    title: 'Testimony',
    text: 'Three accusations against a player.',
    color: 'red',
    accusations: 3,
  },
  spectral_evidence: {
    title: 'Spectral Evidence',
    text: 'Seven accusations at once: the accused turns over a verdict card now.',
    color: 'red',
    accusations: 7,
  },
  vouching: {
    title: 'Vouching',
    text: 'Clear up to three accusations from a player.',
    color: 'green',
    accusations: 0,
  },
  barn_fire: {
    title: 'Barn Fire',
    text: 'Burn every blue card in front of a player.',
    color: 'green',
    accusations: 0,
  },
  hex: {
    title: 'Hex',
    text: 'Discard one blue card in front of a player.',
    color: 'green',
    accusations: 0,
  },
  pickpocket: {
    title: 'Pickpocket',
    text: "Take a player's whole hand and give it to another player.",
    color: 'green',
    accusations: 0,
  },
  deflection: {
    title: 'Deflection',
    text: 'Move every red and blue card from one player to another.',
    color: 'green',
    accusations: 0,
  },
  sanctuary: {
    title: 'Sanctuary',
    text: 'This player cannot be killed at night.',
    color: 'blue',
    accusations: 0,
  },
  good_standing: {
    title: 'Good Standing',
    text: 'No red cards may be played on this player.',
    color: 'blue',
    accusations: 0,
  },
  pillory: {
    title: 'Pillory',
    text: 'This player skips their next turn.',
    color: 'blue',
    accusations: 0,
  },
  betrothal: {
    title: 'Betrothal',
    text: 'Bound to whoever holds the other Betrothal: if one is killed at night, both die.',
    color: 'blue',
    accusations: 0,
  },
  familiar: {
    title: 'Familiar',
    text: "The witches' cat. Counts as one accusation and follows the blue cards.",
    color: 'blue',
    accusations: 1,
  },
  nightfall: {
    title: 'Nightfall',
    text: 'Night falls. The witches choose a victim and the constable chooses whom to guard.',
    color: 'black',
    accusations: 0,
  },
  conspiracy: {
    title: 'Conspiracy',
    text: 'Everyone takes a face-down verdict card from the player on their left.',
    color: 'black',
    accusations: 0,
  },
};

export const CHARACTERS: Record<
  CovenCharacter,
  { title: string; text: string; ability: 'peek' | 'tide' | 'cure' | null }
> = {
  minister: {
    title: 'Minister',
    text: 'Once per game, on your turn, look at one face-down verdict card of any player.',
    ability: 'peek',
  },
  magistrate: {
    title: 'Magistrate',
    text: 'When you place the final accusation, six are enough instead of seven.',
    ability: null,
  },
  farmer: {
    title: 'Farmer',
    text: 'When you draw, draw three cards instead of two.',
    ability: null,
  },
  midwife: {
    title: 'Midwife',
    text: 'A Betrothal cannot drag you to your death.',
    ability: null,
  },
  innkeeper: {
    title: 'Innkeeper',
    text: 'You may play Vouching on yourself.',
    ability: null,
  },
  blacksmith: {
    title: 'Blacksmith',
    text: 'It takes nine accusations to make you turn over a verdict card.',
    ability: null,
  },
  widow: {
    title: 'Widow',
    text: 'When the player on your left dies, their hand becomes yours.',
    ability: null,
  },
  servant: {
    title: 'Servant',
    text: 'Pickpocket and Deflection cannot be played on you.',
    ability: null,
  },
  trapper: {
    title: 'Trapper',
    text: 'A Familiar placed in front of you is thrown out at once.',
    ability: null,
  },
  schoolmaster: {
    title: 'Schoolmaster',
    text: 'You sit out every Conspiracy: you neither give nor take a verdict card.',
    ability: null,
  },
  sea_captain: {
    title: 'Sea Captain',
    text: 'Once per game, on your turn, bring Nightfall to the top of the deck.',
    ability: 'tide',
  },
  physician: {
    title: 'Physician',
    text: 'Once per game, on your turn, clear every accusation from any player, yourself included.',
    ability: 'cure',
  },
};

export const VERDICTS: Record<CovenVerdict, { title: string; color: string }> = {
  innocent: { title: 'Innocent', color: '#4ade80' },
  witch: { title: 'Witch', color: '#c084fc' },
  constable: { title: 'Constable', color: '#fbbf24' },
};

export const COLOR_HEX: Record<CardColor, string> = {
  red: '#b91c1c',
  green: '#15803d',
  blue: '#1d4ed8',
  black: '#111827',
};

/** Cards that need one player picked, and those that need a second one too. */
export const needsTarget = (card: CovenCard) => CARDS[card].color !== 'black';
export const needsSecond = (card: CovenCard) => card === 'pickpocket' || card === 'deflection';
export const needsBlueChoice = (card: CovenCard) => card === 'hex';

export function describe(entry: CovenLog, seats: CovenSeat[]): string {
  const who = (seat: number | undefined | null) =>
    seat === undefined || seat === null ? 'Someone' : (seats[seat]?.name ?? 'Someone');
  const card = entry.card
    ? (CARDS[entry.card as CovenCard]?.title ??
      VERDICTS[entry.card as CovenVerdict]?.title ??
      entry.card)
    : '';
  switch (entry.type) {
    case 'dawn':
      return `The Familiar curls up in front of ${who(entry.seat)}.`;
    case 'draw':
      return `${who(entry.seat)} drew a card.`;
    case 'play':
      return entry.second !== undefined &&
        entry.second !== null &&
        (entry.card === 'pickpocket' || entry.card === 'deflection')
        ? `${who(entry.seat)} played ${card} from ${who(entry.target)} to ${who(entry.second)}.`
        : `${who(entry.seat)} played ${card} on ${who(entry.target)}.`;
    case 'reveal':
      return `${who(entry.seat)} turned over a verdict card: ${card}.`;
    case 'death':
      return entry.how === 'night'
        ? `${who(entry.seat)} was killed in the night.${entry.witch ? ' They were a witch.' : ''}`
        : entry.how === 'betrothal'
          ? `${who(entry.seat)} died beside their betrothed.${entry.witch ? ' They were a witch.' : ''}`
          : `${who(entry.seat)} is out.${entry.witch ? ' They were a witch.' : ''}`;
    case 'nightfall':
      return `${who(entry.seat)} drew Nightfall.`;
    case 'conspiracy':
      return `${who(entry.seat)} drew Conspiracy.`;
    case 'conspiracy_done':
      return 'Verdict cards changed hands around the table.';
    case 'night':
      return 'The witches have chosen. Anyone may confess before the deed is done.';
    case 'confess':
      return `${who(entry.seat)} confessed.`;
    case 'spared':
      return `${who(entry.seat)} was spared.`;
    case 'skipped':
      return `${who(entry.seat)} sat in the pillory and lost a turn.`;
    case 'peek':
      return `${who(entry.seat)} took a quiet look at one of ${who(entry.target)}'s cards.`;
    case 'tide':
      return `${who(entry.seat)} brought the night closer.`;
    case 'cure':
      return `${who(entry.seat)} cleared every accusation from ${who(entry.target)}.`;
    case 'over':
      return entry.winner === 'town'
        ? 'Every witch has been found. The town wins.'
        : 'Only witches remain. The coven wins.';
    default:
      return '';
  }
}
