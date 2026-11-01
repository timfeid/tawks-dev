import * as THREE from 'three';
import {GLTFLoader} from 'three/examples/jsm/loaders/GLTFLoader.js';
import url from './models/traffic-rental.glb?url';
import type {DriverView} from '../protocol';
/** Open-cockpit CC0 rental, curb arrival and return; server owns movement. */
export class BoardRental {
 readonly group=new THREE.Group();
 private car=new THREE.Group();
 private to=new THREE.Vector3();
 private dead=false;
 private view:DriverView|null=null;
 private key:string|null=null;
 private returning=0;
 private crashRemaining=0;
 private returnFrom=new THREE.Vector3();
 constructor(){this.group.name='rental-car';this.group.visible=false;this.group.add(this.car);new GLTFLoader().load(new URL(url,location.href).href,gltf=>{if(this.dead)return;const root=gltf.scene;const bounds=new THREE.Box3().setFromObject(root);const size=bounds.getSize(new THREE.Vector3());root.scale.setScalar(3/Math.max(size.x,size.z));root.position.y=-bounds.min.y*root.scale.x;root.traverse(o=>{if((o as THREE.Mesh).isMesh){o.castShadow=true;o.receiveShadow=true;}});this.car.add(root);this.group.userData.ready=true;});}
 sync(drivers:DriverView[]|undefined,ground:(x:number,z:number)=>number){const view=drivers?.[0]??null;if(!view){if(this.view){this.returning=.55;this.returnFrom.copy(this.group.position);}this.view=null;return;}const key=`${view.seat}:${view.home_space}`;const fresh=this.key!==key||!this.view;this.key=key;this.view=view;this.returning=0;this.group.visible=true;const arrive=THREE.MathUtils.clamp(view.arriving/.7,0,1);const sideX=Math.cos(view.yaw),sideZ=-Math.sin(view.yaw);this.to.set(view.x+sideX*arrive*4,ground(view.x,view.z)+.1,view.z+sideZ*arrive*4);if(fresh){this.group.position.copy(this.to);this.group.rotation.y=view.yaw;}this.group.userData.seat=view.seat;}
 impact(){this.crashRemaining=1.15;this.returning=.55;}
 update(dt:number){if(this.crashRemaining>0){this.crashRemaining=Math.max(0,this.crashRemaining-dt);this.group.position.x+=Math.sin(this.group.rotation.y)*dt*.84;this.group.position.z+=Math.cos(this.group.rotation.y)*dt*.84;this.car.rotation.z=Math.sin((1.15-this.crashRemaining)*3)*.12;this.returnFrom.copy(this.group.position);if(this.crashRemaining===0)this.car.rotation.z=0;return;}if(this.view){this.group.position.lerp(this.to,1-Math.exp(-18*dt));const d=Math.atan2(Math.sin(this.view.yaw-this.group.rotation.y),Math.cos(this.view.yaw-this.group.rotation.y));this.group.rotation.y+=d*(1-Math.exp(-16*dt));}else if(this.returning>0){this.returning=Math.max(0,this.returning-dt);const progress=1-this.returning/.55;this.group.position.copy(this.returnFrom);this.group.position.x+=Math.sin(this.group.rotation.y)*progress*5;this.group.position.z+=Math.cos(this.group.rotation.y)*progress*5;this.car.scale.setScalar(1-progress*.35);if(this.returning===0)this.group.visible=false;}if(this.view)this.car.scale.setScalar(1);}
 seat(seat:number):THREE.Vector3|null{return this.view?.seat===seat?this.group.position.clone():null;}
 dispose(){this.dead=true;this.group.traverse(o=>{const m=o as THREE.Mesh;if(m.isMesh){m.geometry.dispose();for(const mat of Array.isArray(m.material)?m.material:[m.material]){for(const v of Object.values(mat))if(v instanceof THREE.Texture)v.dispose();mat.dispose();}}});this.group.clear();}
}
