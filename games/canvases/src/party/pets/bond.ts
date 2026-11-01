import BOND from './bond.json' with { type: 'json' };

/** A growth stage: reached at `bond`, standing `scale` times as tall. */
export type PetStage = { id: string; name: string; bond: number; scale: number };
/** Something a pet does on command, from `stage` on. */
export type PetTrick = { id: PetTrickId; name: string; stage: number };
export type PetTrickId = 'dance' | 'hop' | 'spin' | 'flip' | 'show-off';
/** What a pet does: a trick, or reacting to being petted or poked. */
export type PetMove = PetTrickId | 'pet' | 'poke';

export const PET_STAGES = BOND.stages as PetStage[];
export const PET_TRICKS = BOND.tricks as PetTrick[];
export const PET_GAINS = BOND.gains as { source: string; points: number; label: string }[];

/** The stage reached with `bond`, as an index into PET_STAGES, as the server works it out in pet_bond.rs. */
export function stageFor(bond: number) {
  let stage = 0;
  PET_STAGES.forEach((each, index) => {
    if (bond >= each.bond) stage = index;
  });
  return stage;
}

export function stageScale(stage: number | null | undefined) {
  return (PET_STAGES[stage ?? 0] ?? PET_STAGES[PET_STAGES.length - 1]).scale;
}

/** The next stage and how much more bond it needs, or null at the last one. */
export function nextStage(bond: number) {
  const next = PET_STAGES[stageFor(bond) + 1];
  return next ? { stage: next, needed: next.bond - bond } : null;
}

export function tricksAt(stage: number) {
  return PET_TRICKS.filter((trick) => trick.stage <= stage);
}
