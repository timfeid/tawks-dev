import type { HeistEvent, HeistMsg, HeistPoses, HeistView, HeistWorld } from '../heist/protocol';
import type { Action, ClientMsg, Found, Pass, Place, Pose, RoomView, ServerMsg } from './types';

const PLACE_MEMORY = 'outroads.place';

/** The lobby's drive socket. The room is the server's; poses and honks go straight to listeners. */
export class DriveConnection {
  status = $state<'connecting' | 'online' | 'offline'>('connecting');
  room = $state<RoomView | null>(null);
  me = $state<string | null>(null);
  attribution = $state('');
  error = $state<string | null>(null);
  results = $state<Found[]>([]);
  searching = $state(false);
  ping = $state(0);

  /** The lobby's Port Run, or null in free roam. */
  heist = $state<HeistView | null>(null);
  heistWorld = $state<HeistWorld | null>(null);
  onHeistPoses: (poses: HeistPoses) => void = () => {};
  onHeistEvent: (event: HeistEvent) => void = () => {};

  onPoses: (cars: (Pose & { id: string })[]) => void = () => {};
  onHonk: (id: string) => void = () => {};

  private socket: WebSocket | null = null;
  private retry = 0;
  private closed = false;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private errorTimer: ReturnType<typeof setTimeout> | null = null;
  private lastQuery = '';

  constructor(
    private pass: Pass,
    private renewPass: () => Promise<Pass>,
  ) {
    this.open();
  }

  /** Elevation tiles come through the game server, which adds the headers a frame without an origin needs. */
  get terrainUrl(): string {
    return this.pass.ws_url.replace(/^ws/, 'http').replace(/\/ws$/, '/terrain');
  }

  get host() {
    return !!this.room && this.room.host === this.me;
  }

  private send(msg: ClientMsg) {
    if (this.socket?.readyState !== WebSocket.OPEN) return false;
    this.socket.send(JSON.stringify(msg));
    return true;
  }

  act(action: Action) {
    if (!this.send({ type: 'act', action })) this.flash('Reconnecting...');
  }

  pose(pose: Pose) {
    this.send({ type: 'pose', pose });
  }

  heistOpen(open: boolean) {
    if (!this.send(open ? { type: 'heist_open' } : { type: 'heist_close' })) this.flash('Reconnecting...');
  }

  heistSend(msg: HeistMsg) {
    this.send({ type: 'heist', msg });
  }

  honk() {
    this.send({ type: 'honk' });
  }

  search(q: string, near: [number, number] | null) {
    this.lastQuery = q;
    this.searching = this.send({ type: 'search', q, near });
  }

  flash(message: string) {
    this.error = message;
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), 4_000);
  }

  close() {
    this.closed = true;
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.socket?.close();
  }

  private remembered(): Place | null {
    try {
      const raw = sessionStorage.getItem(`${PLACE_MEMORY}.${this.pass.lobby_id}`);
      return raw ? (JSON.parse(raw) as Place) : null;
    } catch {
      return null;
    }
  }

  private remember(place: Place | null) {
    try {
      if (place) sessionStorage.setItem(`${PLACE_MEMORY}.${this.pass.lobby_id}`, JSON.stringify(place));
    } catch {
      /* Frames have no storage; the room in memory is enough. */
    }
  }

  private open() {
    if (this.closed) return;
    this.status = 'connecting';
    const socket = new WebSocket(this.pass.ws_url);
    this.socket = socket;
    socket.onopen = () => {
      this.retry = 0;
      const place = this.room?.place ?? this.remembered();
      socket.send(JSON.stringify({ type: 'hello', token: this.pass.token, place } satisfies ClientMsg));
      if (this.pingTimer) clearInterval(this.pingTimer);
      this.pingTimer = setInterval(() => {
        if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: 'ping', t: performance.now() } satisfies ClientMsg));
      }, 5_000);
    };
    socket.onmessage = (event) => this.receive(JSON.parse(event.data) as ServerMsg);
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.status = 'offline';
      if (this.closed) return;
      setTimeout(() => this.open(), Math.min(10_000, 400 * 2 ** this.retry++));
    };
  }

  private receive(message: ServerMsg) {
    switch (message.type) {
      case 'welcome':
        this.me = message.user_id;
        this.attribution = message.attribution;
        this.status = 'online';
        break;
      case 'room':
        this.room = message.room;
        this.remember(message.room.place);
        break;
      case 'poses':
        this.onPoses(message.cars);
        break;
      case 'honk':
        this.onHonk(message.id);
        break;
      case 'results':
        if (message.q === this.lastQuery) {
          this.results = message.places;
          this.searching = false;
        }
        break;
      case 'heist':
        this.heist = message.view;
        if (!message.view) this.heistWorld = null;
        break;
      case 'heist_world':
        this.heistWorld = message.world;
        break;
      case 'heist_poses':
        this.onHeistPoses(message.poses);
        break;
      case 'heist_event':
        this.onHeistEvent(message.event);
        break;
      case 'pong':
        this.ping = Math.round(performance.now() - message.t);
        break;
      case 'error':
        if (message.message === 'pass_expired') {
          this.renewPass()
            .then((pass) => {
              this.pass = pass;
              this.socket?.close();
            })
            .catch(() => this.flash('Your seat expired. Reopen the game.'));
          return;
        }
        this.searching = false;
        this.flash(message.message);
        break;
    }
  }
}
