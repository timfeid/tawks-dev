import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { clone as cloneSkinned } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { toon, toonTextured } from '../toon';
import { petById } from './catalog';
import { stageScale, type PetMove } from './bond';
import { MOVE_SECONDS, moveAt, moveClip } from './moves';
import { buildGear } from './pet-gear';
import { PetFx } from './pet-fx';
import type { PetItem, ShownPet } from './store';

const files = import.meta.glob('./models/*.glb', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
const coats = import.meta.glob('./models/coats/*.png', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
const loaded = new Map<string, Promise<GLTF>>();
const coatTextures = new Map<string, Promise<THREE.Texture>>();

/** Standing height of a pet beside a party character, who is about 1.5 tall. */
export const PET_HEIGHT = 0.5;

export type PetRig = {
  /** Feet rest at y = 0 and the pet faces +z. */
  group: THREE.Group;
  ready: Promise<void>;
  animate: (time: number, cheer?: boolean) => void;
  /** Plays a trick, or a reaction to a pet or a poke, from the next frame, with its burst. */
  perform: (move: PetMove) => void;
  /** A stage reached, or bond gained, shown in particles over the pet. */
  celebrate: (kind: 'stage-up' | 'gain', points?: number) => void;
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

/** The texture of `coat` for pet `id`, sampled like the model's own; null when there is none. */
function coatTexture(id: string, coat: PetItem | null, like: THREE.Texture | null): Promise<THREE.Texture | null> {
  const key = `${id}-${coat?.style}`;
  const asset = coat?.style ? coats[`./models/coats/${key}.png`] : undefined;
  if (!asset) return Promise.resolve(null);
  let texture = coatTextures.get(key);
  if (!texture) {
    texture = new THREE.TextureLoader().loadAsync(new URL(asset, globalThis.location?.href ?? import.meta.url).href).then((loaded) => {
      loaded.flipY = false;
      loaded.colorSpace = like?.colorSpace ?? THREE.SRGBColorSpace;
      if (like) { loaded.wrapS = like.wrapS; loaded.wrapT = like.wrapT; loaded.magFilter = like.magFilter; loaded.minFilter = like.minFilter; }
      return loaded;
    });
    texture.catch(() => coatTextures.delete(key));
    coatTextures.set(key, texture);
  }
  return texture.catch(() => null);
}

/** Puts gear on the bone the pet's fit names, placed from the model's rest pose so it follows the animation. */
function wear(model: THREE.Object3D, rest: THREE.Object3D, item: PetItem | null, place: { bone: string; at: [number, number, number] }, scale: THREE.Vector3, tilt = 0) {
  const gear = item ? buildGear(item.style, item.colors ?? [], toon) : null;
  const bone = model.getObjectByName(place.bone);
  const restBone = rest.getObjectByName(place.bone);
  if (!gear || !bone || !restBone) { gear?.dispose(); return null; }
  gear.group.position.copy(restBone.worldToLocal(new THREE.Vector3(...place.at)));
  gear.group.quaternion.copy(restBone.getWorldQuaternion(new THREE.Quaternion()).invert()).multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), tilt));
  gear.group.scale.copy(scale);
  bone.add(gear.group);
  return gear;
}

/** How much simulated time a fresh pet's ambience starts with, so a still frame already shows it. */
const WARM_UP = 1.5;

