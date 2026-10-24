import { connect, type Activity, type Json } from '@tawks/activity-sdk';

type Suit = 'crown' | 'gem' | 'chalice';
type Card =
  | { id: number; kind: 'treasure'; suit: Suit; value: number }
  | { id: number; kind: 'thief' | 'dragon' | 'scry' | 'shield' | 'trade' };
type Spot =
  | { at: 'deck' }
  | { at: 'discard' }
  | { at: 'market'; slot: number }
  | { at: 'hand' | 'vault' | 'shield'; seat: number };
type Motion = { from: Spot; to: Spot; card_id: number; card: Card | null };
type GameEvent = { seq: number; text: string; motions: Motion[] };
type Seat = { name: string; hand_count: number; vault: Card[]; shield: boolean; score: number };
type View = {
  phase: 'seating' | 'playing' | 'over';
  seat: number | null;
  seats: Seat[];
  watchers: string[];
  hand: Card[];
  deck_count: number;
  market: (Card | null)[];
  discard_top: Card | null;
  turn: number;
  deadline_ms: number;
  scry: Card[] | null;
  turns_left: number | null;
  can_pass: boolean;
  events: GameEvent[];
  winners: number[];
};

const ACTION_TEXT: Record<string, [string, string]> = {
  thief: ['Thief', "Steal the top card of a rival's vault."],
  dragon: ['Dragon', 'Every rival discards a random card.'],
  scry: ['Scry', 'Look at the top 3 cards, keep one.'],
  shield: ['Shield', 'Blocks the next Thief or Dragon.'],
  trade: ['Trade', 'Swap a hand card with a market card.'],
};

const ICONS: Record<string, string> = {
  crown: '<path d="M3 17h18l-1.5-9-4.5 4-3-7-3 7-4.5-4z"/><rect x="3" y="18.5" width="18" height="2.5" rx="1"/>',
  gem: '<path d="M7 3h10l4 6-9 12L3 9z"/><path d="M3 9h18M9 3l3 6 3-6M12 9v12" fill="none" stroke="rgba(0,0,0,.35)" stroke-width="1"/>',
  chalice: '<path d="M5 3h14c0 6-3 9-7 9s-7-3-7-9z"/><rect x="11" y="12" width="2" height="6"/><path d="M7 21c0-2 2-3 5-3s5 1 5 3z"/>',
  thief: '<path d="M2 10c3-3 6-3 10-1 4-2 7-2 10 1-1 4-4 5-6 4l-4-2-4 2c-2 1-5 0-6-4z"/><circle cx="7.5" cy="11" r="1.4" fill="#f4ead2"/><circle cx="16.5" cy="11" r="1.4" fill="#f4ead2"/>',
  dragon: '<path d="M3 20c2-7 6-11 12-12l-2-4 5 3 3-2-1 5c1 3 0 6-3 7l-3-3-4 3-3-1z"/><circle cx="16.5" cy="10" r="1" fill="#f4ead2"/>',
  scry: '<path d="M1 12c3-5 7-7 11-7s8 2 11 7c-3 5-7 7-11 7S4 17 1 12z"/><circle cx="12" cy="12" r="4" fill="#f4ead2"/><circle cx="12" cy="12" r="2"/>',
  shield: '<path d="M12 2l8 3v6c0 6-4 10-8 11-4-1-8-5-8-11V5z"/><path d="M12 5v14" stroke="#f4ead2" stroke-width="1.5"/>',
  trade: '<path d="M4 8h12l-3-3 1.5-1.5L20 9l-5.5 5.5L13 13l3-3H4z"/><path d="M20 16H8l3 3-1.5 1.5L4 15l5.5-5.5L11 11l-3 3h12z"/>',
};

const icon = (name: string) => `<svg viewBox="0 0 24 24" aria-hidden="true">${ICONS[name]}</svg>`;

const app = document.querySelector<HTMLDivElement>('#app')!;
let activity: Activity<View | null>;
let view: View | null = null;
let lastSeq = 0;
let selected: number | null = null;
let tradeSlot: number | null = null;
let toastTimer = 0;

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls = '', html = ''): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (html) node.innerHTML = html;
  return node;
}

