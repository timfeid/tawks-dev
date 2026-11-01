import * as THREE from 'three';
import{place,placed}from'./models';
import type {TrafficView} from '../protocol';
import {pavingTiles,type PavingRect} from './paving';
const CAR_NAMES=['taxi','sedan','taxi','delivery'];
/** Authoritative traffic positions with local interpolation, never a local
 * collision/reward simulation. Models are CC0 Kenney Car Kit. */
export class BoardTraffic {
 readonly group=new THREE.Group();
 private cars=new Map<number,{group:THREE.Group;to:THREE.Vector3;yaw:number;ready:boolean}>();
 private lights=new Map<number,{red:THREE.Mesh;green:THREE.Mesh;halo:THREE.Mesh}>();
 private resources:(THREE.BufferGeometry|THREE.Material)[]=[];
 private built=false;
 private crashes=new Map<number,{age:number;from:THREE.Vector3}>();
 private dead=false;
 constructor(){this.group.name='town-traffic';}
 private box(size:[number,number,number],color:string,at:[number,number,number],glow=false){const g=new THREE.BoxGeometry(...size);const m=new THREE.MeshStandardMaterial({color,roughness:.85,...(glow?{emissive:color,emissiveIntensity:1.2}:{})});this.resources.push(g,m);const mesh=new THREE.Mesh(g,m);mesh.position.set(...at);mesh.receiveShadow=true;this.group.add(mesh);return mesh;}
 private pave(name:string,rectangles:PavingRect[],color:string,lift:number,ground:(x:number,z:number)=>number,cutouts:PavingRect[]=[]){
  const vertices:number[]=[];
  for(const {x0,x1,z0,z1} of pavingTiles(rectangles,cutouts))for(const [x,z] of [[x0,z0],[x0,z1],[x1,z0],[x1,z0],[x0,z1],[x1,z1]])vertices.push(x,ground(x,z)+lift,z);
  const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.Float32BufferAttribute(vertices,3));geometry.computeVertexNormals();
  const material=new THREE.MeshStandardMaterial({color,roughness:.95});this.resources.push(geometry,material);
  const mesh=new THREE.Mesh(geometry,material);mesh.name=name;mesh.receiveShadow=true;this.group.add(mesh);
 }
 sync(view:TrafficView|null|undefined,ground:(x:number,z:number)=>number){this.group.visible=!!view;if(!view)return;
  if(!this.built){this.built=true;
   const asphalt:PavingRect[]=[],sidewalks:PavingRect[]=[],snow:PavingRect[]=[];
   const rect=(x:number,z:number,w:number,d:number):PavingRect=>({x0:x-w/2,x1:x+w/2,z0:z-d/2,z1:z+d/2});
   for(const road of view.roads)for(let i=1;i<road.points.length;i++){
    const a=road.points[i-1],b=road.points[i],dx=b[0]-a[0],dz=b[1]-a[1],length=Math.hypot(dx,dz),horizontal=Math.abs(dx)>Math.abs(dz);
    asphalt.push(rect((a[0]+b[0])/2,(a[1]+b[1])/2,horizontal?length+road.width:road.width,horizontal?road.width:length+road.width));
    for(let t=2;t<length;t+=4){
      const x=a[0]+dx*t/length,z=a[1]+dz*t/length;
      // Leave the whole junction clear, including the ends of each dash.
      const junction=view.roads.some(other=>other!==road&&other.points.slice(1).some((d,j)=>{
        const c=other.points[j],across=horizontal?Math.abs(d[1]-c[1])>Math.abs(d[0]-c[0]):Math.abs(d[0]-c[0])>Math.abs(d[1]-c[1]);
        return across&&x>=Math.min(c[0],d[0])-other.width/2-1&&x<=Math.max(c[0],d[0])+other.width/2+1&&z>=Math.min(c[1],d[1])-other.width/2-1&&z<=Math.max(c[1],d[1])+other.width/2+1;
      }));
      if(!junction){const line=this.box([.12,.025,1.6],'#e7d59b',[x,ground(x,z)+.12,z]);line.rotation.y=Math.atan2(dx,dz);}
    }
   }
   this.pave('street-asphalt',asphalt,'#35404c',.09,ground);
   // Continuous raised sidewalks stop short of intersections, leaving clear
   // crosswalks instead of paving over the road at every crossing.
   for(const road of view.roads)for(let i=1;i<road.points.length;i++){
    const a=road.points[i-1],b=road.points[i],horizontal=Math.abs(b[0]-a[0])>Math.abs(b[1]-a[1]);
    const lo=Math.min(horizontal?a[0]:a[1],horizontal?b[0]:b[1]),hi=Math.max(horizontal?a[0]:a[1],horizontal?b[0]:b[1]),fixed=horizontal?a[1]:a[0];
    const cuts=[lo,hi];
    for(const other of view.roads)for(let n=1;n<other.points.length;n++){
      const c=other.points[n-1],d=other.points[n],across=horizontal?Math.abs(d[1]-c[1])>Math.abs(d[0]-c[0]):Math.abs(d[0]-c[0])>Math.abs(d[1]-c[1]);
      if(!across)continue;const at=horizontal?c[0]:c[1],min=Math.min(horizontal?c[1]:c[0],horizontal?d[1]:d[0]),max=Math.max(horizontal?c[1]:c[0],horizontal?d[1]:d[0]);
      if(at>lo&&at<hi&&fixed>=min&&fixed<=max)cuts.push(at);
    }
    cuts.sort((x,y)=>x-y);
    for(let n=1;n<cuts.length;n++){
      const start=cuts[n-1]+3.6,end=cuts[n]-3.6;if(end-start<1)continue;
      for(const side of [-1,1]){
        const offset=road.width/2+1.05,x=horizontal?(start+end)/2:fixed+side*offset,z=horizontal?fixed+side*offset:(start+end)/2;
        sidewalks.push(rect(x,z,horizontal?end-start:2.1,horizontal?2.1:end-start));
        snow.push(rect(x+(horizontal?0:side*.58),z+(horizontal?side*.58:0),horizontal?end-start:.45,horizontal?.45:end-start));
      }
    }
   }
   this.pave('street-sidewalks',sidewalks,'#aab6bc',.16,ground,asphalt);
   this.pave('street-snow',snow,'#f0f4f2',.18,ground,asphalt);
   for(const c of view.crossings){for(let i=-2;i<=2;i++){const x=c.x+(c.axis==='z'?i*.75:0),z=c.z+(c.axis==='x'?i*.75:0);this.box(c.axis==='x'?[6.1,.03,.36]:[.36,.03,6.1],'#eef7ff',[x,ground(x,z)+.13,z]);}
    const x=c.x+(c.axis==='x'?4.4:2.8),z=c.z+(c.axis==='z'?4.4:2.8),y=ground(x,z);this.box([.18,2.6,.18],'#526170',[x,y+1.3,z]);this.box([.62,1.2,.34],'#172330',[x,y+2.6,z]);const red=this.box([.32,.32,.37],'#ef4444',[x,y+2.88,z],true);const green=this.box([.32,.32,.37],'#75ff46',[x,y+2.35,z],true);const halo=this.box(c.axis==='x'?[6.2,.018,4.1]:[4.1,.018,6.2],'#acff80',[c.x,ground(c.x,c.z)+.115,c.z]);(halo.material as THREE.Material).transparent=true;(halo.material as THREE.Material).opacity=.12;this.lights.set(c.id,{red,green,halo});
   }
  }
  for(const c of view.crossings){const l=this.lights.get(c.id);if(l){l.red.visible=!c.pedestrian_green;l.green.visible=c.pedestrian_green;l.halo.visible=c.pedestrian_green;}}
  for(const car of view.cars){let entry=this.cars.get(car.id);if(!entry){const group=new THREE.Group();group.name=`traffic-car-${car.id}`;this.group.add(group);entry={group,to:new THREE.Vector3(car.x,ground(car.x,car.z)+.1,car.z),yaw:car.yaw,ready:false};entry.group.position.copy(entry.to);this.cars.set(car.id,entry);const own=entry;const name=CAR_NAMES[car.id%CAR_NAMES.length];void place(group,`traffic-${name}`,{x:0,z:0}).then(root=>{if(!root)return;if(this.dead){root.removeFromParent();placed.delete(root);return;}root.removeFromParent();root.position.set(0,0,0);root.updateMatrixWorld(true);const box=new THREE.Box3().setFromObject(root);const size=box.getSize(new THREE.Vector3());root.scale.setScalar(3/Math.max(size.x,size.z));root.position.y=-box.min.y*root.scale.x;root.traverse(o=>{if((o as THREE.Mesh).isMesh){o.castShadow=true;o.receiveShadow=true;const mesh=o as THREE.Mesh;const many=Array.isArray(mesh.material);const materials=(many?mesh.material as THREE.Material[]:[mesh.material as THREE.Material]).map(m=>{const c=m.clone() as THREE.MeshStandardMaterial;c.color.multiply(new THREE.Color(["#ffd34f","#e5e8ee","#ffd34f","#93bfaa"][car.id%4]));c.metalness=0;return c;});mesh.material=many?materials:materials[0];}});own.group.add(root);own.group.userData.modelRoot=root;own.ready=true;});}
   entry.to.set(car.x,ground(car.x,car.z)+.1,car.z);entry.yaw=car.yaw;entry.group.userData.braking=car.braking;entry.group.userData.speed=car.speed;
  }
 }
 impact(id:number){const car=this.cars.get(id);if(car)this.crashes.set(id,{age:0,from:car.group.position.clone()});}
 update(dt:number){for(const [id,car] of this.cars){const crash=this.crashes.get(id);if(crash){crash.age+=dt;if(crash.age<1.15){car.group.position.copy(crash.from);car.group.position.x+=Math.sin(car.group.rotation.y)*crash.age*.84;car.group.position.z+=Math.cos(car.group.rotation.y)*crash.age*.84;continue;}this.crashes.delete(id);}car.group.position.lerp(car.to,1-Math.exp(-18*dt));const d=Math.atan2(Math.sin(car.yaw-car.group.rotation.y),Math.cos(car.yaw-car.group.rotation.y));car.group.rotation.y+=d*(1-Math.exp(-16*dt));}}
 dispose(){this.dead=true;this.resources.forEach(r=>r.dispose());for(const car of this.cars.values()){if(car.group.userData.modelRoot)placed.delete(car.group.userData.modelRoot);car.group.traverse(o=>{const m=o as THREE.Mesh;if(m.isMesh){for(const mat of Array.isArray(m.material)?m.material:[m.material])mat.dispose();}});}this.cars.clear();this.group.clear();}
}
