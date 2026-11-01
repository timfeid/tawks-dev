import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PerspectiveCamera, Quaternion, Vector3 } from 'three';
import { CameraFlow } from '../src/party/camera-flow.ts';

const close = (a: number, b: number, epsilon = 1e-6) => assert.ok(Math.abs(a - b) < epsilon, `${a} should equal ${b}`);
const target = (camera: PerspectiveCamera, x: number, yaw = 0, fov = 50, zoom = 1) => {
  camera.position.set(x, 2, 5);
  camera.quaternion.setFromAxisAngle(new Vector3(0, 1, 0), yaw);
  camera.fov = fov;
  camera.zoom = zoom;
};

test('initial rendering and unchanged-mode FPS mouse look have no added input lag', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow();
  target(camera, 12, 0.3, 70, 1.2);
  assert.equal(flow.apply(camera, 'fps', 100), false);
  close(camera.position.x, 12); close(camera.fov, 70);
  for (let i = 0; i < 40; i++) {
    target(camera, 12 + i, i * 0.06);
    const wanted = camera.quaternion.clone();
    assert.equal(flow.apply(camera, 'fps', 100 + i / 60), false);
    close(camera.position.x, 12 + i); close(camera.quaternion.angleTo(wanted), 0);
  }
});

test('mode handoff is continuous, gently starts, and reaches position, quaternion, FOV and zoom', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow({duration: 0.5});
  target(camera, 0); flow.apply(camera, 'orbit', 0);
  target(camera, 10, Math.PI / 2, 80, 2);
  assert.equal(flow.apply(camera, 'vacuum', 1), true);
  close(camera.position.x, 0); close(camera.fov, 50); close(camera.zoom, 1);
  target(camera, 10, Math.PI / 2, 80, 2); flow.apply(camera, 'vacuum', 1.01);
  assert.ok(camera.position.x > 0 && camera.position.x < 0.02, 'no abrupt initial camera kick');
  target(camera, 10, Math.PI / 2, 80, 2); flow.apply(camera, 'vacuum', 1.25);
  close(camera.position.x, 5); close(camera.fov, 65); close(camera.zoom, 1.5);
  close(camera.quaternion.angleTo(new Quaternion().setFromAxisAngle(new Vector3(0,1,0), Math.PI / 4)), 0);
  const oldProjection = camera.projectionMatrix.clone();
  target(camera, 10, Math.PI / 2, 80, 2);
  assert.equal(flow.apply(camera, 'vacuum', 1.5), false);
  close(camera.position.x, 10); close(camera.fov, 80); close(camera.zoom, 2);
  assert.notDeepEqual(camera.projectionMatrix.elements, oldProjection.elements);
  close(camera.matrixWorld.elements[12], 10);
});

test('an interrupted handoff restarts at the last ACTUAL rendered pose', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow({duration: 0.5});
  target(camera, 0); flow.apply(camera, 'orbit', 0);
  target(camera, 10, 1, 70); flow.apply(camera, 'item', 1);
  target(camera, 10, 1, 70); flow.apply(camera, 'item', 1.2);
  const actual = {position:camera.position.clone(), quaternion:camera.quaternion.clone(), fov:camera.fov};
  target(camera, -20, -1, 40); flow.apply(camera, 'storm', 1.21);
  close(camera.position.distanceTo(actual.position), 0); close(camera.quaternion.angleTo(actual.quaternion), 0); close(camera.fov, actual.fov);
  target(camera, -20, -1, 40); flow.apply(camera, 'storm', 1.71);
  close(camera.position.x, -20); close(camera.fov, 40);
});

test('the destination follows a moving target and uses its latest FOV, without restarting the blend', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow({duration: 0.5});
  target(camera, 0); flow.apply(camera, 'orbit', 0);
  target(camera, 10, 0, 60); flow.apply(camera, 'walk', 1);
  target(camera, 20, 0, 70); flow.apply(camera, 'walk', 1.25);
  close(camera.position.x, 10); close(camera.fov, 60);
  target(camera, 30, 0, 90); assert.equal(flow.apply(camera, 'walk', 1.5), false);
  close(camera.position.x, 30); close(camera.fov, 90);
  target(camera, 31); assert.equal(flow.apply(camera, 'walk', 1.51), false); close(camera.position.x, 31);
});

test('equivalent quaternion signs never cause a camera spin', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow();
  target(camera, 0, Math.PI - 0.01); const orientation = camera.quaternion.clone();
  flow.apply(camera, 'orbit', 0);
  for (let i = 0; i <= 48; i++) {
    camera.quaternion.copy(orientation); if (i % 2 === 0) camera.quaternion.set(-orientation.x,-orientation.y,-orientation.z,-orientation.w);
    flow.apply(camera, 'cinematic', 1 + i / 100);
    close(camera.quaternion.angleTo(orientation), 0);
  }
});

test('same-mode transitions are explicit and reset makes a fresh scene immediate', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow();
  target(camera, 0); flow.apply(camera, 'orbit', 0);
  target(camera, 100); flow.transition(); assert.equal(flow.apply(camera, 'orbit', 1), true); close(camera.position.x, 0);
  target(camera, 100); flow.apply(camera, 'orbit', 1.48); close(camera.position.x, 100);
  flow.reset(); target(camera, -100); assert.equal(flow.apply(camera, 'new-scene', 10), false); close(camera.position.x, -100);
});

test('a caller clock correction preserves continuity and never reads a device clock', () => {
  const camera = new PerspectiveCamera(), flow = new CameraFlow({duration: 0.5});
  target(camera, 0); flow.apply(camera, 'orbit', 100);
  target(camera, 10); flow.apply(camera, 'shop', 101);
  target(camera, 10); flow.apply(camera, 'shop', 101.2); const actual = camera.position.x;
  target(camera, 10); assert.equal(flow.apply(camera, 'shop', 2), true); close(camera.position.x, actual);
  target(camera, 10); flow.apply(camera, 'shop', 2.5); close(camera.position.x, 10);
  target(camera, 22); assert.equal(flow.apply(camera, 'shop', Number.NaN), false); close(camera.position.x, 22);
});

test('releasing a forced shot flows back to the caller\'s saved orbit and zoom without altering lens clipping', () => {
  const camera = new PerspectiveCamera(55, 1.8, 0.03, 700), flow = new CameraFlow({duration: 0.5});
  target(camera, 4, 0.9, 55, 1.6);
  const saved = {position:camera.position.clone(), quaternion:camera.quaternion.clone(), fov:camera.fov, zoom:camera.zoom};
  flow.apply(camera, 'manual-orbit', 0);
  target(camera, -8, -1.2, 38); flow.apply(camera, 'storm', 1);
  target(camera, -8, -1.2, 38); flow.apply(camera, 'storm', 1.5);
  const forced = camera.position.clone();
  const restore = () => {camera.position.copy(saved.position);camera.quaternion.copy(saved.quaternion);camera.fov=saved.fov;camera.zoom=saved.zoom;};
  restore(); assert.equal(flow.apply(camera, 'manual-orbit', 2), true); close(camera.position.distanceTo(forced), 0);
  restore(); assert.equal(flow.apply(camera, 'manual-orbit', 2.5), false);
  close(camera.position.distanceTo(saved.position), 0); close(camera.quaternion.angleTo(saved.quaternion), 0); close(camera.fov, saved.fov); close(camera.zoom, saved.zoom);
  close(camera.aspect, 1.8); close(camera.near, 0.03); close(camera.far, 700);
});
