/**
 * One pet and its particles at a moment, for frame sheets: `?pet=pet-fizz&stage=4&effect=halo&play=flip&t=0.4`.
 * `play` is a move, `stage-up` or `gain`; the pet's ambience has already run 1.5 s when it starts.
 * The random numbers are seeded, so a frame comes out the same every time. `window.ready` turns true once drawn.
 */
import * as THREE from 'three';
import { buildPet } from '../src/party/pets/pet-model';
import type { PetMove } from '../src/party/pets/bond';

let seed = 11;
Math.random = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) >>> 8) / 2 ** 24;

const query = new URLSearchParams(location.search);
const play = query.get('play');
const t = Number(query.get('t') ?? 0);
const canvas = document.querySelector('canvas')!;
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setSize(320, 320, false);
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.2;
const scene = new THREE.Scene();
scene.background = new THREE.Color('#161b26');
scene.add(new THREE.HemisphereLight('#ffffff', '#64748b', 2.4));
const light = new THREE.DirectionalLight('#fff4db', 2.8);
light.position.set(-3, 5, 4);
scene.add(light);
const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 20);
camera.position.set(0, 0.7, 2.6);
camera.lookAt(0, 0.45, 0);
const floor = new THREE.Mesh(new THREE.CircleGeometry(0.7, 48), new THREE.MeshStandardMaterial({ color: '#223049' }));
floor.rotation.x = -Math.PI / 2;
scene.add(floor);

const pet = buildPet(query.get('pet') ?? 'pet-fizz', 0.6, {
  coat: null,
  hat: null,
  neck: null,
  stage: Number(query.get('stage') ?? 0),
  effect: query.get('effect'),
});
pet.group.rotation.y = -0.4;
scene.add(pet.group);
await pet.ready;
pet.animate(0);
if (play === 'stage-up') pet.celebrate('stage-up');
else if (play === 'gain') pet.celebrate('gain', 10);
else if (play) pet.perform(play as PetMove);
for (let at = 1 / 30; at <= t + 1e-6; at += 1 / 30) pet.animate(at);
renderer.render(scene, camera);
(window as unknown as { ready: boolean }).ready = true;
