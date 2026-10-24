import { connect } from '@tawks/activity-sdk';

type View = {
  me: number | null;
  phase: 'waiting' | 'playing' | 'over';
  players: { name: string; alive: boolean; holding: boolean }[];
  holder: number | null;
  held_for: number;
  can_pass_to: number[];
  sizzling: boolean;
  round: number;
  winner: number | null;
  can_join: boolean;
  can_start: boolean;
  max_players: number;
  log: string[];
};

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const status = $<HTMLHeadingElement>('status');
const hint = $<HTMLParagraphElement>('hint');
const circle = $<HTMLDivElement>('circle');
const join = $<HTMLButtonElement>('join');
const start = $<HTMLButtonElement>('start');
const log = $<HTMLUListElement>('log');

let send: (input: { type: string; to?: number }) => void = () => {};

function headline(view: View): string {
  const name = (seat: number | null) => (seat === null ? '' : view.players[seat].name);
  if (view.phase === 'waiting') return `Waiting for players (${view.players.length}/${view.max_players})`;
  if (view.phase === 'over') return view.winner === view.me ? 'You survived!' : `${name(view.winner)} survived.`;
  if (view.holder === view.me) return 'You have the potato! Pass it!';
  return `${name(view.holder)} has the potato`;
}

function render(view: View | null) {
  if (!view) return;
  status.textContent = headline(view);
  hint.textContent =
    view.phase === 'playing' ? `Round ${view.round}. Held for ${view.held_for}s.${view.sizzling ? ' It is sizzling!' : ''}` : '';
  circle.classList.toggle('sizzling', view.sizzling);
  circle.replaceChildren(
    ...view.players.map((p, i) => {
      const angle = (i / view.players.length) * 2 * Math.PI - Math.PI / 2;
      const seat = document.createElement('button');
      seat.type = 'button';
      seat.className = 'seat';
      seat.style.left = `${50 + 42 * Math.cos(angle)}%`;
      seat.style.top = `${50 + 42 * Math.sin(angle)}%`;
      seat.classList.toggle('me', i === view.me);
      seat.classList.toggle('out', !p.alive);
      seat.classList.toggle('holding', p.holding);
      const target = view.can_pass_to.includes(i);
      seat.classList.toggle('target', target);
      seat.disabled = !target;
      seat.dataset.seat = String(i);
      seat.textContent = p.name;
      const potato = document.createElement('span');
      potato.className = 'potato';
      seat.append(potato);
      if (target) seat.addEventListener('click', () => send({ type: 'pass', to: i }));
      return seat;
    }),
  );
  join.hidden = !view.can_join;
  start.hidden = !view.can_start;
  start.textContent = view.phase === 'over' ? 'Play again' : 'Light the fuse';
  log.replaceChildren(...view.log.map((line) => Object.assign(document.createElement('li'), { textContent: line })));
}

try {
  const activity = await connect<View | null>();
  send = (input) => activity.send(input).catch((error: Error) => (hint.textContent = error.message));
  render(activity.state);
  activity.onState(render);
  join.addEventListener('click', () => send({ type: 'join' }));
  start.addEventListener('click', () => send({ type: 'start' }));
} catch (error) {
  status.textContent = error instanceof Error ? error.message : 'Could not reach tawks.';
}