/** `look` is the coat and gear it wears and its stage and effect, from shownPet. `fx` false leaves every particle off, for reduced motion, performance mode and guests. */
export function buildPet(
  id: string,
  height = PET_HEIGHT,
  look?: (Pick<ShownPet, 'coat' | 'hat' | 'neck'> & { stage?: number; effect?: string | null }) | null,
  { fx: particles = true }: { fx?: boolean } = {},
): PetRig {
  const group = new THREE.Group();
  group.name = `pet-${id}`;
  const tall = height * stageScale(look?.stage);
  // Moves turn and tip the pet about its middle, not its feet.
  const pivot = new THREE.Group();
  pivot.position.y = tall / 2;
  group.add(pivot);
  const fx = particles && typeof document !== 'undefined' ? new PetFx(tall) : null;
  if (fx) {
    fx.setAmbience(look?.stage ?? 0, look?.effect);
    group.add(fx.group);
  }
  let lastTime: number | null = null;
  let hopClip: THREE.AnimationAction | null = null;
  let move: { name: PetMove; at: number | null } | null = null;
  let moving: THREE.AnimationAction | null = null;
  const materials: THREE.Material[] = [];
  const gear: { dispose(): void }[] = [];
  let mixer: THREE.AnimationMixer | null = null;
  let idle: THREE.AnimationAction | null = null;
  let cheer: THREE.AnimationAction | null = null;
  let disposed = false;
  const ready = load(id).then(async (gltf) => {
    let own: THREE.Texture | null = null;
    gltf.scene.traverse((object) => { if (object instanceof THREE.Mesh) own ??= (object.material as THREE.MeshStandardMaterial).map; });
    const coat = await coatTexture(id, look?.coat ?? null, own);
    if (disposed) return;
    const model = cloneSkinned(gltf.scene);
    model.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      const source = object.material as THREE.MeshStandardMaterial;
      const material = toonTextured(coat ?? source.map);
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
    const hopped = clip('Hop');
    hopClip = hopped ? mixer.clipAction(hopped) : null;
    mixer.setTime(0);
    model.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(model, true);
    const size = box.getSize(new THREE.Vector3());
    const scale = tall / Math.max(size.y, 1e-3);
    model.scale.multiplyScalar(scale);
    model.position.set(-(box.min.x + box.max.x) / 2 * scale, -box.min.y * scale - tall / 2, -(box.min.z + box.max.z) / 2 * scale);
    const fit = petById(id)?.fit;
    if (fit && look) {
      gltf.scene.updateMatrixWorld(true);
      const [wide, deep] = fit.neck.size;
      for (const worn of [
        wear(model, gltf.scene, look.hat, fit.hat, new THREE.Vector3().setScalar(fit.hat.size), fit.hat.tilt),
        wear(model, gltf.scene, look.neck, fit.neck, new THREE.Vector3(wide, (wide + deep) / 2, deep)),
      ]) if (worn) gear.push(worn);
    }
    pivot.add(model);
  });
  let cheering = false;
  return {
    group,
    ready: ready.catch(() => {}),
    animate(time, wantCheer = false) {
      if (fx) {
        if (lastTime === null) for (let warm = 0; warm < WARM_UP; warm += 1 / 30) fx.update(1 / 30);
        else fx.update(Math.min(Math.max(time - lastTime, 0), 0.1));
        lastTime = time;
      }
      if (!mixer) return;
      if (move && move.at === null) move.at = time;
      const t = move && move.at !== null ? time - move.at : 0;
      if (move && t >= MOVE_SECONDS[move.name]) move = null;
      const named = move ? moveClip(move.name) : null;
      const wanted = named === 'Hop' ? (hopClip ?? cheer) : named === 'Cheer' ? cheer : null;
      if (wanted !== moving) {
        moving?.stop();
        moving = wanted;
        if (moving) {
          idle?.stop();
          cheer?.stop();
          moving.play();
        } else {
          cheering = !wantCheer;
        }
      }
      if (!moving && cheer && wantCheer !== cheering) {
        cheering = wantCheer;
        (wantCheer ? idle : cheer)?.stop();
        (wantCheer ? cheer : idle)?.play();
      }
      const pose = move ? moveAt(move.name, t) : { lift: 0, spin: 0, flip: 0, squash: 0 };
      pivot.position.y = tall / 2 + pose.lift * tall;
      pivot.rotation.set(pose.flip, pose.spin, 0);
      pivot.scale.set(1 + pose.squash / 2, 1 - pose.squash, 1 + pose.squash / 2);
      mixer.setTime(time);
    },
    perform(name) {
      move = { name, at: null };
      fx?.burst(name);
    },
    celebrate(kind, points = 0) {
      if (kind === 'stage-up') fx?.burst('stage-up');
      else fx?.gain(points);
    },
    dispose() {
      disposed = true;
      fx?.dispose();
      mixer?.stopAllAction();
      gear.forEach((worn) => worn.dispose());
      group.removeFromParent();
      materials.forEach((material) => material.dispose());
    },
  };
}
