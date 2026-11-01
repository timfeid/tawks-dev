export type BallSnap = {
  seat: number;
  x: number;
  z: number;
  y: number;
  vx: number;
  vz: number;
  state: 'rolling' | 'falling' | 'out';
};

export type BumperSnap = {
  game: 'bumper';
  time: number;
  limit: number;
  radius: number;
  ball_radius: number;
  balls: BallSnap[];
};

/** The platform's full and fully closed size, wider for a bigger field. */
export type BumperSetup = {
  game: 'bumper';
  radius: number;
  min_radius: number;
};

export type BumperEvent =
  | { kind: 'bump'; a: number; b: number; strength: number; x: number; z: number }
  | { kind: 'fall'; seat: number }
  | { kind: 'splash'; seat: number; x: number; z: number };
