import * as THREE from 'three';
import { place } from './models';
import type { BoardLogEntry } from '../protocol';

export type BoardWeapon = 'drive_by' | 'slapshot' | 'slash_rush';
export type WeaponActor = {seat:number;at:THREE.Vector3;yaw:number;item:BoardWeapon|null;slash:number;body:THREE.Object3D};
type Held = {root:THREE.Group;pivot:THREE.Group;item:BoardWeapon;elapsed:number;remaining:number;ready:boolean};
type Shot = {mesh:THREE.Mesh;trail:THREE.Line;flash:THREE.Mesh;from:THREE.Vector3;to:THREE.Vector3;age:number;progress:number;seconds:number;item:BoardWeapon;land:()=>void};
const ease=(t:number)=>{const x=THREE.MathUtils.clamp(t,0,1);return x*x*(3-2*x);};
/** Cosmetic impact ramp only. Authoritative damage has already resolved in
 * the item log; slowing this projectile never slows room time or inputs. */
export function boardShotSpeed(t:number){return t<.6?1:t<.82?1-.8*ease((t-.6)/.22):t<.94?.2:.2+.8*ease((t-.94)/.06);}

/** CC0 held gun/stick models plus visible cartoon bullets/pucks. Owns no
 * character, camera, turn, or gameplay state; raids never request a camera. */
export class BoardWeapons {
  private held=new Map<number,Held>();
  private shots:Shot[]=[];
  private disposed=false;
  private bullet=new THREE.CylinderGeometry(.055,.055,.3,6);
  private puck=new THREE.CylinderGeometry(.2,.2,.09,12);
  private flashGeometry=new THREE.IcosahedronGeometry(.17,0);
  private bulletMaterial=new THREE.MeshStandardMaterial({color:'#fcd34d',emissive:'#f59e0b',emissiveIntensity:1.2,metalness:.2});
  private puckMaterial=new THREE.MeshStandardMaterial({color:'#111827',roughness:.65});
  private flashMaterial=new THREE.MeshBasicMaterial({color:'#fff29b',transparent:true,opacity:.9});
  constructor(private parent:THREE.Object3D,private floor:(x:number,z:number)=>number,private impact:(at:THREE.Vector3,target:number|null,blocked:boolean,item:BoardWeapon,coins:number,first:boolean,damage:number)=>void){}

  private weapon(seat:number,item:BoardWeapon){
    const previous=this.held.get(seat);
    if(previous?.item===item)return previous;
    previous?.root.removeFromParent();
    const root=new THREE.Group(),pivot=new THREE.Group();root.name=`held-${item}-${seat}`;root.add(pivot);this.parent.add(root);
    const held:Held={root,pivot,item,elapsed:0,remaining:0,ready:false};this.held.set(seat,held);
    void place(pivot,item==='drive_by'?'drive-by-uzi':'hockey-stick',{x:0,z:0}).then(model=>{
      if(!model)return;
      if(this.disposed||this.held.get(seat)!==held){model.removeFromParent();return;}
      model.removeFromParent();model.updateMatrixWorld(true);
      const size=new THREE.Box3().setFromObject(model).getSize(new THREE.Vector3());
      model.scale.setScalar((item==='drive_by'?1.1:1.3)/Math.max(.001,size.x,size.y,size.z));
      if(item==='drive_by')model.rotation.y=Math.PI;
      else {model.rotation.z=Math.PI;model.rotation.x=-.28;}
      pivot.add(model);held.ready=true;
    });
    return held;
  }

  fire(entry:BoardLogEntry,origin:THREE.Vector3){
    const item=entry.item as BoardWeapon;
    if(!['drive_by','slapshot'].includes(item)||!Array.isArray(entry.shots))return;
    const held=this.weapon(Number(entry.by),item);held.elapsed=0;held.remaining=item==='drive_by'?.18:.68;
    const seen=new Set<number>();
    for(const raw of entry.shots){
      if(!Number.isFinite(raw.x)||!Number.isFinite(raw.z))continue;
      const target=typeof raw.target==='number'?raw.target:null;
      const hit=Array.isArray(entry.hits)?entry.hits.find((hit:{seat:number})=>hit.seat===target):null;
      const from=origin.clone();from.y+=item==='drive_by'?1.15:.15;
      const to=new THREE.Vector3(raw.x,this.floor(raw.x,raw.z)+(target===null?.14:1.35),raw.z);
      const mesh=new THREE.Mesh(item==='drive_by'?this.bullet:this.puck,item==='drive_by'?this.bulletMaterial:this.puckMaterial);mesh.name=item==='drive_by'?'board-bullet':'board-puck';mesh.castShadow=true;
      const trail=new THREE.Line(new THREE.BufferGeometry().setFromPoints([from,from]),new THREE.LineBasicMaterial({color:item==='drive_by'?'#ffde72':'#b5f5ff',transparent:true,opacity:.8}));trail.name='board-projectile-trail';
      const flash=new THREE.Mesh(this.flashGeometry,this.flashMaterial);flash.position.copy(from);flash.visible=false;
      this.parent.add(mesh,trail,flash);
      this.shots.push({mesh,trail,flash,from,to,age:-(Number(raw.delay)||0)-(item==='slapshot'?.22:.025),progress:0,seconds:Math.max(.12,Math.min(1,Number(raw.flight)||.25)),item,
        land:()=>{const first=target!==null&&!seen.has(target);if(target!==null)seen.add(target);this.impact(new THREE.Vector3(raw.x,this.floor(raw.x,raw.z)+.1,raw.z),target,raw.blocked===true,item,Number(hit?.coins)||0,first,Number(hit?.damage)||0);}});
    }
  }

