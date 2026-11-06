export type Pass = { ws_url: string; token: string; lobby_id: string };

export type Place = { name: string; lat: number; lon: number };
export type Found = Place & { label: string };

export type Nomination = { id: number; place: Place; by: string; votes: string[] };
export type Driver = { user_id: string; name: string; avatar_url: string; color: string };

export type RoomView = {
  host: string | null;
  place: Place | null;
  epoch: number;
  hour: number;
  nominations: Nomination[];
  drivers: Driver[];
};

/** Heading 0 is north and grows clockwise; f bit 1 is headlights, bit 2 braking. */
export type Pose = { x: number; z: number; h: number; s: number; st: number; f: number };

export type Action =
  | { kind: 'set_place'; place: Place }
  | { kind: 'nominate'; place: Place }
  | { kind: 'vote'; id: number }
  | { kind: 'unvote' }
  | { kind: 'set_hour'; hour: number };

export type ClientMsg =
  | { type: 'hello'; token: string; place?: Place | null }
  | { type: 'ping'; t: number }
  | { type: 'pose'; pose: Pose }
  | { type: 'honk' }
  | { type: 'act'; action: Action }
  | { type: 'search'; q: string; near?: [number, number] | null }
  | { type: 'heist_open' }
  | { type: 'heist_close' }
  | { type: 'heist'; msg: import('../heist/protocol').HeistMsg };

export type ServerMsg =
  | { type: 'welcome'; user_id: string; attribution: string }
  | { type: 'room'; room: RoomView }
  | { type: 'poses'; cars: (Pose & { id: string })[] }
  | { type: 'honk'; id: string }
  | { type: 'results'; q: string; places: Found[] }
  | { type: 'pong'; t: number }
  | { type: 'error'; message: string }
  | { type: 'heist'; view: import('../heist/protocol').HeistView | null }
  | { type: 'heist_world'; world: import('../heist/protocol').HeistWorld }
  | { type: 'heist_poses'; poses: import('../heist/protocol').HeistPoses }
  | { type: 'heist_event'; event: import('../heist/protocol').HeistEvent };
