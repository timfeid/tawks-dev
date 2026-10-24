import type { MapCity } from './board';

export type Card =
  | 'red'
  | 'orange'
  | 'yellow'
  | 'green'
  | 'blue'
  | 'purple'
  | 'black'
  | 'white'
  | 'locomotive';
export type RouteView = {
  id: number;
  from: string;
  to: string;
  length: number;
  color: Card | 'any';
  twin: number | null;
  owner: string | null;
  points: number;
};
export type PlayerView = {
  id: string;
  name: string;
  avatar_url: string;
  is_ai: boolean;
  is_host: boolean;
  is_active: boolean;
  seat: number;
  trains: number;
  hand_size: number;
  ticket_count: number;
  score: number;
  claimed: number[];
};
export type TicketView = {
  id: number;
  from: string;
  to: string;
  from_name: string;
  to_name: string;
  points: number;
  completed: boolean;
};
export type Game = {
  stage: 'lobby' | 'playing' | 'finished';
  phase: 'start' | 'drew_one' | 'choosing_tickets';
  round: number;
  host_id: string | null;
  active_player_id: string | null;
  final_round: boolean;
  players: PlayerView[];
  viewer: {
    id: string;
    seated: boolean;
    is_host: boolean;
    is_active: boolean;
    hand: { card: Card; count: number }[];
    tickets: TicketView[];
    ticket_offer: TicketView[];
    must_keep: number;
    claimable: { route: number; options: { card: Card; locomotives: number }[] }[];
    can_draw_deck: boolean;
    can_draw_tickets: boolean;
  } | null;
  face_up: Card[];
  deck_count: number;
  ticket_deck_count: number;
  cities: MapCity[];
  routes: RouteView[];
  winner_id: string | null;
  standings: { id: string; name: string; score: number; tickets_done: number; tickets_missed: number; longest_path: boolean }[];
  log: any[];
};
