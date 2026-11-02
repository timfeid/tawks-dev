import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {hatchedBy, PET_CATALOG} from '../src/party/pets/catalog.ts';

const fixture: [string, string][] = JSON.parse(readFileSync(new URL('../src/party/pets/hatched-by.fixture.json', import.meta.url), 'utf8'));

test('an account hatches the same pet the server gives it', () => {
  for (const [user, pet] of fixture) assert.equal(hatchedBy(user).id, pet, user);
  assert.deepEqual(new Set(fixture.map(([, pet]) => pet)), new Set(PET_CATALOG.map(pet => pet.id)), 'the fixture covers every pet');
});

test('every pet has its own 3D model', () => {
  for (const pet of PET_CATALOG) assert.ok(readFileSync(new URL(`../src/party/pets/models/${pet.id}.glb`, import.meta.url)).subarray(0, 4).equals(Buffer.from('glTF')), pet.id);
});

import * as THREE from 'three';
import {GEAR_STYLES, buildGear} from '../src/party/pets/pet-gear.ts';
import {PET_ITEMS, petSeasonNote, shownPet, wearsPetItem, withPetItem} from '../src/party/pets/store.ts';
import {hasUnownedSelection, takeOff} from '../src/party/costumes/selection.ts';

const glb = (id: string) => {
  const bytes = readFileSync(new URL(`../src/party/pets/models/${id}.glb`, import.meta.url));
  return JSON.parse(bytes.subarray(20, 20 + bytes.readUInt32LE(12)).toString('utf8'));
};

test('every pet can be adopted, and every coat has a texture for every pet', () => {
  assert.deepEqual(PET_ITEMS.filter(item => item.slot === 'pet').map(item => item.id).sort(), PET_CATALOG.map(pet => pet.id).sort());
  for (const coat of PET_ITEMS.filter(item => item.slot === 'pet-coat'))
    for (const pet of PET_CATALOG) {
      const png = readFileSync(new URL(`../src/party/pets/models/coats/${pet.id}-${coat.style}.png`, import.meta.url));
      assert.ok(png.subarray(1, 4).equals(Buffer.from('PNG')), `${pet.id} ${coat.id}`);
      assert.ok(png.equals(readFileSync(new URL(`../../../../apps/tawkathon-native/assets/pets/coats/${pet.id}-${coat.style}.png`, import.meta.url))), `the native game has the same ${pet.id} ${coat.id}`);
    }
});

test('every hat and neckwear is a built style, sized from the bones each pet really has', () => {
  for (const item of PET_ITEMS.filter(item => item.slot === 'pet-hat' || item.slot === 'pet-neck')) {
    const gear = buildGear(item.style, item.colors ?? [], color => new THREE.MeshToonMaterial({color}));
    assert.ok(gear && gear.group.children.length === GEAR_STYLES[item.style!].length, item.id);
    assert.ok(GEAR_STYLES[item.style!].every(part => part.tint < (item.colors?.length ?? 0)), `${item.id} has a colour for every part`);
    gear!.dispose();
  }
  for (const pet of PET_CATALOG) {
    const bones = new Set(glb(pet.id).nodes.map((node: {name?: string}) => node.name));
    assert.ok(bones.has(pet.fit.hat.bone) && bones.has(pet.fit.neck.bone), pet.id);
    assert.ok(pet.fit.hat.size > 0.2 && pet.fit.hat.size < 0.5 && pet.fit.neck.size.every(size => size > 0.5 && size < 0.9), pet.id);
  }
});

test('putting pet gear on and taking it off, with the hatched pet out by default', () => {
  const look = withPetItem(withPetItem({}, 'pet-hat-party'), 'pet-fizz', 'pet-mochi');
  assert.deepEqual(look, {hat: 'pet-hat-party', id: 'pet-fizz'});
  assert.equal(shownPet('pet-mochi', look)!.id, 'pet-fizz');
  assert.equal(shownPet('pet-mochi', {})!.hat, null);
  assert.ok(wearsPetItem({}, 'pet-mochi', 'pet-mochi'), 'the hatched pet is out until another is');
  assert.ok(!wearsPetItem(look, 'pet-mochi', 'pet-mochi'));
  assert.equal(withPetItem(look, 'pet-mochi', 'pet-mochi').id, null, 'choosing the hatched pet goes back to it');
  assert.equal(takeOff({color: 'blip', pet: look}, 'pet-fizz').pet!.id, null, 'taking an adopted pet off brings the hatched one back');
  assert.deepEqual(takeOff({color: 'blip', hat: 'hat-cap', pet: look}, 'pet-hat-party'), {color: 'blip', hat: 'hat-cap', pet: {hat: null, id: 'pet-fizz'}});
  assert.ok(hasUnownedSelection({color: 'blip', pet: look}, ['pet-fizz'], []), 'the party hat is not owned');
  assert.ok(!hasUnownedSelection({color: 'blip', pet: look}, ['pet-fizz', 'pet-hat-party'], []));
});

