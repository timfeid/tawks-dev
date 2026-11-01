import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import crawlerUrl from './models/crawler/zombie.glb?url';
import type { CrawlerView } from '../protocol';

/** One real skinned CC0 zombie, with an authored prone Crawl clip. Snapshot
 * replacement updates its destination without resetting the animation. */
export class BoardCrawler {
  readonly group = new THREE.Group();
  private rig: THREE.Object3D | null = null;
  private mixer: THREE.AnimationMixer | null = null;
  private target = new THREE.Vector3();
  private yaw = 0;
  private initialized = false;
  private dead = false;
  private view: CrawlerView | null = null;
  private targeted=false;
  private halo=new THREE.Mesh(new THREE.RingGeometry(.65,.83,40),new THREE.MeshBasicMaterial({color:'#67e8f9',transparent:true,opacity:.8,side:THREE.DoubleSide,depthWrite:false}));
  highlight(on:boolean){this.targeted=on;}
  constructor() {
    this.group.name = 'board-crawler'; this.group.visible = false;this.halo.rotation.x=-Math.PI/2;this.halo.position.y=.05;this.halo.visible=false;this.halo.name='crawler-vacuum-target';this.group.add(this.halo);
    new GLTFLoader().load(new URL(crawlerUrl, location.href).href, (gltf) => {
      if (this.dead) { this.disposeRig(gltf.scene); return; }
      this.rig = gltf.scene; this.group.add(this.rig);
      this.rig.traverse(o => {if((o as THREE.Mesh).isMesh){o.castShadow=true;o.receiveShadow=true;}});
      this.mixer = new THREE.AnimationMixer(this.rig);
      const clip = gltf.animations.find(c=>c.name==='Crawl') ?? gltf.animations[0];
      if (clip) this.mixer.clipAction(clip).setEffectiveTimeScale(.55).play();
      this.group.userData.ready = true;
    },undefined,()=>{this.group.userData.failed=true;});
  }
  sync(view: CrawlerView | null | undefined, ground:(x:number,z:number)=>number) {
    this.view=view??null;this.group.visible=!!view;
    if(!view)return;
    this.target.set(view.x,ground(view.x,view.z)+.04,view.z);this.yaw=view.yaw;
    if(!this.initialized){this.group.position.copy(this.target);this.group.rotation.y=this.yaw;this.initialized=true;}
    this.group.userData.target=view.target;
    this.group.userData.grace=view.grace;
  }
  update(dt:number) {
    if(!this.view)return;
    this.halo.visible=this.targeted||this.view.strength>0;this.halo.material.color.set(this.view.strength>0?(this.view.blowing?'#fdba74':'#a3e635'):'#67e8f9');this.halo.scale.setScalar(1+Math.sin(performance.now()/100)*.06);
    this.group.position.lerp(this.target,1-Math.exp(-14*dt));
    const delta=Math.atan2(Math.sin(this.yaw-this.group.rotation.y),Math.cos(this.yaw-this.group.rotation.y));
    this.group.rotation.y+=delta*(1-Math.exp(-8*dt));
    this.mixer?.update(Math.min(dt,.1));
  }
  private disposeRig(root:THREE.Object3D){root.traverse(o=>{const mesh=o as THREE.Mesh;if(!mesh.isMesh)return;mesh.geometry.dispose();for(const material of Array.isArray(mesh.material)?mesh.material:[mesh.material])material.dispose();});}
  dispose(){this.halo.geometry.dispose();this.halo.material.dispose();this.dead=true;this.mixer?.stopAllAction();if(this.rig)this.disposeRig(this.rig);this.group.clear();}
}
