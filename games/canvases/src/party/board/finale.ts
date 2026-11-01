import * as THREE from 'three';
import { place } from './models';
import type { Pose } from '../characters';
import type { MapFinaleView, FinaleRacer } from '../protocol';
import{finaleGuidance}from'./finale-guidance';
import{FinaleObstacles}from'./finale-obstacles';

export type FinaleActorFrame={at:THREE.Vector3;yaw:number;pose:Pose;speed:number};
type Racer={view:FinaleRacer;at:THREE.Vector3;target:THREE.Vector3;yaw:number;pose:Pose;speed:number;halo:THREE.Mesh;payload:THREE.Group|null};
type Gate={root:THREE.Group;ring:THREE.Mesh<THREE.RingGeometry,THREE.MeshBasicMaterial>;bar:THREE.Mesh<THREE.BoxGeometry,THREE.MeshBasicMaterial>;label:THREE.Sprite;texture:THREE.CanvasTexture};
const colors={island:'#bef264',volcano:'#fb923c',shipwreck:'#38bdf8',frozen:'#67e8f9'};

/** Presentation over the actual board, with no alternate arena or simulation.
 * Racer identity and gate geometry survive fresh authoritative room objects. */
export class MapFinaleScene {
  private view:MapFinaleView|null=null;
  private group=new THREE.Group();
  private racers=new Map<number,Racer>();
  private gates:Gate[]=[];
  private route:THREE.Line|null=null;
  private routeKey='';
  private lastElapsed=0;
  private arrowGeometry:THREE.ShapeGeometry;
  private arrowMaterial=new THREE.MeshBasicMaterial({color:'#baff52',transparent:true,depthWrite:false,polygonOffset:true,polygonOffsetFactor:-2});
  private arrowShadowMaterial=new THREE.MeshBasicMaterial({color:'#153113',depthWrite:false,transparent:true,opacity:.88});
  private arrows:THREE.InstancedMesh;
  private arrowShadows:THREE.InstancedMesh;
  private guidanceKey='';
  private arrowMatrix=new THREE.Object3D();
  private viewer:number|null=null;
  private obstacles:FinaleObstacles;
  private haloGeometry=new THREE.RingGeometry(.5,.55,32);
  private coconutGeometry=new THREE.IcosahedronGeometry(.22,1);
  private coconutMaterial=new THREE.MeshStandardMaterial({color:'#78502d',roughness:1});
  private coconutEyeGeometry=new THREE.SphereGeometry(.028,6,4);
  private coconutEyeMaterial=new THREE.MeshBasicMaterial({color:'#241b15'});
  constructor(private parent:THREE.Object3D,private floor:(x:number,z:number)=>number){
    this.group.name='map-finale';this.group.visible=false;parent.add(this.group);
    this.obstacles=new FinaleObstacles(floor);this.group.add(this.obstacles.group);
    const arrow=new THREE.Shape();arrow.moveTo(0,.7);arrow.lineTo(-.45,.08);arrow.lineTo(-.18,.08);arrow.lineTo(-.18,-.6);arrow.lineTo(.18,-.6);arrow.lineTo(.18,.08);arrow.lineTo(.45,.08);arrow.closePath();
    this.arrowGeometry=new THREE.ShapeGeometry(arrow);this.arrowGeometry.rotateX(-Math.PI/2);
    this.arrows=new THREE.InstancedMesh(this.arrowGeometry,this.arrowMaterial,128);this.arrowShadows=new THREE.InstancedMesh(this.arrowGeometry,this.arrowShadowMaterial,128);
    this.arrows.name='finale-road-arrows';this.arrowShadows.name='finale-road-arrow-outlines';
    this.arrows.instanceMatrix.setUsage(THREE.DynamicDrawUsage);this.arrowShadows.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    this.arrows.frustumCulled=this.arrowShadows.frustumCulled=false;this.arrows.count=this.arrowShadows.count=0;
    this.arrows.renderOrder=5;this.arrowShadows.renderOrder=4;this.group.add(this.arrows,this.arrowShadows);
  }
  setViewer(seat:number|null){if(this.viewer!==seat){this.viewer=seat;this.guidanceKey='';}}
  get active(){return this.view!==null;}
  frame(seat:number):FinaleActorFrame|null{const racer=this.racers.get(seat);return racer?{at:racer.at,yaw:racer.yaw,pose:racer.pose,speed:racer.speed}:null;}

