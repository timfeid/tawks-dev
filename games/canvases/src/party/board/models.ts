import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';

/** CC0 scenery from Kenney; held gun/stick assets from Quaternius and Pixor.
 * Weapon provenance is recorded in models/WEAPON-LICENSES.md. */
const FILES = import.meta.glob('./models/*.glb', { query: '?url', import: 'default', eager: true }) as Record<string, string>;

const loader = new GLTFLoader();
const cache = new Map<string, Promise<THREE.Object3D | null>>();
const repainted = new Map<string, THREE.Material>();

export type ModelLoadStatus = {name:string;state:'loading'|'ready'|'failed'|'missing';attempts:number;meshes:number;error?:string};
const diagnostics=new Map<string,ModelLoadStatus>();
/** Read-only diagnostics for scene/browser checks, with no network URLs or user data. */
export const modelLoadReport=()=>[...diagnostics.values()].map(status=>({...status}));

function load(name: string) {
  let pending = cache.get(name);
  if (!pending) {
    const url = FILES[`./models/${name}.glb`];
    if(!url){
      if(!diagnostics.has(name))console.warn(`[Tawkathon scenery] Missing bundled model: ${name}`);
      diagnostics.set(name,{name,state:'missing',attempts:0,meshes:0});
      return Promise.resolve(null);
    }
    const previous=diagnostics.get(name);
    const status:ModelLoadStatus={name,state:'loading',attempts:previous?.attempts??0,meshes:0};
    diagnostics.set(name,status);
    const attempt=async(retry:boolean):Promise<THREE.Object3D|null>=>{
      status.attempts++;
      try{
        const gltf=await loader.loadAsync(url);
        let meshes=0;gltf.scene.traverse(child=>{if((child as THREE.Mesh).isMesh)meshes++;});
        if(!meshes)throw new Error('The scenery model has no mesh.');
        status.state='ready';status.meshes=meshes;delete status.error;
        return gltf.scene;
      }catch(error){
        // A one-off failed fetch must not poison every copy of this building
        // for the rest of the session. Retry once, then allow the next caller
        // to try again rather than keeping a fulfilled null in the cache.
        if(retry){await new Promise(resolve=>setTimeout(resolve,250));return attempt(false);}
        status.state='failed';status.error=error instanceof Error?error.message:'Model download failed';
        if(cache.get(name)===pending)cache.delete(name);
        console.warn(`[Tawkathon scenery] Could not load ${name}; a later placement can retry.`,status.error);
        return null;
      }
    };
    pending=attempt(true);
    cache.set(name,pending);
  }
  return pending;
}

export type Placement = {
  x: number;
  z: number;
  y?: number;
  scale?: number;
  yaw?: number;
  /**
   * New colours by the kit's material name (woodBark, leafsGreen, dirt,
   * grass, colormap and so on); null hides that part, so a tree can lose
   * its leaves. `_all` paints every part not named. Textured materials are
   * tinted rather than replaced.
   */
  paint?: Record<string, string | null>;
};

function repaint(material: THREE.Material, colour: string) {
  const key = `${material.uuid}|${colour}`;
  let copy = repainted.get(key);
  if (!copy) {
    copy = material.clone();
    (copy as THREE.MeshStandardMaterial).color?.set(colour);
    // The kits leave metalness at the glTF default of 1, which turns any
    // colour near black under this lighting.
    if ((copy as THREE.MeshStandardMaterial).isMeshStandardMaterial) (copy as THREE.MeshStandardMaterial).metalness = 0;
    repainted.set(key, copy);
  }
  return copy;
}

/**
 * Puts a copy of a model on the board when it has loaded. Copies share the
 * cached geometry and materials, so they are cheap and nothing needs freeing.
 */
/** Every prop placed so far, so a camera up close can move what is in its way. */
export const placed = new Set<THREE.Object3D>();

export function place(parent: THREE.Object3D, name: string, at: Placement): Promise<THREE.Object3D | null> {
  return load(name).then((source) => {
    if (!source) return null;
    const copy = source.clone(true);
    copy.name=`board-model-${name}`;copy.userData.boardModel=name;
    copy.position.set(at.x, at.y ?? 0, at.z);
    copy.scale.setScalar(at.scale ?? 1);
    copy.rotation.y = at.yaw ?? 0;
    copy.traverse((child) => {
      const mesh = child as THREE.Mesh;
      if (!mesh.isMesh) return;
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      if (!at.paint || Array.isArray(mesh.material)) return;
      // `_all` paints every part one colour, as basalt or snow.
      const colour = mesh.material.name in at.paint ? at.paint[mesh.material.name] : at.paint._all;
      if (colour === null) mesh.visible = false;
      else if (colour !== undefined) mesh.material = repaint(mesh.material, colour);
    });
    parent.add(copy);
    placed.add(copy);
    return copy;
  });
}
