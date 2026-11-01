import type { BuzzerSnap } from './types';

export type PanelPlayer = { seat: number; name: string; color: string; points: number; state: string; me: boolean };

export type PanelState = {
  /** Countdown shows the frame without the question; playing shows it all. */
  stage: 'hidden' | 'countdown' | 'playing';
  snapshot: BuzzerSnap | null;
  /** Whether the viewer can answer, as opposed to watching. */
  answering: boolean;
  picked: number | null;
  highlight: number;
  players: PanelPlayer[];
  note: string;
};

const STYLE_ID = 'tawks-buzzer-panel';
const KEYS = ['1', '2', '3', '4'];

const CSS = `
.buzzer-panel { position: absolute; inset: 0; z-index: 20; pointer-events: none; display: flex; flex-direction: column;
  align-items: center; justify-content: flex-start; gap: 12px; padding: 4.75rem 16px 16px; font-family: inherit; color: #fff; }
.buzzer-panel[hidden] { display: none; }
.buzzer-card { width: 100%; max-width: 44rem; border-radius: 1rem; border: 1px solid rgb(255 255 255 / 0.15);
  background: rgb(15 23 42 / 0.82); backdrop-filter: blur(8px); padding: 14px 18px; text-align: center;
  box-shadow: 0 10px 30px rgb(2 6 23 / 0.35); }
.buzzer-count { font-size: 11px; font-weight: 800; letter-spacing: 0.16em; text-transform: uppercase; color: rgb(253 230 138); }
.buzzer-question { margin-top: 6px; font-size: clamp(17px, 2.6vw, 26px); font-weight: 800; line-height: 1.25; }
.buzzer-note { margin-top: 8px; min-height: 1.2em; font-size: 13px; font-weight: 600; color: rgb(255 255 255 / 0.75); }
.buzzer-grid { width: 100%; max-width: 44rem; display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.buzzer-grid[hidden] { display: none; }
.buzzer-option { pointer-events: auto; display: flex; align-items: center; gap: 10px; min-height: 58px; padding: 10px 14px;
  border-radius: 0.9rem; border: 2px solid rgb(255 255 255 / 0.18); background: rgb(30 41 59 / 0.88); color: #fff;
  font: inherit; font-size: clamp(14px, 1.9vw, 18px); font-weight: 700; text-align: left; cursor: pointer;
  transition: transform 80ms ease, background-color 120ms ease, border-color 120ms ease, opacity 120ms ease; touch-action: manipulation; }
.buzzer-option:disabled { cursor: default; }
.buzzer-option:not(:disabled):hover, .buzzer-option.is-highlight { border-color: rgb(253 224 71); }
.buzzer-option:not(:disabled):active { transform: scale(0.98); }
.buzzer-option.is-picked { background: rgb(79 70 229 / 0.92); border-color: rgb(199 210 254); }
.buzzer-option.is-right { background: rgb(5 150 105 / 0.95); border-color: rgb(167 243 208); }
.buzzer-option.is-wrong { background: rgb(190 18 60 / 0.92); border-color: rgb(254 205 211); }
.buzzer-option.is-dim { opacity: 0.45; }
.buzzer-key { flex: none; display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px;
  background: rgb(255 255 255 / 0.14); font-size: 13px; font-weight: 900; }
.buzzer-scores { display: flex; flex-wrap: wrap; justify-content: center; gap: 6px; }
.buzzer-chip { display: flex; align-items: center; gap: 6px; padding: 4px 10px; border-radius: 999px;
  background: rgb(15 23 42 / 0.75); border: 1px solid rgb(255 255 255 / 0.12); font-size: 12px; font-weight: 700; }
.buzzer-chip.is-me { border-color: rgb(253 224 71 / 0.7); }
.buzzer-chip.is-wrong { border-color: rgb(244 63 94 / 0.8); }
.buzzer-chip.is-right { border-color: rgb(52 211 153 / 0.9); }
.buzzer-dot { width: 10px; height: 10px; border-radius: 999px; }
.buzzer-points { font-variant-numeric: tabular-nums; color: rgb(253 230 138); }
@media (max-height: 560px) {
  .buzzer-panel { padding-top: 3.5rem; gap: 8px; }
  .buzzer-option { min-height: 46px; }
}
`;