function spotKey(spot: Spot): string {
  return 'seat' in spot ? `${spot.at}:${spot.seat}` : 'slot' in spot ? `market:${spot.slot}` : spot.at;
}

function cardName(card: Card): string {
  return card.kind === 'treasure' ? `${card.suit} ${card.value}` : ACTION_TEXT[card.kind][0];
}

function cardFace(card: Card, size: 'lg' | 'sm' = 'lg'): HTMLElement {
  const node = el('div', `card face ${size} ${card.kind === 'treasure' ? `treasure ${card.suit}` : `spell ${card.kind}`}`);
  node.dataset.cardId = String(card.id);
  node.setAttribute('aria-label', cardName(card));
  if (card.kind === 'treasure') {
    node.innerHTML = `<span class="pip tl">${card.value}</span><span class="emblem">${icon(card.suit)}</span><span class="pip br">${card.value}</span>`;
  } else {
    const [title, text] = ACTION_TEXT[card.kind];
    node.innerHTML = `<span class="title">${title}</span><span class="emblem">${icon(card.kind)}</span>${size === 'lg' ? `<span class="rule">${text}</span>` : ''}`;
  }
  return node;
}

function cardBack(size: 'lg' | 'sm' = 'lg'): HTMLElement {
  return el('div', `card back ${size}`, '<span class="sigil"></span>');
}

function send(input: Json) {
  activity.send(input).catch((error: Error) => toast(error.message));
}

function toast(text: string) {
  const node = document.querySelector<HTMLDivElement>('.toast');
  if (!node) return;
  node.textContent = text;
  node.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => node.classList.remove('show'), 3200);
}

function selectedCard(): Card | undefined {
  return view?.hand.find((c) => c.id === selected);
}

function myTurn(v: View): boolean {
  return v.phase === 'playing' && v.seat !== null && v.seat === v.turn;
}

function hint(v: View): string {
  if (v.phase !== 'playing') return '';
  if (v.seat === null) return `Watching. ${v.seats[v.turn].name} is thinking.`;
  if (!myTurn(v)) return `${v.seats[v.turn].name} is thinking...`;
  if (v.scry) return 'Keep one of the scried cards.';
  const card = selectedCard();
  if (!card) return 'Your turn: pick a card from your hand, take a market card, or draw 2 from the deck.';
  if (card.kind === 'treasure') return 'Bank it in your vault.';
  if (card.kind === 'thief') return 'Click a rival to rob their vault.';
  if (card.kind === 'trade') return tradeSlot === null ? 'Click a market card to trade for.' : 'Now click the hand card to give away.';
  return `Cast ${ACTION_TEXT[card.kind][0]}.`;
}

function renderSeat(v: View, i: number, mine: boolean): HTMLElement {
  const seat = v.seats[i];
  const node = el('section', `seat ${mine ? 'mine' : 'rival'}`);
  node.classList.toggle('active', v.phase === 'playing' && v.turn === i);
  node.classList.toggle('winner', v.winners.includes(i));
  const card = selectedCard();
  const targetable = !mine && myTurn(v) && card?.kind === 'thief' && seat.vault.length > 0;
  node.classList.toggle('targetable', targetable);
  if (targetable) node.addEventListener('click', () => {
    send({ type: 'cast', card: card!.id, target: i });
    selected = null;
  });

  const plate = el('div', 'plate');
  plate.innerHTML = `<span class="avatar"></span><span class="name"></span><span class="score">${seat.score}</span>`;
  plate.querySelector('.avatar')!.textContent = seat.name.slice(0, 1).toUpperCase();
  plate.querySelector('.name')!.textContent = mine ? `${seat.name} (you)` : seat.name;
  if (seat.shield) {
    const shield = el('span', 'shield-badge', icon('shield'));
    shield.dataset.spot = `shield:${i}`;
    shield.title = 'Shield up';
    plate.append(shield);
  } else {
    const anchor = el('span', 'shield-anchor');
    anchor.dataset.spot = `shield:${i}`;
    plate.append(anchor);
  }
  if (v.phase === 'playing' && v.turn === i) plate.append(el('span', 'clock'));
  node.append(plate);

  if (!mine) {
    const hand = el('div', 'rival-hand');
    hand.dataset.spot = `hand:${i}`;
    for (let k = 0; k < seat.hand_count; k++) {
      const back = cardBack('sm');
      back.style.setProperty('--i', String(k - (seat.hand_count - 1) / 2));
      hand.append(back);
    }
    node.append(hand);
  }

  const vault = el('div', 'vault');
  vault.dataset.spot = `vault:${i}`;
  if (!seat.vault.length) vault.append(el('span', 'empty', 'empty vault'));
  for (const c of seat.vault) vault.append(cardFace(c, 'sm'));
  node.append(vault);
  return node;
}

