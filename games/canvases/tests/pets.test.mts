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
