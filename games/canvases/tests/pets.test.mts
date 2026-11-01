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