function renderCenter(v: View): HTMLElement {
  const center = el('section', 'center');
  const turn = myTurn(v) && !v.scry;

  const deck = el('button', 'pile deck');
  deck.dataset.spot = 'deck';
  deck.disabled = !turn || v.deck_count === 0;
  deck.append(v.deck_count ? cardBack() : el('div', 'card slot'));
  deck.append(el('span', 'count', `${v.deck_count}`));
  deck.title = 'Draw 2';
  deck.addEventListener('click', () => {
    selected = null;
    send({ type: 'forage' });
  });

  const market = el('div', 'market');
  v.market.forEach((card, slot) => {
    const stall = el('button', 'stall');
    stall.dataset.spot = `market:${slot}`;
    stall.disabled = !turn || !card;
    stall.classList.toggle('picked', tradeSlot === slot);
    stall.append(card ? cardFace(card) : el('div', 'card slot'));
    stall.addEventListener('click', () => {
      if (selectedCard()?.kind === 'trade') {
        tradeSlot = slot;
        render();
      } else {
        selected = null;
        send({ type: 'forage', slot });
      }
    });
    market.append(stall);
  });

  const discard = el('div', 'pile discard');
  discard.dataset.spot = 'discard';
  discard.append(v.discard_top ? cardFace(v.discard_top) : el('div', 'card slot'));

  const labels = el('div', 'labels', '<span>Deck</span><span class="wide">Market</span><span>Discard</span>');
  const row = el('div', 'row');
  row.append(deck, market, discard);
  center.append(row, labels);
  if (v.turns_left !== null && v.phase === 'playing') center.append(el('div', 'final', `Deck is empty: ${v.turns_left} turn${v.turns_left === 1 ? '' : 's'} left`));
  return center;
}

function renderHand(v: View): HTMLElement {
  const hand = el('div', 'hand');
  if (v.seat !== null) hand.dataset.spot = `hand:${v.seat}`;
  const n = v.hand.length;
  v.hand.forEach((card, k) => {
    const face = cardFace(card);
    face.style.setProperty('--i', String(k - (n - 1) / 2));
    face.classList.toggle('selected', card.id === selected);
    face.tabIndex = 0;
    face.addEventListener('click', () => {
      const chosen = selectedCard();
      if (chosen?.kind === 'trade' && tradeSlot !== null && card.id !== chosen.id) {
        send({ type: 'cast', card: chosen.id, slot: tradeSlot, give: card.id });
        selected = null;
        tradeSlot = null;
        return;
      }
      selected = selected === card.id ? null : card.id;
      tradeSlot = null;
      render();
    });
    hand.append(face);
  });
  return hand;
}

function renderTray(v: View): HTMLElement {
  const tray = el('div', 'tray');
  const text = el('span', 'hint');
  text.textContent = hint(v);
  tray.append(text);
  const card = selectedCard();
  if (myTurn(v) && card && !v.scry) {
    if (card.kind === 'treasure') {
      const b = el('button', 'act', 'Bank');
      b.addEventListener('click', () => {
        send({ type: 'bank', card: card.id });
        selected = null;
      });
      tray.append(b);
    } else if (card.kind === 'dragon' || card.kind === 'scry' || card.kind === 'shield') {
      const b = el('button', 'act', `Cast ${ACTION_TEXT[card.kind][0]}`);
      b.disabled = card.kind === 'scry' && v.deck_count === 0;
      b.addEventListener('click', () => {
        send({ type: 'cast', card: card.id });
        selected = null;
      });
      tray.append(b);
    }
  }
  if (v.can_pass) {
    const b = el('button', 'act', 'Pass');
    b.addEventListener('click', () => send({ type: 'pass' }));
    tray.append(b);
  }
  return tray;
}

