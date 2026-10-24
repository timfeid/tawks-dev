export type RuleTextSegment =
  | {
      type: 'text';
      text: string;
    }
  | {
      type: 'term';
      text: string;
      key: string;
      title: string;
      description: string;
    };

type RuleMatcher = {
  key: string;
  pattern: RegExp;
  title?: string;
  describe: (match: RegExpMatchArray) => string;
};

function formatLabel(text: string): string {
  return text
    .trim()
    .replace(/\s+/g, ' ')
    .split(' ')
    .map((part) => {
      if (!part) return part;
      if (/^\d+$/.test(part)) return part;
      return `${part[0].toUpperCase()}${part.slice(1).toLowerCase()}`;
    })
    .join(' ');
}

function pluralize(
  count: number,
  singular: string,
  plural = `${singular}s`,
): string {
  return count === 1 ? singular : plural;
}

const RULE_MATCHERS: RuleMatcher[] = [
  {
    key: 'reactive_armor',
    pattern: /\breactive armor(?:\s+(\d+))?\b/i,
    title: 'Reactive Armor',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      return amount > 0
        ? `Reduces incoming damage by ${amount}.`
        : 'Reduces incoming damage.';
    },
  },
  {
    key: 'firewall',
    pattern: /\bfirewall(?:\s+(\d+))?\b/i,
    title: 'Firewall',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      return amount > 0
        ? `Spells and abilities that target this cost ${amount} more.`
        : 'Spells and abilities that target this cost extra.';
    },
  },
  {
    key: 'throttle',
    pattern: /\bthrottle(?:\s+(\d+))?\b/i,
    title: 'Throttle',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      return amount > 0
        ? `Opponents' spells cost ${amount} more while this is active.`
        : "Opponents' spells cost more while this is active.";
    },
  },
  {
    key: 'absorb',
    pattern: /\babsorb(?:\s+(\d+))?\b/i,
    title: 'Absorb',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      return amount > 0
        ? `Prevents the first ${amount} damage dealt to this each turn.`
        : 'Prevents some damage dealt to this each turn.';
    },
  },
  {
    key: 'modular',
    pattern: /\bmodular(?:\s+(\d+))?\b/i,
    title: 'Modular',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      if (amount > 0) {
        return `Enters with ${amount} +1/+1 ${pluralize(amount, 'counter')}. When it dies, you may move those counters to another unit you control.`;
      }
      return 'Enters with +1/+1 counters. When it dies, you may move those counters to another unit you control.';
    },
  },
  {
    key: 'rampage',
    pattern: /\brampage(?:\s+(\d+))?\b/i,
    title: 'Rampage',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      return amount > 0
        ? `Whenever this attacks, it deals ${amount} damage to the defending player.`
        : 'Whenever this attacks, it deals extra damage to the defending player.';
    },
  },
  {
    key: 'scavenge',
    pattern: /\bscavenge(?:\s+(\d+))?\b/i,
    title: 'Scavenge',
    describe: (match) => {
      const amount = Number(match[1] ?? 0);
      if (amount > 0) {
        return `Exile a unit from your graveyard to put ${amount} +1/+1 ${pluralize(amount, 'counter')} on a target unit.`;
      }
      return 'Exile a unit from your graveyard to put +1/+1 counters on a target unit.';
    },
  },
  {
    key: 'signal_jam',
    pattern: /\bsignal jam\b/i,
    title: 'Signal Jam',
    describe: () => "Opponents can't use activated abilities.",
  },
  {
    key: 'camouflage',
    pattern: /\bcamouflage\b/i,
    title: 'Camouflage',
    describe: () => "This can't be targeted by opponent spells or abilities.",
  },
  {
    key: 'stealth',
    pattern: /\b(?:stealth|unblockable)\b/i,
    describe: () => "This can't be blocked.",
  },
  {
    key: 'aero',
    pattern: /\baero\b/i,
    title: 'Aero',
    describe: () =>
      'This can only be blocked by units with Aero or Interceptor.',
  },
  {
    key: 'interceptor',
    pattern: /\binterceptor\b/i,
    title: 'Interceptor',
    describe: () => 'This can block Aero units.',
  },
  {
    key: 'flash',
    pattern: /\bflash\b/i,
    title: 'Flash',
    describe: () => 'You can cast this any time you could cast an instant.',
  },
  {
    key: 'gravecast',
    pattern: /\bgravecast\b/i,
    title: 'Gravecast',
    describe: () => 'You can cast this from your graveyard.',
  },
  {
    key: 'cascade',
    pattern: /\bcascade\b/i,
    title: 'Cascade',
    describe: () =>
      'When you cast this, reveal cards from the top of your deck until you hit a cheaper card, then cast that card for free.',
  },
  {
    key: 'haste',
    pattern: /\bhaste\b/i,
    title: 'Haste',
    describe: () => 'This can attack and use tap abilities immediately.',
  },
  {
    key: 'deathtouch',
    pattern: /\bdeathtouch\b/i,
    title: 'Deathtouch',
    describe: () => 'Any amount of damage this deals to a unit is lethal.',
  },
  {
    key: 'piercing',
    pattern: /\bpiercing\b/i,
    title: 'Piercing',
    describe: () => 'Damage from this ignores Armor and Shield.',
  },
  {
    key: 'glitch',
    pattern: /\bglitch\b/i,
    title: 'Glitch',
    describe: () => "This nullifies a unit's text box and abilities.",
  },
  {
    key: 'malware',
    pattern: /\bmalware\b/i,
    title: 'Malware',
    describe: () => 'Units damaged by this lose their abilities.',
  },
  {
    key: 'ethereal',
    pattern: /\bethereal\b/i,
    title: 'Ethereal',
    describe: () =>
      "This can't be targeted by physical damage or physical effects.",
  },
  {
    key: 'trample',
    pattern: /\btrample\b/i,
    title: 'Trample',
    describe: () => 'Extra combat damage carries over to the defending player.',
  },
  {
    key: 'indestructible',
    pattern: /\bindestructible\b/i,
    title: 'Indestructible',
    describe: () =>
      "This can't be destroyed by lethal damage or destroy effects.",
  },
  {
    key: 'vigilance',
    pattern: /\bvigilance\b/i,
    title: 'Vigilance',
    describe: () => "This doesn't tap when it attacks.",
  },
  {
    key: 'menace',
    pattern: /\bmenace\b/i,
    title: 'Menace',
    describe: () => 'This must be blocked by two or more units.',
  },
  {
    key: 'wither',
    pattern: /\bwither\b/i,
    title: 'Wither',
    describe: () =>
      'Damage this deals to units is dealt as -1/-1 counters instead.',
  },
  {
    key: 'evolve',
    pattern: /\bevolve\b/i,
    title: 'Evolve',
    describe: () =>
      'Whenever a bigger unit enters under your control, put a +1/+1 counter on this.',
  },
  {
    key: 'freeze',
    pattern: /\bfreeze\b/i,
    title: 'Freeze',
    describe: () =>
      "A frozen unit can't attack, block, or untap for the stated duration.",
  },
  {
    key: 'echo',
    pattern: /\becho\b/i,
    title: 'Echo',
    describe: () => 'At your next upkeep, pay the echo cost or sacrifice this.',
  },
];

