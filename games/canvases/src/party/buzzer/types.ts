/** `stage` is the radius of the stage the lecterns stand on, wider for a bigger field. */
export type BuzzerSetup = { game: 'buzzer'; rounds: number; seconds: number; stage?: number };

/** `silent` is someone who never answered a question that has been revealed. */
export type BuzzerState = 'thinking' | 'wrong' | 'right' | 'silent';

export type BuzzerPlayer = {
  seat: number;
  points: number;
  state: BuzzerState;
  /** What they picked, once the answer is shown. */
  choice: number | null;
};

export type BuzzerSnap = {
  game: 'buzzer';
  time: number;
  limit: number;
  round: number;
  rounds: number;
  stage: 'asking' | 'reveal';
  question: string;
  options: string[];
  /** The right option, sent only once the question is closed. */
  answer: number | null;
  scorers: number[];
  players: BuzzerPlayer[];
};

export type BuzzerEvent =
  | { kind: 'buzz'; seat: number; right: boolean }
  | { kind: 'reveal'; answer: number; scorers: number[] };
