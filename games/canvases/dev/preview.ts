/**
 * The party board on its own, with no server: a fixture from
 * `cargo run -p party --example board_json [map]` and the same stage the lobby uses.
 *
 * Drag to orbit, scroll to zoom, 1-4 for set shots, R to replay a roll.
 * `?map=volcano` picks a fixture and `?shot=2` the opening shot.
 */
import * as THREE from 'three';
import { PartyBoard } from '../src/party/board/scene';
import { createPartyStage } from '../src/party/stage';
import type { BoardView, CharacterDef, Member } from '../src/party/protocol';
import island from './board.json';

/** Other boards come from `board-<map>.json`, picked with `?map=volcano`. */
const FIXTURES = import.meta.glob('./board-*.json', { eager: true, import: 'default' }) as Record<string, unknown>;
const wanted = new URLSearchParams(location.search).get('map');
const view = (FIXTURES[`./board-${wanted}.json`] ?? island) as unknown as BoardView;

const ROSTER: CharacterDef[] = [
  { id: 'sunny', name: 'Sunny', body: '#ffd23f', accent: '#f08c00', belly: '#fff3c4', feature: 'long_ears' },
  { id: 'bolt', name: 'Bolt', body: '#ff5f6d', accent: '#c92a2a', belly: '#ffd9dc', feature: 'visor' },
  { id: 'vex', name: 'Vex', body: '#9b5de5', accent: '#6741d9', belly: '#e5d4ff', feature: 'horns' },
  { id: 'rosie', name: 'Rosie', body: '#4cc9f0', accent: '#1971c2', belly: '#d0f0ff', feature: 'long_ears' },
] as CharacterDef[];

const members: Member[] = view.tokens.map((token, index) => ({
  user_id: `u${index}`,
  name: ROSTER[index % ROSTER.length].name,
  avatar_url: '',
  character: ROSTER[index % ROSTER.length].id,
  seat: token.seat,
  is_bot: index > 0,
  connected: true,
  wins: 0,
  in_round: true,
}));

const canvas = document.querySelector('canvas')!;
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.05;

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 400);
const stage = createPartyStage(scene, { mobile: false });
for (const islet of stage.islands) islet.visible = false;
const party = new PartyBoard(scene);
// The island is far wider than an arena, so the sun covers all of it.
Object.assign(stage.sun.shadow.camera, { left: -46, right: 46, top: 46, bottom: -46, near: 1, far: 140 });
stage.sun.shadow.mapSize.set(4096, 4096);
stage.sun.shadow.normalBias = 0.09;
stage.sun.position.set(-26, 54, 30);
stage.sun.shadow.camera.updateProjectionMatrix();
party.sync(view, members, new Map(ROSTER.map((c) => [c.id, c])));

const bounds = party.bounds();
/** What the orbit turns around; moved by focusSpace() while poking at things. */
const focus = bounds.center.clone();

function focusSpace(id: number) {
  const space = view.spaces.find((one) => one.id === id);
  if (!space) return;
  focus.set(space.x, party.groundAt(space.x, space.z), space.z);
}
/** Set shots: whole island, over a shoulder, low and close, and the ridge. */
const SHOTS = [
  { distance: 132, pitch: 58, yaw: 0, look: 0 },
  { distance: 34, pitch: 42, yaw: 0.6, look: 1 },
  { distance: 16, pitch: 18, yaw: 2.2, look: 1.4 },
  { distance: 52, pitch: 26, yaw: 3.9, look: 2 },
];
let shot = { ...(SHOTS[Number(new URLSearchParams(location.search).get('shot')) - 1] ?? SHOTS[0]) };
let cinematic = false;
let dragging: { x: number; y: number } | null = null;

canvas.addEventListener('pointerdown', (event) => (dragging = { x: event.clientX, y: event.clientY }));
addEventListener('pointerup', () => (dragging = null));
addEventListener('pointermove', (event) => {
  if (!dragging) return;
  shot.yaw -= (event.clientX - dragging.x) * 0.006;
  shot.pitch = Math.max(5, Math.min(85, shot.pitch - (event.clientY - dragging.y) * 0.25));
  dragging = { x: event.clientX, y: event.clientY };
});
canvas.addEventListener('wheel', (event) => {
  shot.distance = Math.max(8, Math.min(160, shot.distance * (1 + Math.sign(event.deltaY) * 0.08)));
  event.preventDefault();
});
addEventListener('keydown', (event) => {
  const index = Number(event.key) - 1;
  if (SHOTS[index]) { shot = { ...SHOTS[index] }; cinematic = false; }
  if (event.key.toLowerCase() === 'r') replayRoll();
  if (event.key.toLowerCase() === 'i') throwItem();
});

