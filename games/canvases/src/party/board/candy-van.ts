import * as THREE from 'three';
import {GLTFLoader} from 'three/examples/jsm/loaders/GLTFLoader.js';
import url from './models/candy-van.glb?url';
import type {CandyVanView} from '../protocol';
/** A real CC0 van with white body paint, glass, lights and tyres. Arrival and
 * departure are local presentation only; purchases remain authoritative. */
export class BoardCandyVan {
 readonly group=new THREE.Group();
 private model=new THREE.Group();
 private destination=new THREE.Vector3();
 private origin=new THREE.Vector3();
 private phase:'absent'|'arrive'|'parked'|'leave'='absent';
 private age=0;
 private phaseAt=0;
 private dead=false;
 constructor(){this.group.name='candy-van';this.group.visible=false;this.group.add(this.model);new GLTFLoader().load(new URL(url,location.href).href,gltf=>{if(this.dead)return;const root=gltf.scene;const box=new THREE.Box3().setFromObject(root),size=box.getSize(new THREE.Vector3());root.scale.setScalar(4.5/Math.max(size.x,size.z));root.position.y=-box.min.y*root.scale.x;root.traverse(o=>{if((o as THREE.Mesh).isMesh){o.castShadow=true;o.receiveShadow=true;}});this.model.add(root);this.group.userData.ready=true;});}
 sync(view:CandyVanView|null|undefined,ground:(x:number,z:number)=>number){if(view){this.destination.set(view.x,ground(view.x,view.z)+.12,view.z);this.group.rotation.y=view.yaw;if(this.phase==='absent'||this.phase==='leave'){this.origin.copy(this.destination);this.origin.x-=Math.sin(view.yaw)*75;this.origin.z-=Math.cos(view.yaw)*75;this.origin.y=this.destination.y;this.age=0;this.phaseAt=performance.now();this.phase='arrive';this.group.position.copy(this.origin);}this.group.visible=true;this.group.userData.remaining=view.remaining;this.group.userData.canBuy=view.can_buy;}else if(this.phase==='parked'||this.phase==='arrive'){this.origin.copy(this.group.position);this.destination.copy(this.origin);this.destination.x+=Math.sin(this.group.rotation.y)*75;this.destination.z+=Math.cos(this.group.rotation.y)*75;this.phase='leave';this.age=0;this.phaseAt=performance.now();}}
 update(dt:number){if(this.phase==='absent')return;this.age=Math.max(0,(performance.now()-this.phaseAt)/1000);if(this.phase==='arrive'){const t=THREE.MathUtils.clamp(this.age/1.4,0,1),eased=1-(1-t)**4;this.group.position.lerpVectors(this.origin,this.destination,eased);if(t>=1)this.phase='parked';}else if(this.phase==='parked'){this.group.position.copy(this.destination);}else if(this.phase==='leave'){const t=THREE.MathUtils.clamp(this.age/1.3,0,1);this.group.position.lerpVectors(this.origin,this.destination,t*t);if(t>=1){this.phase='absent';this.group.visible=false;}}}
 dispose(){this.dead=true;this.group.traverse(o=>{const mesh=o as THREE.Mesh;if(mesh.isMesh){mesh.geometry.dispose();for(const mat of Array.isArray(mesh.material)?mesh.material:[mesh.material]){for(const v of Object.values(mat))if(v instanceof THREE.Texture)v.dispose();mat.dispose();}}});this.group.clear();}
}