function renderOverlay(v: View): HTMLElement | null {
  if (v.scry && myTurn(v)) {
    const box = el('div', 'overlay');
    const panel = el('div', 'panel', '<h2>Scry</h2><p>Keep one. The rest go to the bottom of the deck.</p>');
    const row = el('div', 'choices');
    for (const card of v.scry) {
      const face = cardFace(card);
      face.addEventListener('click', () => send({ type: 'keep', card: card.id }));
      row.append(face);
    }
    panel.append(row);
    box.append(panel);
    return box;
  }
  if (v.phase === 'seating') {
    const box = el('div', 'overlay');
    const panel = el('div', 'panel', '<h2>Hoard</h2><p>2 to 4 players. Bank treasure, rob vaults, wake the dragon. Three of a suit in your vault is worth 5 more.</p>');
    const list = el('ul', 'seated');
    v.seats.forEach((s) => {
      const li = el('li');
      li.textContent = s.name;
      list.append(li);
    });
    for (let k = v.seats.length; k < 4; k++) list.append(el('li', 'open', 'open seat'));
    panel.append(list);
    const buttons = el('div', 'buttons');
    if (v.seat === null) {
      const sit = el('button', 'act', 'Take a seat');
      sit.disabled = v.seats.length >= 4;
      sit.addEventListener('click', () => send({ type: 'sit' }));
      buttons.append(sit);
    } else {
      const start = el('button', 'act', 'Deal');
      start.disabled = v.seats.length < 2;
      start.addEventListener('click', () => send({ type: 'start' }));
      const stand = el('button', 'ghost', 'Stand up');
      stand.addEventListener('click', () => send({ type: 'stand' }));
      buttons.append(start, stand);
    }
    panel.append(buttons);
    if (v.watchers.length) panel.append(el('p', 'watchers', `Watching: ${v.watchers.join(', ')}`));
    box.append(panel);
    return box;
  }
  if (v.phase === 'over') {
    const box = el('div', 'overlay');
    const panel = el('div', 'panel', '<h2>The hoard is counted</h2>');
    const list = el('ol', 'results');
    [...v.seats.keys()]
      .sort((a, b) => v.seats[b].score - v.seats[a].score)
      .forEach((i) => {
        const li = el('li', v.winners.includes(i) ? 'won' : '');
        li.textContent = `${v.seats[i].name}: ${v.seats[i].score}`;
        list.append(li);
      });
    panel.append(list);
    if (v.seat !== null) {
      const again = el('button', 'act', 'Deal again');
      again.addEventListener('click', () => send({ type: 'rematch' }));
      const stand = el('button', 'ghost', 'Stand up');
      stand.addEventListener('click', () => send({ type: 'stand' }));
      const buttons = el('div', 'buttons');
      buttons.append(again, stand);
      panel.append(buttons);
    }
    box.append(panel);
    return box;
  }
  return null;
}

function render() {
  const v = view;
  if (!v) {
    app.innerHTML = '<p class="connecting">Waiting for the table...</p>';
    return;
  }
  if (selected !== null && !v.hand.some((c) => c.id === selected)) selected = null;
  if (!myTurn(v)) tradeSlot = null;

  const table = el('main', 'table');
  table.classList.toggle('my-turn', myTurn(v));
  const rivals = el('div', 'rivals');
  const me = v.seat;
  const order = me === null ? [...v.seats.keys()] : [...v.seats.keys()].map((k) => (me + 1 + k) % v.seats.length).filter((k) => k !== me);
  for (const i of order) rivals.append(renderSeat(v, i, false));
  table.append(rivals, renderCenter(v));

  const log = el('div', 'log');
  for (const e of v.events.slice(-3)) {
    const line = el('p');
    line.textContent = e.text;
    log.append(line);
  }
  table.append(log);

  if (me !== null) {
    const bottom = el('div', 'bottom');
    bottom.append(renderSeat(v, me, true), renderHand(v), renderTray(v));
    table.append(bottom);
  } else if (v.phase === 'playing') {
    table.append(el('div', 'spectating', `Spectating${v.watchers.length > 1 ? ` with ${v.watchers.length - 1} other${v.watchers.length > 2 ? 's' : ''}` : ''}`));
  }

  const overlay = renderOverlay(v);
  const toastNode = el('div', 'toast');
  app.replaceChildren(table, ...(overlay ? [overlay] : []), toastNode);
  tickClock();
}