/** Walks a token a few spaces, so hops over the hills can be watched. */
function replayRoll() {
  cinematic = true;
  const seat = view.turn_seat;
  const token = view.tokens.find((t) => t.seat === seat);
  if (!token) return;
  const path: number[] = [];
  let at = token.space;
  for (let i = 0; i < 5; i++) {
    const space = view.spaces.find((s) => s.id === at);
    if (!space) break;
    at = space.next[0];
    path.push(at);
  }
  token.space = at;
  view.log = [
    { kind: 'roll', seat, roll: path.length, dice: [path.length], cursed: false, golden: false },
    { kind: 'move', seat, path },
  ];
  party.sync(view, members, new Map(ROSTER.map((c) => [c.id, c])));
}

/** Fires a bomb from the player in turn at the furthest rival, for the camera. */
function throwItem() {
  cinematic = true;
  const by = view.turn_seat;
  const mine = view.tokens.find((t) => t.seat === by);
  const target = view.tokens.filter((t) => t.seat !== by).pop();
  if (!mine || !target) return;
  view.log = [
    { kind: 'item', item: 'bomb', by, target: target.seat, space: target.space, caught: [target.seat], coins: 5 },
  ];
  party.sync(view, members, new Map(ROSTER.map((c) => [c.id, c])));
}

const clock = new THREE.Clock();
function frame() {
  const dt = Math.min(clock.getDelta(), 0.1);
  const time = clock.getElapsedTime();
  const size = new THREE.Vector2(canvas.clientWidth, canvas.clientHeight);
  if (canvas.width !== size.x || canvas.height !== size.y) {
    renderer.setSize(size.x, size.y, false);
    camera.aspect = size.x / Math.max(1, size.y);
    camera.updateProjectionMatrix();
  }
  stage.update(time);
  party.update(time, dt);
  // The board asks for its own shot during a roll or a thrown item; blend to
  // it the way the lobby canvas does.
  const cinema = cinematic ? party.shot() : null;
  const at = cinema ? cinema.at : focus;
  const pitchDegrees = cinema ? THREE.MathUtils.lerp(shot.pitch, cinema.pitch, cinema.weight) : shot.pitch;
  const distance = cinema ? THREE.MathUtils.lerp(shot.distance, cinema.wide * 2.6, cinema.weight) : shot.distance;
  if (scene.fog instanceof THREE.Fog) {
    scene.fog.near = Math.max(90, distance * 1.1);
    scene.fog.far = Math.max(280, distance * 2.6);
  }
  const yaw = cinema ? shot.yaw + cinema.yaw * cinema.weight : shot.yaw;
  const look = cinema ? THREE.MathUtils.lerp(shot.look, at.y + 1.1, cinema.weight) : shot.look;
  const pitch = THREE.MathUtils.degToRad(pitchDegrees);
  const centre = focus.clone().lerp(at, cinema ? cinema.weight : 0);
  camera.position.set(
    centre.x + Math.sin(yaw) * Math.cos(pitch) * distance,
    at.y + Math.sin(pitch) * distance,
    centre.z + Math.cos(yaw) * Math.cos(pitch) * distance,
  );
  const floor = party.groundAt(camera.position.x, camera.position.z) + 3;
  camera.position.y = Math.max(camera.position.y, floor);
  camera.lookAt(centre.x, look, centre.z);
  renderer.render(scene, camera);
  requestAnimationFrame(frame);
}
frame();

Object.assign(window as unknown as Record<string, unknown>, {
  party, scene, camera, view, replayRoll, throwItem, focusSpace,
  resync: () => party.sync(view, members, new Map(ROSTER.map((c) => [c.id, c]))),
  setShot: (next: Partial<typeof shot>) => { Object.assign(shot, next); cinematic = false; },
});
