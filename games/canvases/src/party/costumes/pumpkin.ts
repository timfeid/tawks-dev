import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import pumpkinUrl from "./models/pumpkin-head.glb?url";
import { pumpkinFit } from "./pumpkin-fit";
import { headFrame, pumpkinWidth } from "./head-frame";

// A trusted, bundled CC0 KayKit mesh. Every clone shares source GPU resources;
// disposing a character removes its clone without breaking everyone else.
let source: Promise<THREE.Group> | null = null;
function pumpkinSource() {
  if (!source)
    source = new GLTFLoader()
      .loadAsync(pumpkinUrl)
      .then((gltf) => gltf.scene)
      .catch((error) => {
        source = null;
        throw error;
      });
  return source;
}
export function attachPumpkin(parent: THREE.Object3D, width: number) {
  const group = new THREE.Group();
  group.name = "default-pumpkin-head";
  parent.add(group);
  let disposed = false;
  /** Skull bounds, excluding the stalk, centred on the parent's origin. */
  const fitted = new THREE.Vector3();
  const ready = pumpkinSource()
    .then((source) => {
      if (disposed) return false;
      const model = source.clone(true);
      model.updateMatrixWorld(true);
      const points: THREE.Vector3[] = [];
      model.traverse((object) => {
        const mesh = object as THREE.Mesh;
        if (!mesh.isMesh) return;
        const positions = mesh.geometry.getAttribute("position");
        for (let i = 0; i < positions.count; i++)
          points.push(
            new THREE.Vector3()
              .fromBufferAttribute(positions, i)
              .applyMatrix4(mesh.matrixWorld),
          );
      });
      const fit = pumpkinFit(points, width);
      model.scale.setScalar(fit.scale);
      fitted.set(fit.size.x, fit.size.y, fit.size.z);
      model.position.sub(
        new THREE.Vector3(
          fit.center.x,
          fit.center.y,
          fit.center.z,
        ).multiplyScalar(fit.scale),
      );
      model.traverse((object) => {
        if ((object as THREE.Mesh).isMesh) {
          object.castShadow = true;
          object.receiveShadow = true;
        }
      });
      group.add(model);
      return true;
    })
    .catch(() => false);
  return {
    group,
    ready,
    size: fitted,
    dispose() {
      disposed = true;
      group.removeFromParent();
    },
  };
}

/** Follows the actual evaluated head bone after the animation mixer, keeping
 * its bind orientation aligned with the game's +Z face direction. Unsupported
 * non-humanoid uploads keep their appearance rather than a floating pumpkin. */
export function attachAnimatedPumpkin(
  wrapper: THREE.Group,
  root: THREE.Object3D,
) {
  const frame = headFrame(wrapper, root);
  if (!frame) return { update() {}, dispose() {}, ready: Promise.resolve() };
  const anchor = frame.anchor(),
    pumpkin = attachPumpkin(anchor.group, pumpkinWidth(frame.size));
  return {
    update: anchor.update,
    ready: pumpkin.ready.then(() => {}),
    dispose() {
      pumpkin.dispose();
      anchor.dispose();
    },
  };
}
