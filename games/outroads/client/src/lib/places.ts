import type { Place } from './types';

export type Preset = Place & { blurb: string; kind: 'city' | 'town' };

export const PRESETS: Preset[] = [
  { name: 'Times Square', lat: 40.758, lon: -73.9855, blurb: 'Manhattan, New York', kind: 'city' },
  { name: 'Exchange Place', lat: 40.7163, lon: -74.0327, blurb: 'Jersey City, New Jersey', kind: 'city' },
  { name: 'The Loop', lat: 41.8819, lon: -87.6278, blurb: 'Chicago, Illinois', kind: 'city' },
  { name: 'Market Street', lat: 37.7897, lon: -122.4011, blurb: 'San Francisco, California', kind: 'city' },
  { name: 'Shibuya Crossing', lat: 35.6595, lon: 139.7005, blurb: 'Tokyo, Japan', kind: 'city' },
  { name: 'Arc de Triomphe', lat: 48.8738, lon: 2.295, blurb: 'Paris, France', kind: 'city' },
  { name: 'Trafalgar Square', lat: 51.508, lon: -0.1281, blurb: 'London, England', kind: 'city' },
  { name: 'Colosseum', lat: 41.8902, lon: 12.4922, blurb: 'Rome, Italy', kind: 'city' },
  { name: 'Downtown Dubai', lat: 25.1972, lon: 55.2744, blurb: 'Dubai, United Arab Emirates', kind: 'city' },
  { name: 'Circular Quay', lat: -33.8615, lon: 151.2108, blurb: 'Sydney, Australia', kind: 'city' },
  { name: 'Cape May', lat: 38.9351, lon: -74.906, blurb: 'A shore town in New Jersey', kind: 'town' },
  { name: 'Hallstatt', lat: 47.5622, lon: 13.6493, blurb: 'A lakeside village in Austria', kind: 'town' },
];

/** About a hundred meters, so "near me" never gives away a front door. */
export function roughly(lat: number, lon: number): { lat: number; lon: number } {
  return { lat: Math.round(lat * 1000) / 1000, lon: Math.round(lon * 1000) / 1000 };
}