  sync(view:MapFinaleView|null){
    if(!view){if(this.view)this.clearRacers();this.view=null;this.group.visible=false;return;}
    if(this.view&&(this.view.map!==view.map||view.elapsed<this.lastElapsed-.3))this.clearRacers();
    this.lastElapsed=view.elapsed;this.view=view;this.group.visible=true;
    const key=JSON.stringify([view.map,view.checkpoints.map(point=>[point.x,point.z,point.radius,point.via]),view.obstacles]);
    if(key!==this.routeKey){this.clearRoute();this.routeKey=key;this.buildRoute(view);this.obstacles.sync(view.obstacles??[]);}
    const seats=new Set<number>();
    for(const next of view.racers){
      seats.add(next.seat);let racer=this.racers.get(next.seat);
      if(!racer){
        const at=new THREE.Vector3(next.x,this.floor(next.x,next.z)+.1,next.z);
        const halo=new THREE.Mesh(this.haloGeometry,new THREE.MeshBasicMaterial({color:colors[view.map],transparent:true,opacity:.25,depthWrite:false}));halo.rotation.x=-Math.PI/2;this.group.add(halo);
        racer={view:{...next},at,target:at.clone(),yaw:next.yaw,pose:'idle',speed:0,halo,payload:null};
        if(view.map==='island'){
          const payload=new THREE.Group();payload.name=`finale-coconut-${next.seat}`;payload.add(new THREE.Mesh(this.coconutGeometry,this.coconutMaterial));
          for(const [x,y]of [[-.06,.035],[.06,.035],[0,-.055]]){const eye=new THREE.Mesh(this.coconutEyeGeometry,this.coconutEyeMaterial);eye.position.set(x,y,.205);payload.add(eye);}
          this.group.add(payload);racer.payload=payload;
        }
        this.racers.set(next.seat,racer);
      }
      racer.view={...next};racer.target.set(next.x,this.floor(next.x,next.z)+.1,next.z);
    }
    for(const [seat,racer]of this.racers)if(!seats.has(seat)){racer.halo.removeFromParent();(racer.halo.material as THREE.Material).dispose();racer.payload?.removeFromParent();this.racers.delete(seat);}
  }

  private buildRoute(view:MapFinaleView){
    const accent=colors[view.map];
    for(let i=0;i<view.checkpoints.length;i++){
      const checkpoint=view.checkpoints[i],previous=view.checkpoints[Math.max(0,i-1)],next=view.checkpoints[Math.min(view.checkpoints.length-1,i+1)];
      const first=next.via?.[0],last=checkpoint.via?.at(-2);
      const direction=i===0&&first?{x:first[0],z:first[1]}:i===0?next:checkpoint,origin=i===0?checkpoint:last?{x:last[0],z:last[1]}:previous;
      const root=new THREE.Group();root.name=`finale-gate-${i}`;root.position.set(checkpoint.x,this.floor(checkpoint.x,checkpoint.z)+.12,checkpoint.z);root.rotation.y=Math.atan2(direction.x-origin.x,direction.z-origin.z);this.group.add(root);
      const ring=new THREE.Mesh(new THREE.RingGeometry(Math.max(.3,checkpoint.radius-.09),checkpoint.radius,48),new THREE.MeshBasicMaterial({color:accent,transparent:true,opacity:.5,depthWrite:false,side:THREE.DoubleSide}));ring.rotation.x=-Math.PI/2;root.add(ring);
      const bar=new THREE.Mesh(new THREE.BoxGeometry(checkpoint.radius*2,.055,.055),new THREE.MeshBasicMaterial({color:accent,transparent:true,opacity:.45}));bar.position.y=1.8;root.add(bar);
      const canvas=document.createElement('canvas');canvas.width=512;canvas.height=128;const ctx=canvas.getContext('2d')!;ctx.fillStyle='#07111df0';ctx.fillRect(6,4,500,120);ctx.strokeStyle=accent;ctx.lineWidth=5;ctx.strokeRect(6,4,500,120);ctx.fillStyle='#fff';ctx.font='900 64px sans-serif';ctx.textAlign='center';ctx.textBaseline='middle';ctx.fillText(i===0?'START':i===view.checkpoints.length-1?'FINISH':`NEXT · ${i}`,256,68,475);
      const texture=new THREE.CanvasTexture(canvas);texture.colorSpace=THREE.SRGBColorSpace;const label=new THREE.Sprite(new THREE.SpriteMaterial({map:texture,transparent:true,depthWrite:false,depthTest:false}));label.position.y=2.8;label.scale.set(2.4,.6,1);label.renderOrder=20;root.add(label);
      this.gates.push({root,ring,bar,label,texture});
      for(const side of [-1,1])void place(root,'flag-pennant',{x:side*checkpoint.radius,z:0,y:0,paint:{_all:accent}}).then(flag=>{if(!flag)return;flag.updateMatrixWorld(true);const height=new THREE.Box3().setFromObject(flag).getSize(new THREE.Vector3()).y;if(height>.001)flag.scale.multiplyScalar(1.8/height);});
    }
    const road=view.checkpoints.flatMap((point,index)=>index===0?[[point.x,point.z]]:point.via?.length?point.via:[[point.x,point.z]]);
    const points=road.map(point=>new THREE.Vector3(point[0],this.floor(point[0],point[1])+.07,point[1]));
    this.route=new THREE.Line(new THREE.BufferGeometry().setFromPoints(points),new THREE.LineDashedMaterial({color:accent,transparent:true,opacity:.22,dashSize:.55,gapSize:.45,depthWrite:false}));this.route.computeLineDistances();this.group.add(this.route);
  }