type ParsedRuleMatch = {
  index: number;
  text: string;
  key: string;
  title: string;
  description: string;
};

function findNextRuleMatch(
  text: string,
  startIndex: number,
): ParsedRuleMatch | null {
  const remainder = text.slice(startIndex);
  let best: ParsedRuleMatch | null = null;

  for (const matcher of RULE_MATCHERS) {
    const relativeIndex = remainder.search(matcher.pattern);
    if (relativeIndex < 0) continue;

    const matched = remainder.slice(relativeIndex).match(matcher.pattern);
    if (!matched || !matched[0]) continue;

    const candidate: ParsedRuleMatch = {
      index: startIndex + relativeIndex,
      text: matched[0],
      key: matcher.key,
      title: matcher.title ?? formatLabel(matched[0]),
      description: matcher.describe(matched),
    };

    if (
      !best ||
      candidate.index < best.index ||
      (candidate.index === best.index &&
        candidate.text.length > best.text.length)
    ) {
      best = candidate;
    }
  }

  return best;
}

export function parseRulesText(
  text: string | null | undefined,
): RuleTextSegment[] {
  if (!text) return [];

  const source = text;
  const segments: RuleTextSegment[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const next = findNextRuleMatch(source, cursor);
    if (!next) {
      segments.push({ type: 'text', text: source.slice(cursor) });
      break;
    }

    if (next.index > cursor) {
      segments.push({ type: 'text', text: source.slice(cursor, next.index) });
    }

    segments.push({
      type: 'term',
      text: source.slice(next.index, next.index + next.text.length),
      key: next.key,
      title: next.title,
      description: next.description,
    });

    cursor = next.index + next.text.length;
  }

  return segments;
}

export function hasRuleTermMatch(text: string | null | undefined): boolean {
  return parseRulesText(text).some((segment) => segment.type === 'term');
}
