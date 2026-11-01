import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { clone as cloneSkinned } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { toonTextured } from '../toon';

const files = import.meta.glob('./models/*.glb', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
const loaded = new Map<string, Promise<GLTF>>();

/** Standing height of a pet beside a party character, who is about 1.5 tall. */
export const PET_HEIGHT = 0.5;

export type PetRig = {
  /** Feet rest at y = 0 and the pet faces +z. */
  group: THREE.Group;
  ready: Promise<void>;
  animate: (time: number, cheer?: boolean) => void;
  dispose: () => void;
};

export function petModelUrl(id: string, base = globalThis.location?.href ?? import.meta.url): string | null {
  const asset = files[`./models/${id}.glb`];
  return asset ? new URL(asset, base).href : null;
}

function load(id: string): Promise<GLTF> {
  let gltf = loaded.get(id);
  if (!gltf) {
    const url = petModelUrl(id);
    gltf = url ? new GLTFLoader().loadAsync(url) : Promise.reject(new Error(`No model for ${id}`));
    gltf.catch(() => loaded.delete(id));
    loaded.set(id, gltf);
  }
  return gltf;
}

export function buildPet(id: string, height = PET_HEIGHT): PetRig {
  const group = new THREE.Group();
  group.name = `pet-${id}`;
  const materials: THREE.Material[] = [];
  let mixer: THREE.AnimationMixer | null = null;
  let idle: THREE.AnimationAction | null = null;
  let cheer: THREE.AnimationAction | null = null;
  let disposed = false;
  const ready = load(id).then((gltf) => {
    if (disposed) return;
    const model = cloneSkinned(gltf.scene);
    model.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const source = object.material as THREE.MeshStandardMaterial;
      const material = toonTextured(source.map);
      materials.push(material);
      object.material = material;
      object.castShadow = true;
      object.frustumCulled = false;
    });
    mixer = new THREE.AnimationMixer(model);
    const clip = (name: string) => gltf.animations.find((animation) => animation.name === name);
    const idleClip = clip('Idle') ?? gltf.animations[0];
    idle = idleClip ? mixer.clipAction(idleClip).play() : null;
    const cheerClip = clip('Cheer');
    cheer = cheerClip ? mixer.clipAction(cheerClip) : null;
    mixer.setTime(0);
    model.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(model, true);
    const size = box.getSize(new THREE.Vector3());
    const scale = height / Math.max(size.y, 1e-3);
    model.scale.multiplyScalar(scale);
    model.position.set(-(box.min.x + box.max.x) / 2 * scale, -box.min.y * scale, -(box.min.z + box.max.z) / 2 * scale);
    group.add(model);
  });
  let cheering = false;
  return {
    group,
    ready: ready.catch(() => {}),
    animate(time, wantCheer = false) {
      if (!mixer) return;
      if (cheer && wantCheer !== cheering) {
        cheering = wantCheer;
        (wantCheer ? idle : cheer)?.stop();
        (wantCheer ? cheer : idle)?.play();
      }
      mixer.setTime(time);
    },
    dispose() {
      disposed = true;
      mixer?.stopAllAction();
      group.removeFromParent();
      materials.forEach((material) => material.dispose());
    },
  };
}