test('seasonal pet gear follows the head seasons', () => {
  const santa = PET_ITEMS.find(item => item.id === 'pet-hat-santa')!;
  assert.equal(petSeasonNote(santa, new Date('2026-11-01T12:00:00Z')), 'Back Nov 15');
  assert.equal(petSeasonNote(santa, new Date('2026-12-24T12:00:00Z')), 'Until Dec 31');
  assert.equal(petSeasonNote(PET_ITEMS.find(item => item.id === 'pet-hat-party')!), '');
  assert.ok(PET_ITEMS.every(item => !item.description.includes('—') && !item.name.includes('—')));
});

import {PET_STAGES, PET_TRICKS, nextStage, stageFor, stageScale, tricksAt} from '../src/party/pets/bond.ts';
import {MOVE_SECONDS, moveAt} from '../src/party/pets/moves.ts';

test('a pet grows through its stages with bond and learns tricks as it does', () => {
  assert.equal(stageFor(0), 0);
  assert.equal(stageFor(PET_STAGES[1].bond - 1), 0);
  assert.equal(stageFor(PET_STAGES[1].bond), 1);
  assert.equal(stageFor(1e9), PET_STAGES.length - 1);
  assert.equal(stageScale(null), 1);
  assert.ok(stageScale(4) > stageScale(1));
  assert.deepEqual(nextStage(0), { stage: PET_STAGES[1], needed: PET_STAGES[1].bond });
  assert.equal(nextStage(1e9), null);
  assert.deepEqual(tricksAt(0).map(trick => trick.id), ['dance', 'hop']);
  assert.ok(tricksAt(PET_STAGES.length - 1).length === PET_TRICKS.length, 'every trick is learned by the last stage');
});

test('every move plays out and settles back where it started', () => {
  for (const move of [...PET_TRICKS.map(trick => trick.id), 'pet', 'poke'] as const) {
    const length = MOVE_SECONDS[move];
    assert.ok(length > 0, move);
    assert.deepEqual(moveAt(move, length), { lift: 0, spin: 0, flip: 0, squash: 0 }, `${move} ends at rest`);
    const middle = moveAt(move, length / 3);
    assert.ok(middle.lift !== 0 || middle.spin !== 0 || middle.flip !== 0 || middle.squash !== 0, `${move} moves`);
  }
  assert.ok(Math.abs(moveAt('spin', MOVE_SECONDS.spin - 1e-3).spin - Math.PI * 2) < 0.01, 'a spin goes all the way round');
});

import {FX_BURSTS, FX_COSMETICS, FX_MAX, FxSim, ambienceFor, fade} from '../src/party/pets/fx-sim.ts';

const seeded = (seed = 7) => () => ((seed = (seed * 1664525 + 1013904223) >>> 0) >>> 8) / 2 ** 24;

test('every move, stage-up and bond gain has a burst that plays out', () => {
  for (const name of ['pet', 'poke', 'dance', 'hop', 'spin', 'flip', 'show-off', 'stage-up', 'gain'] as const) {
    const sim = new FxSim(seeded());
    sim.burst(name);
    let most = sim.particles.length;
    for (let i = 0; i < 120; i++) { sim.step(1 / 30); most = Math.max(most, sim.particles.length); }
    assert.ok(most > 0, `${name} makes particles`);
    assert.equal(sim.particles.length, 0, `${name} ends`);
  }
});

test('ambience grows with the stage, every store effect has particles, and the cap holds', () => {
  assert.equal(ambienceFor(0, null).length, 0, 'a hatchling is plain');
  assert.equal(ambienceFor(2, null).length, 0);
  assert.ok(ambienceFor(3, null).length > 0 && ambienceFor(4, null).length > ambienceFor(3, null).length);
  for (const item of PET_ITEMS.filter(item => item.slot === 'pet-effect')) assert.ok(FX_COSMETICS[item.style!]?.length, item.id);
  const sim = new FxSim(seeded());
  sim.setAmbient(ambienceFor(4, 'stardust'));
  for (let i = 0; i < 300; i++) { sim.step(1 / 30, 1); if (i % 10 === 0) sim.burst('show-off'); }
  assert.ok(sim.particles.length <= FX_MAX && sim.particles.length > 0);
  assert.ok(sim.particles.every(p => fade(p) >= 0 && fade(p) <= 1));
  assert.ok(Object.values(FX_BURSTS).flat().every(e => e.count && !e.rate), 'bursts count, ambience streams');
});

test('a pet effect goes on the pet that is out, and each pet keeps its own', () => {
  let look = withPetItem({}, 'pet-fx-halo', 'pet-mochi');
  assert.deepEqual(look.effects, { 'pet-mochi': 'pet-fx-halo' });
  look = withPetItem(withPetItem(look, 'pet-fizz', 'pet-mochi'), 'pet-fx-hearts', 'pet-mochi');
  assert.deepEqual(look.effects, { 'pet-mochi': 'pet-fx-halo', 'pet-fizz': 'pet-fx-hearts' });
  assert.equal(shownPet('pet-mochi', look)!.effect, 'hearts');
  assert.ok(wearsPetItem(look, 'pet-fx-hearts', 'pet-mochi') && !wearsPetItem(look, 'pet-fx-halo', 'pet-mochi'));
  assert.equal(shownPet('pet-mochi', { ...look, id: null })!.effect, 'halo');
});
