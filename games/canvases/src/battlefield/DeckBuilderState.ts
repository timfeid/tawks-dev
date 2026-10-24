import type { CardView } from './types';

export function computeDeckBuilderLibrary(deckLib: any[]): CardView[] {
  const out: CardView[] = [];
  const seen = new Map<string, CardView>();
  (deckLib as CardView[]).forEach((card) => {
    const existing = seen.get(card.name);
    if (existing) {
      (existing as any).pile_count = ((existing as any).pile_count || 1) + 1;
    } else {
      const c = { ...card, pile_count: 1 };
      seen.set(card.name, c);
      out.push(c);
    }
  });
  return out;
}

export function computeOperatorLibrary(source: CardView[]): CardView[] {
  return source.filter((card) => {
    const label = (card?.kind_label || '').toLowerCase();
    const tag = (card?.kind_tag || '').toLowerCase();
    const supertype =
      typeof card?.supertype === 'string' ? card.supertype.toLowerCase() : '';

    const isApex =
      label.includes('apex') ||
      supertype === 'apex' ||
      (card?.supertype as any)?.apex !== undefined;

    const isOperatorKind =
      label.includes('unit') ||
      label.includes('agent') ||
      tag === 'unit' ||
      tag === 'agent';

    return isApex && isOperatorKind;
  });
}

export function computeDeckBuilderTableCards(deckBuilder: any): CardView[] {
  const out: CardView[] = [];
  const entries = deckBuilder?.cards || [];
  entries.forEach((entry: any) => {
    const base = entry?.card;
    if (!base) return;
    const qty = Math.max(0, entry?.quantity ?? 0);
    if (qty <= 0) return;
    out.push({ ...base, id: `${base.id}::pile`, pile_count: qty } as any);
  });
  return out;
}

export function computeDeckBuilderTotalCards(deckBuilder: any): number {
  if (!deckBuilder) return 0;
  const cardsTotal = (deckBuilder.cards || []).reduce(
    (acc: number, curr: any) => acc + (curr.quantity || 0),
    0,
  );
  const hasOperator = !!deckBuilder.operator_card_id;
  return cardsTotal + (hasOperator ? 1 : 0);
}

export function buildRingCards(
  library: CardView[],
  startIndex: number,
  hidden: Set<string>,
  maxSize: number,
  wrap = true,
): CardView[] {
  if (library.length === 0) return [];
  const targetSize = Math.min(maxSize, library.length);
  const out: CardView[] = [];
  let cursor = wrap
    ? ((startIndex % library.length) + library.length) % library.length
    : Math.max(0, Math.min(startIndex, library.length - 1));
  let guard = 0;
  while (out.length < targetSize && guard < library.length * 2) {
    if (!wrap && cursor >= library.length) break;
    const candidate = library[cursor];
    if (
      candidate &&
      !hidden.has(candidate.id) &&
      !out.find((c) => c.id === candidate.id)
    ) {
      out.push(candidate);
    }
    cursor = wrap ? (cursor + 1) % library.length : cursor + 1;
    guard += 1;
  }
  return out;
}