let styleUsers = 0;

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, parent?: HTMLElement) {
  const node = document.createElement(tag);
  node.className = className;
  parent?.appendChild(node);
  return node;
}

function setText(node: HTMLElement, text: string) {
  if (node.textContent !== text) node.textContent = text;
}

function setClass(node: HTMLElement, className: string) {
  if (node.className !== className) node.className = className;
}

/**
 * The question and its four answers, laid over the arena as plain buttons so a
 * tap, a click, a number key or a gamepad all answer the same way.
 */
export class BuzzerPanel {
  private root: HTMLDivElement;
  private count: HTMLDivElement;
  private question: HTMLDivElement;
  private note: HTMLDivElement;
  private grid: HTMLDivElement;
  private options: { button: HTMLButtonElement; label: HTMLSpanElement }[];
  private scores: HTMLDivElement;
  private scoresKey = '';

  constructor(parent: HTMLElement, onPick: (choice: number) => void) {
    if (styleUsers++ === 0 && !document.getElementById(STYLE_ID)) {
      const style = document.createElement('style');
      style.id = STYLE_ID;
      style.textContent = CSS;
      document.head.appendChild(style);
    }
    this.root = el('div', 'buzzer-panel', parent);
    this.root.hidden = true;
    const card = el('div', 'buzzer-card', this.root);
    this.count = el('div', 'buzzer-count', card);
    this.question = el('div', 'buzzer-question', card);
    this.question.setAttribute('aria-live', 'polite');
    this.note = el('div', 'buzzer-note', card);
    this.grid = el('div', 'buzzer-grid', this.root);
    this.options = KEYS.map((key, index) => {
      const button = el('button', 'buzzer-option', this.grid);
      button.type = 'button';
      el('span', 'buzzer-key', button).textContent = key;
      const label = el('span', '', button);
      button.addEventListener('click', () => onPick(index));
      return { button, label };
    });
    this.scores = el('div', 'buzzer-scores', this.root);
  }

  render(state: PanelState) {
    const snap = state.snapshot;
    const shown = state.stage !== 'hidden' && !!snap;
    if (this.root.hidden === shown) this.root.hidden = !shown;
    if (!shown || !snap) return;

    setText(this.count, `Question ${snap.round + 1} of ${snap.rounds}`);
    const live = state.stage === 'playing';
    setText(this.question, live ? snap.question : 'Get ready');
    setText(this.note, state.note);
    if (this.grid.hidden === live) this.grid.hidden = !live;

    const asking = snap.stage === 'asking';
    this.options.forEach(({ button, label }, index) => {
      setText(label, snap.options[index] ?? '');
      const open = live && asking && state.answering && state.picked === null;
      if (button.disabled === open) button.disabled = !open;
      let className = 'buzzer-option';
      if (!asking && snap.answer === index) className += ' is-right';
      else if (!asking && state.picked === index) className += ' is-wrong';
      else if (asking && state.picked === index) {
        const mine = state.players.find((player) => player.me);
        className += mine?.state === 'wrong' ? ' is-wrong' : ' is-picked';
      } else if (state.picked !== null || !asking) className += ' is-dim';
      else if (open && state.highlight === index) className += ' is-highlight';
      setClass(button, className);
    });

    const scoresKey = JSON.stringify(state.players);
    if (scoresKey !== this.scoresKey) {
      this.scoresKey = scoresKey;
      this.scores.replaceChildren(
        ...state.players.map((player) => {
          const chip = el('span', `buzzer-chip${player.me ? ' is-me' : ''}${player.state === 'wrong' || player.state === 'right' ? ` is-${player.state}` : ''}`);
          el('span', 'buzzer-dot', chip).style.background = player.color;
          el('span', '', chip).textContent = player.name;
          el('span', 'buzzer-points', chip).textContent = String(player.points);
          return chip;
        }),
      );
    }
  }

  dispose() {
    this.root.remove();
    if (--styleUsers === 0) document.getElementById(STYLE_ID)?.remove();
  }
}