  update(time:number,dt:number){
    const view=this.view;if(!view)return;
    const focused=this.racers.get(this.viewer??-1)??[...this.racers.values()].sort((a,b)=>a.view.place-b.view.place)[0];
    const next=focused?.view.next_checkpoint??1;
    const mine=this.viewer===null?null:this.racers.get(this.viewer)?.view;
    const guidance=finaleGuidance(view,mine);
    const key=guidance?`${this.routeKey}:${this.viewer}:${mine?.next_checkpoint}:${guidance.waypoint}:${guidance.progressBucket}`:'none';
    if(key!==this.guidanceKey){
      this.guidanceKey=key;const arrows=guidance?.arrows??[];this.arrows.count=this.arrowShadows.count=arrows.length;
      for(let i=0;i<arrows.length;i++){
        const arrow=arrows[i],matrix=this.arrowMatrix;matrix.position.set(arrow.x,this.floor(arrow.x,arrow.z)+.115,arrow.z);matrix.rotation.set(0,arrow.yaw+Math.PI,0);matrix.scale.setScalar(1);matrix.updateMatrix();this.arrows.setMatrixAt(i,matrix.matrix);
        matrix.position.y-=.018;matrix.scale.setScalar(1.2);matrix.updateMatrix();this.arrowShadows.setMatrixAt(i,matrix.matrix);
      }
      this.arrows.instanceMatrix.needsUpdate=this.arrowShadows.instanceMatrix.needsUpdate=true;
    }
    this.arrowMaterial.opacity=.85+Math.sin(time*3)*.15;
    this.gates.forEach((gate,index)=>{
      const point=view.checkpoints[index],blocked=point?.blocked===true,current=index===next,passed=index<next;
      const color=blocked?'#fb7185':current?'#baff52':colors[view.map];gate.ring.material.color.set(color);gate.bar.material.color.set(color);
      gate.ring.material.opacity=current?.8+Math.sin(time*4)*.15:passed?.14:.28;gate.bar.material.opacity=blocked?.55+Math.sin(time*6)*.2:current?.8:.18;
      gate.label.visible=current||index===view.checkpoints.length-1||view.ready>0&&index===0;gate.label.scale.set(current?3.8:2.4,current?.95:.6,1);gate.label.position.y=current?3.2:2.8;gate.root.visible=true;
    });
    for(const racer of this.racers.values()){
      const oldX=racer.at.x,oldZ=racer.at.z;racer.at.lerp(racer.target,1-Math.exp(-dt*16));racer.at.y=this.floor(racer.at.x,racer.at.z)+.1;
      const yaw=Math.atan2(Math.sin(racer.view.yaw-racer.yaw),Math.cos(racer.view.yaw-racer.yaw));racer.yaw+=yaw*(1-Math.exp(-dt*15));
      const moving=Math.hypot(racer.at.x-oldX,racer.at.z-oldZ)/Math.max(.001,dt);racer.speed=Math.min(view.speed*1.5,moving);
      racer.pose=racer.view.finished?'cheer':racer.view.dnf?'sad':view.ready>0?'idle':racer.view.stun>0?'fall':racer.view.waiting?'idle':moving>.1?'ride':'idle';
      racer.halo.position.copy(racer.at);racer.halo.position.y+=.025;
      const hot=view.map==='volcano'?racer.view.gauge:0;const color=racer.view.stun>0?'#fb7185':racer.view.finished?'#baff52':hot>.7?'#fb7185':colors[view.map];
      (racer.halo.material as THREE.MeshBasicMaterial).color.set(color);(racer.halo.material as THREE.MeshBasicMaterial).opacity=racer.view.finished?.55:racer.view.stun>0?.7:.17+hot*.35;
      if(racer.payload){racer.payload.visible=!racer.view.dnf&&racer.view.stun<=0;const bob=Math.sin(time*8)*Math.min(.045,racer.speed*.01);racer.payload.position.copy(racer.at).add(new THREE.Vector3(Math.sin(racer.yaw)*.42,.7+bob,Math.cos(racer.yaw)*.42));racer.payload.rotation.set(0,racer.yaw,Math.sin(time*14)*racer.view.gauge*.25);}
    }
  }
  private clearRacers(){for(const racer of this.racers.values()){racer.halo.removeFromParent();(racer.halo.material as THREE.Material).dispose();racer.payload?.removeFromParent();}this.racers.clear();}
  private clearRoute(){this.guidanceKey='';this.arrows.count=this.arrowShadows.count=0;for(const gate of this.gates){gate.root.removeFromParent();gate.ring.geometry.dispose();gate.ring.material.dispose();gate.bar.geometry.dispose();gate.bar.material.dispose();gate.texture.dispose();gate.label.material.dispose();}this.gates=[];if(this.route){this.route.removeFromParent();this.route.geometry.dispose();(this.route.material as THREE.Material).dispose();this.route=null;}}
  dispose(){this.clearRacers();this.clearRoute();this.obstacles.dispose();this.arrowGeometry.dispose();this.arrowMaterial.dispose();this.arrowShadowMaterial.dispose();this.arrows.dispose();this.arrowShadows.dispose();this.haloGeometry.dispose();this.coconutGeometry.dispose();this.coconutMaterial.dispose();this.coconutEyeGeometry.dispose();this.coconutEyeMaterial.dispose();this.group.removeFromParent();}
}
