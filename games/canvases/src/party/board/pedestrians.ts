import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { clone } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { PULL, pullKey, type PedestrianView } from '../protocol';

const FILES=import.meta.glob('./pedestrian-models/*.glb',{query:'?url',import:'default',eager:true})as Record<string,string>;
const NAMES=['casual-man','casual-woman','town-gentleman'];
const cache=new Map<string,Promise<GLTF>>();
const loader=new GLTFLoader();
const source=(name:string)=>{
  let pending=cache.get(name);
  if(!pending){pending=loader.loadAsync(FILES[`./pedestrian-models/${name}.glb`]);cache.set(name,pending);}
  return pending;
};
type Rig={root:THREE.Group;mixer:THREE.AnimationMixer;actions:Map<string,THREE.AnimationAction>;current:string};
type Person={root:THREE.Group;wallet:THREE.Group;card:THREE.Mesh;tag:THREE.Sprite;view:PedestrianView;target:THREE.Vector3;rig:Rig|null;initialDown:boolean};
type Bill={mesh:THREE.Mesh;from:THREE.Vector3;seat:number;age:number;delay:number};

/** Six author-rigged CC0 civilians. Only their wallets/cash react to suction;
 * the character always follows authoritative pedestrian x/z and heading. */
export class PedestrianScene {
  private people=new Map<number,Person>();
  private bills:Bill[]=[];
  private disposed=false;
  private walletGeometry=new THREE.BoxGeometry(.18,.13,.055);
  private cardGeometry=new THREE.PlaneGeometry(.15,.075);
  private billGeometry=new THREE.PlaneGeometry(.26,.13);
  private walletMaterial=new THREE.MeshStandardMaterial({color:'#704127',roughness:.9});
  private cardMaterial=new THREE.MeshBasicMaterial({color:'#91df80',side:THREE.DoubleSide});
  private billMaterial:THREE.MeshBasicMaterial;
  private tagMaterial:THREE.SpriteMaterial;
  private textures:THREE.CanvasTexture[]=[];

  constructor(private parent:THREE.Object3D,private floor:(x:number,z:number)=>number,private collector:(seat:number)=>THREE.Vector3|null){
    const money=document.createElement('canvas');money.width=128;money.height=64;
    const ctx=money.getContext('2d')!;ctx.fillStyle='#5bb269';ctx.fillRect(0,0,128,64);ctx.strokeStyle='#b8f2a9';ctx.lineWidth=5;ctx.strokeRect(5,5,118,54);ctx.fillStyle='#d6ffca';ctx.beginPath();ctx.ellipse(64,32,15,21,0,0,Math.PI*2);ctx.fill();ctx.fillStyle='#3a7f45';ctx.font='bold 32px sans-serif';ctx.textAlign='center';ctx.textBaseline='middle';ctx.fillText('1',64,34);
    const cashTexture=new THREE.CanvasTexture(money);cashTexture.colorSpace=THREE.SRGBColorSpace;this.textures.push(cashTexture);
    this.billMaterial=new THREE.MeshBasicMaterial({map:cashTexture,side:THREE.DoubleSide});
    const icon=document.createElement('canvas');icon.width=64;icon.height=64;const ink=icon.getContext('2d')!;ink.fillStyle='#142b22';ink.beginPath();ink.arc(32,32,29,0,Math.PI*2);ink.fill();ink.strokeStyle='#a5f59a';ink.lineWidth=4;ink.strokeRect(13,20,36,26);ink.fillStyle='#a5f59a';ink.fillRect(34,26,18,13);ink.fillStyle='#142b22';ink.beginPath();ink.arc(41,32,2.4,0,Math.PI*2);ink.fill();
    const iconTexture=new THREE.CanvasTexture(icon);iconTexture.colorSpace=THREE.SRGBColorSpace;this.textures.push(iconTexture);
    this.tagMaterial=new THREE.SpriteMaterial({map:iconTexture,transparent:true,depthWrite:false});
  }

  sync(views:PedestrianView[]){
    const visible=new Set<number>();
    for(const view of views.slice(0,6)){
      if(!Number.isInteger(view.id)||!Number.isFinite(view.x)||!Number.isFinite(view.z))continue;
      visible.add(view.id);let person=this.people.get(view.id);
      if(!person){
        const root=new THREE.Group();root.name=`pedestrian-${view.id}`;root.position.set(view.x,this.floor(view.x,view.z)+.1,view.z);root.rotation.y=view.yaw;this.parent.add(root);
        const wallet=new THREE.Group();wallet.position.set(.24,.6,.12);root.add(wallet);
        wallet.add(new THREE.Mesh(this.walletGeometry,this.walletMaterial));
        const card=new THREE.Mesh(this.cardGeometry,this.cardMaterial);card.position.set(0,.025,.032);wallet.add(card);
        const tag=new THREE.Sprite(this.tagMaterial);tag.position.y=1.8;tag.scale.set(.34,.34,1);tag.visible=false;root.add(tag);
        person={root,wallet,card,tag,view:{...view},target:root.position.clone(),rig:null,initialDown:!!view.knocked_down};this.people.set(view.id,person);
        const current=person;
        void source(NAMES[view.id%NAMES.length]).then(gltf=>{
          if(this.disposed||this.people.get(view.id)!==current)return;
          const model=clone(gltf.scene);model.updateMatrixWorld(true);const box=new THREE.Box3().setFromObject(model),size=box.getSize(new THREE.Vector3());
          const scaled=new THREE.Group();scaled.add(model);model.position.sub(new THREE.Vector3((box.min.x+box.max.x)/2,box.min.y,(box.min.z+box.max.z)/2));scaled.scale.setScalar(1.5/Math.max(.001,size.y));current.root.add(scaled);
          scaled.traverse(object=>{if((object as THREE.Mesh).isMesh){object.castShadow=true;object.receiveShadow=true;}});
          const mixer=new THREE.AnimationMixer(model),actions=new Map<string,THREE.AnimationAction>();for(const clip of gltf.animations)actions.set(clip.name,mixer.clipAction(clip));current.rig={root:scaled,mixer,actions,current:''};
          // A late join sees the settled pose; a newly struck pedestrian plays
          // the authored fall exactly once, then remains prone until round reset.
          if(current.initialDown&&current.view.knocked_down){
            const death=actions.get('Death');
            if(death){death.reset().setLoop(THREE.LoopOnce,1);death.clampWhenFinished=true;death.play();death.time=death.getClip().duration;death.paused=true;mixer.update(0);current.rig.current='Death';}
          }
        }).catch(()=>{current.wallet.visible=false;});
      }
      person.view={...view};person.target.set(view.x,this.floor(view.x,view.z)+.1,view.z);
    }
    for(const [id,person]of this.people)if(!visible.has(id)){this.remove(person);this.people.delete(id);}
  }

