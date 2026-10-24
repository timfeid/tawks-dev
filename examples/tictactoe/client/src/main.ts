import { connect } from '@tawks/activity-sdk';

type View = {
  seat: number | null;
  players: { name: string; mark: 'X' | 'O' }[];
  board: ('X' | 'O' | null)[];
  turn: number;
  winner: number | null;
  line: number[] | null;
  draw: boolean;
};

const status = document.querySelector<HTMLParagraphElement>('#status')!;
const board = document.querySelector<HTMLDivElement>('#board')!;
const players = document.querySelector<HTMLParagraphElement>('#players')!;
const reset = document.querySelector<HTMLButtonElement>('#reset')!;

const cells = Array.from({ length: 9 }, (_, i) => {
  const cell = document.createElement('button');
  cell.type = 'button';
  cell.className = 'cell';
  cell.setAttribute('aria-label', `Square ${i + 1}`);
  board.append(cell);
  return cell;
});

function describe(view: View): string {
  if (view.players.length < 2) return 'Waiting for a second player.';
  if (view.winner !== null) {
    return view.winner === view.seat ? 'You win!' : `${view.players[view.winner].name} wins.`;
  }
  if (view.draw) return "It's a draw.";
  if (view.seat === null) return `${view.players[view.turn].name} to move.`;
  return view.turn === view.seat ? 'Your move.' : `Waiting for ${view.players[view.turn].name}.`;
}

function render(view: View | null) {
  if (!view) return;
  status.textContent = describe(view);
  const over = view.winner !== null || view.draw;
  const myTurn = !over && view.players.length === 2 && view.seat === view.turn;
  view.board.forEach((mark, i) => {
    const cell = cells[i];
    cell.textContent = mark ?? '';
    cell.dataset.mark = mark ?? '';
    cell.classList.toggle('won', view.line?.includes(i) ?? false);
    cell.disabled = !myTurn || mark !== null;
  });
  players.textContent = view.players.map((p) => `${p.mark} ${p.name}`).join('  vs  ');
  reset.hidden = !over || view.seat === null;
}

try {
  const activity = await connect<View | null>();
  render(activity.state);
  activity.onState(render);

  const send = (input: Parameters<typeof activity.send>[0]) =>
    activity.send(input).catch((error: Error) => (status.textContent = error.message));
  cells.forEach((cell, i) => cell.addEventListener('click', () => send({ type: 'move', cell: i })));
  reset.addEventListener('click', () => send({ type: 'reset' }));
} catch (error) {
  status.textContent = error instanceof Error ? error.message : 'Could not reach tawks.';
}