function tickClock() {
  const clock = document.querySelector<HTMLSpanElement>('.clock');
  if (!clock || !view) return;
  const left = Math.max(0, Math.ceil((view.deadline_ms - Date.now()) / 1000));
  clock.textContent = `${left}s`;
  clock.classList.toggle('low', left <= 10);
}
setInterval(tickClock, 250);

function rectOf(spot: Spot, cardId: number): DOMRect | null {
  const key = spotKey(spot);
  const holder = document.querySelector<HTMLElement>(`[data-spot="${key}"]`);
  if (!holder) return null;
  const card = holder.querySelector<HTMLElement>(`[data-card-id="${cardId}"]`);
  return (card ?? holder).getBoundingClientRect();
}

function animate(events: GameEvent[], before: Map<string, DOMRect>) {
  let delay = 0;
  for (const event of events) {
    for (const motion of event.motions) {
      const from = before.get(`${spotKey(motion.from)}#${motion.card_id}`) ?? before.get(spotKey(motion.from));
      const to = rectOf(motion.to, motion.card_id);
      if (!from || !to) continue;
      const target = document.querySelector<HTMLElement>(`[data-spot="${spotKey(motion.to)}"] [data-card-id="${motion.card_id}"]`);
      const ghost = motion.card ? cardFace(motion.card) : cardBack();
      ghost.classList.add('ghost');
      ghost.removeAttribute('data-card-id');
      const w = 72;
      const h = 100;
      Object.assign(ghost.style, { left: `${from.left + from.width / 2 - w / 2}px`, top: `${from.top + from.height / 2 - h / 2}px` });
      document.body.append(ghost);
      const dx = to.left + to.width / 2 - (from.left + from.width / 2);
      const dy = to.top + to.height / 2 - (from.top + from.height / 2);
      const scale = Math.max(0.4, Math.min(1.2, to.height / h));
      if (target) target.style.opacity = '0';
      const anim = ghost.animate(
        [
          { transform: 'translate(0,0) scale(1) rotate(0deg)', opacity: 1 },
          { transform: `translate(${dx / 2}px,${dy / 2 - 30}px) scale(1.1) rotate(8deg)`, opacity: 1, offset: 0.5 },
          { transform: `translate(${dx}px,${dy}px) scale(${scale}) rotate(0deg)`, opacity: 1 },
        ],
        { duration: 520, delay, easing: 'cubic-bezier(.3,.7,.3,1)', fill: 'backwards' },
      );
      anim.onfinish = () => {
        ghost.remove();
        if (target) target.style.opacity = '';
      };
      delay += 140;
    }
  }
}

function snapshotRects(): Map<string, DOMRect> {
  const rects = new Map<string, DOMRect>();
  document.querySelectorAll<HTMLElement>('[data-spot]').forEach((holder) => {
    const key = holder.dataset.spot!;
    rects.set(key, holder.getBoundingClientRect());
    holder.querySelectorAll<HTMLElement>('[data-card-id]').forEach((c) => rects.set(`${key}#${c.dataset.cardId}`, c.getBoundingClientRect()));
  });
  return rects;
}

function onState(next: View | null) {
  const before = snapshotRects();
  const fresh = next ? next.events.filter((e) => e.seq > lastSeq) : [];
  const firstPaint = view === null;
  view = next;
  render();
  if (next) {
    if (!firstPaint && fresh.length && fresh.length < 6) animate(fresh, before);
    lastSeq = next.events.at(-1)?.seq ?? 0;
  }
}

try {
  activity = await connect<View | null>();
  onState(activity.state);
  activity.onState(onState);
  window.addEventListener('resize', render);
} catch (error) {
  app.innerHTML = '';
  const p = el('p', 'connecting');
  p.textContent = error instanceof Error ? error.message : 'Could not reach tawks.';
  app.append(p);
}