  robbed(id:number,seat:number,coins:number){
    const person=this.people.get(id);if(!person)return;
    person.root.updateMatrixWorld(true);
    const from=person.wallet.getWorldPosition(new THREE.Vector3());
    for(let i=0;i<Math.min(5,Math.max(0,Math.floor(coins)))&&this.bills.length<30;i++){
      const mesh=new THREE.Mesh(this.billGeometry,this.billMaterial);mesh.name='pedestrian-cash-transfer';mesh.visible=false;this.parent.add(mesh);this.bills.push({mesh,from:from.clone(),seat,age:0,delay:i*.07});
    }
  }

  update(time:number,dt:number,pulling:[number,number][]=[]){
    const progress=new Map(pulling);
    for(const person of this.people.values()){
      const oldX=person.root.position.x,oldZ=person.root.position.z;
      person.root.position.lerp(person.target,1-Math.exp(-dt*15));person.root.position.y=this.floor(person.root.position.x,person.root.position.z)+.1;
      const angle=Math.atan2(Math.sin(person.view.yaw-person.root.rotation.y),Math.cos(person.view.yaw-person.root.rotation.y));person.root.rotation.y+=angle*(1-Math.exp(-dt*13));
      const tug=THREE.MathUtils.clamp(progress.get(pullKey(PULL.pedestrian,person.view.id))??0,0,1);
      person.wallet.visible=!!person.rig&&person.view.remaining_cash>0;
      person.wallet.position.set(.24,person.view.knocked_down?.15:.6,person.view.knocked_down?.45:.12);
      person.tag.position.y=person.view.knocked_down?.6:1.8;
      person.wallet.rotation.z=Math.sin(time*24+person.view.id)*tug*.16;person.card.position.z=.032+tug*.12;
      person.tag.visible=!!person.rig&&tug>0.015&&person.view.remaining_cash>0;
      const name=person.view.knocked_down?'Death':person.view.fleeing?'Run':tug>.015?'RecieveHit':person.view.walking?'Walk':'Idle';
      const rig=person.rig;
      if(rig){
        if(rig.current!==name){
          if(rig.current)rig.actions.get(rig.current)?.fadeOut(.12);
          const next=rig.actions.get(name);
          if(next){next.reset().setLoop(name==='Death'?THREE.LoopOnce:THREE.LoopRepeat,name==='Death'?1:Infinity);next.clampWhenFinished=name==='Death';next.fadeIn(.12).play();}
          rig.current=name;
        }
        const action=rig.actions.get(name);if(action){const speed=Math.hypot(person.root.position.x-oldX,person.root.position.z-oldZ)/Math.max(.001,dt);action.timeScale=name==='Walk'?THREE.MathUtils.clamp(speed/1.3,.65,1.5):name==='Run'?1.15:1;}
        rig.mixer.update(Math.min(.1,Math.max(0,dt)));
      }
    }
    for(let i=this.bills.length-1;i>=0;i--){const bill=this.bills[i];bill.age+=dt;const t=THREE.MathUtils.clamp((bill.age-bill.delay)/.45,0,1);bill.mesh.visible=bill.age>=bill.delay;const to=(this.collector(bill.seat)??bill.from).clone().add(new THREE.Vector3(0,1.15,0));bill.mesh.position.lerpVectors(bill.from,to,t);bill.mesh.position.y+=Math.sin(t*Math.PI)*.45;bill.mesh.rotation.set(.3+Math.sin(time*18+i)*.4,time*5+i,Math.sin(time*13+i)*.3);bill.mesh.scale.setScalar(1-t*.5);if(t>=1){bill.mesh.removeFromParent();this.bills.splice(i,1);}}
  }
  private remove(person:Person){
    person.rig?.mixer.stopAllAction();
    if(person.rig){
      person.rig.mixer.uncacheRoot(person.rig.root.children[0]);
      const skeletons=new Set<THREE.Skeleton>();
      person.rig.root.traverse(object=>{if((object as THREE.SkinnedMesh).isSkinnedMesh)skeletons.add((object as THREE.SkinnedMesh).skeleton);});
      for(const skeleton of skeletons)skeleton.dispose();
    }
    person.root.removeFromParent();
  }
  dispose(){this.disposed=true;for(const person of this.people.values())this.remove(person);this.people.clear();for(const bill of this.bills)bill.mesh.removeFromParent();this.bills=[];this.walletGeometry.dispose();this.cardGeometry.dispose();this.billGeometry.dispose();this.walletMaterial.dispose();this.cardMaterial.dispose();this.billMaterial.dispose();this.tagMaterial.dispose();for(const texture of this.textures)texture.dispose();}
}