  actors(actors:WeaponActor[],dt:number){
    const seats=new Set(actors.map(actor=>actor.seat));
    for(const actor of actors){
      let held=this.held.get(actor.seat);
      if(actor.item)held=this.weapon(actor.seat,actor.item);
      if(!held)continue;
      held.elapsed+=dt;held.remaining=Math.max(0,held.remaining-dt);
      if(!actor.item&&held.remaining===0){held.root.removeFromParent();this.held.delete(actor.seat);continue;}
      held.root.position.copy(actor.at);held.root.rotation.y=actor.yaw;
      held.pivot.position.set(.35,held.item==='drive_by'?1.05:1.2,.15);
      held.pivot.rotation.set(0,0,0);
      if(held.item==='drive_by'){
        const kick=held.remaining>0?Math.sin(Math.min(1,held.elapsed/.18)*Math.PI)*.09:0;
        held.pivot.position.z-=kick;held.pivot.rotation.x=kick*.6;
      }else if(actor.item==='slash_rush'){
        const slash=actor.slash>0?1-THREE.MathUtils.clamp(actor.slash/.18,0,1):null;
        held.pivot.rotation.y=slash===null?-.18:-.85+1.7*ease(slash);
        held.pivot.rotation.x=slash===null?0:-.25;
        if(slash!==null)actor.body.rotation.z+=Math.sin(slash*Math.PI)*.07;
      }else if(held.remaining>0){
        const t=held.elapsed;
        const angle=t<.2?-1.05*ease(t/.2):t<.34?-1.05+2.2*ease((t-.2)/.14):1.15*(1-ease((t-.34)/.34));
        held.pivot.rotation.y=angle;held.pivot.rotation.x=-.2*Math.sin(Math.min(1,t/.34)*Math.PI);
        actor.body.rotation.z+=Math.sin(Math.min(1,t/.68)*Math.PI)*.08;
      }
    }
    for(const [seat,held]of this.held)if(!seats.has(seat)){held.root.removeFromParent();this.held.delete(seat);}
  }

  update(dt:number){
    for(let i=this.shots.length-1;i>=0;i--){
      const shot=this.shots[i];shot.age+=dt;
      shot.mesh.visible=shot.trail.visible=shot.age>=0;shot.flash.visible=shot.age>=0&&shot.age<.065&&shot.item==='drive_by';
      if(shot.age<0)continue;
      shot.progress=Math.min(1,shot.progress+dt/shot.seconds*boardShotSpeed(shot.progress));
      shot.mesh.position.lerpVectors(shot.from,shot.to,shot.progress);
      if(shot.item==='slapshot')shot.mesh.position.y+=Math.sin(shot.progress*Math.PI)*.18;
      const direction=shot.to.clone().sub(shot.from).normalize();
      if(shot.item==='drive_by')shot.mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0,1,0),direction);
      else shot.mesh.rotation.y+=dt*28;
      const tail=shot.mesh.position.clone().addScaledVector(direction,-.7);
      shot.trail.geometry.setFromPoints([tail,shot.mesh.position]);
      if(shot.progress>=1){shot.land();shot.mesh.removeFromParent();shot.trail.removeFromParent();shot.trail.geometry.dispose();(shot.trail.material as THREE.Material).dispose();shot.flash.removeFromParent();this.shots.splice(i,1);}
    }
  }
  dispose(){
    this.disposed=true;for(const held of this.held.values())held.root.removeFromParent();this.held.clear();
    for(const shot of this.shots){shot.mesh.removeFromParent();shot.flash.removeFromParent();shot.trail.removeFromParent();shot.trail.geometry.dispose();(shot.trail.material as THREE.Material).dispose();}this.shots=[];
    this.bullet.dispose();this.puck.dispose();this.flashGeometry.dispose();this.bulletMaterial.dispose();this.puckMaterial.dispose();this.flashMaterial.dispose();
  }
}
