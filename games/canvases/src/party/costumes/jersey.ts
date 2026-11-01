import * as THREE from 'three';
import {GLTFLoader} from 'three/examples/jsm/loaders/GLTFLoader.js';
import jerseyUrl from './models/hockey-jersey.glb?url';
let source: Promise<THREE.Group> | null = null;
/** The owner's authored PuckLab garment, refitted to this base and attached
 * as three moving parts. Shared source textures stay cached like board assets. */
export async function attachJersey(torso: THREE.Group, arms: THREE.Group[], signal: AbortSignal, atlas?: THREE.Texture) {
  source ??= new GLTFLoader().loadAsync(jerseyUrl).then(gltf=>gltf.scene).catch(error=>{source=null;throw error});
  const asset=await source;
  if(signal.aborted) return null;
  const meshes: THREE.Mesh[]=[];
  asset.traverse(node=>{
    if(!(node instanceof THREE.Mesh))return;
    const sign=node.name==='jersey_sleeve_negx'?-1:node.name==='jersey_sleeve_posx'?1:0;
    const geometry=node.geometry.clone();
    if(sign)geometry.applyMatrix4(new THREE.Matrix4().compose(new THREE.Vector3(sign*.38,.72,0),new THREE.Quaternion().setFromEuler(new THREE.Euler(0,0,sign*.35)),new THREE.Vector3(1,1,1)).invert());
    const original=(Array.isArray(node.material)?node.material[0]:node.material) as THREE.MeshStandardMaterial;
    if (original.map) original.map.anisotropy = 8;
    const material=new THREE.MeshStandardMaterial({map:atlas??original.map,color:'#ffffff',roughness:.9,metalness:0,side:THREE.DoubleSide});
    const mesh=new THREE.Mesh(geometry,material);mesh.name=node.name;mesh.castShadow=true;
    (sign<0?arms[0]:sign>0?arms[1]:torso).add(mesh);meshes.push(mesh);
  });
  return ()=>{for(const mesh of meshes){mesh.removeFromParent();mesh.geometry.dispose();(mesh.material as THREE.Material).dispose();}};
}
