import type { BoutModule } from '@tawks/bout-sdk';

type Loader = () => Promise<{ default: unknown }>;

const shooter: Loader = () => import('./shooter');
const floor: Loader = () => import('./floor');
const battle: Loader = () => import('./battle');
const race: Loader = () => import('./race');

/** Each bout's renderer, loaded the first time someone picks it. */
export const BOUT_RENDERERS: Record<string, Loader> = {
  bumper: () => import('@tawks/example-bumper-balls'),
  shooter,
  target: shooter,
  popgun: shooter,
  ricochet: shooter,
  retake: shooter,
  speedball: shooter,
  dodgeball: () => import('./dodgeball'),
  hill: () => import('./hill'),
  memory: () => import('./memory'),
  rising: () => import('./rising'),
  buzzer: () => import('./buzzer'),
  minecart: () => import('./minecart'),
  relay: () => import('./relay'),
  ghosts: () => import('./ghosts'),
  crumble: floor,
  hotpotato: floor,
  balloon: floor,
  sumo: floor,
  ledge: () => import('./ledge'),
  coins: floor,
  tug: floor,
  tank: battle,
  titan: battle,
  race,
  redlight: race,
};

const loaded = new Map<string, Promise<BoutModule>>();

export function loadBout(key: string): Promise<BoutModule> | null {
  const loader = BOUT_RENDERERS[key];
  if (!loader) return null;
  let module = loaded.get(key);
  if (!module) {
    module = loader().then((m) => m.default as BoutModule);
    loaded.set(key, module);
  }
  return module;
}
