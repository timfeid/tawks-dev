import * as THREE from 'three';
import type { CharacterDef, Feature } from './protocol';
import { wardrobeClient } from './costumes/client';
import { loadCustomModel } from './models/custom-model-rig';
import { resolveCatalogModel } from './models/catalog';
import { attachJersey } from './costumes/jersey';
import { jerseyAtlas, jerseyDesign } from './costumes/pucklab-jersey';
import {attachPumpkin}from'./costumes/pumpkin';
import {bodyWidth}from'./costumes/body-build';
import {GRIPS,heldPoint}from'./controller';
import {HOLIDAY_HATS}from'./costumes/selection';
import {buildFace,reactionWeight,REACTION_LENGTH,type Face,type FaceReaction,type Mood} from './reactions/face';
import {chooseReaction} from './reactions/catalog';

export type Pose = 'idle' | 'ride' | 'cheer' | 'sad' | 'fall' | 'sit' | 'wave' | 'play';

export type CharacterRig = {
  group: THREE.Group;
  /** Advance the animation. `speed` is ground speed in units per second. */
  animate: (pose: Pose, time: number, speed: number, waveFor?: number) => void;
  dispose: () => void;
  /** Authored meshes have finished loading, for still previews of store items. */
  ready?: Promise<void>;
  setBodyWidth?: (width: number) => void;
  attachments?: {head:THREE.Group;neck:THREE.Group};
  /** Makes a face that eases back after a moment: the look's own reaction when none is named, `null` to stop. `hold` keeps it on, for stills. */
  react?: (reaction?: FaceReaction | null, hold?: boolean) => void;
};

export type CharacterOptions = {
  /** Make the look's reaction when the pose turns to a cheer. Previews leave it to hovering. */
  reactOnCheer?: boolean;
};

let toonRamp: THREE.DataTexture | null = null;

function ramp() {
  if (!toonRamp) {
    toonRamp = new THREE.DataTexture(new Uint8Array([90, 90, 90, 255, 175, 175, 175, 255, 255, 255, 255, 255]), 3, 1, THREE.RGBAFormat);
    toonRamp.minFilter = THREE.NearestFilter;
    toonRamp.magFilter = THREE.NearestFilter;
    toonRamp.needsUpdate = true;
  }
  return toonRamp;
}

const toon = (color: string) => new THREE.MeshToonMaterial({ color, gradientMap: ramp() });

/**
 * Builds a party character from its roster entry: a round body, big eyes and
 * a signature headpiece. Feet rest at y = 0 and the character faces +z.
 */
export function buildCharacter(def: CharacterDef, {reactOnCheer = true}: CharacterOptions = {}): CharacterRig {
  const group = new THREE.Group();
  group.name = `character-${def.id}`;
  const width = bodyWidth(def.costume?.body_width), depth = Math.sqrt(width);
  group.userData.bodyWidth = def.costume?.model_id ? 1 : width;
  const lowerBody = new THREE.Group();lowerBody.name="body-build-legs";lowerBody.scale.set(width,1,depth);group.add(lowerBody);
  const materials = {
    body: toon(def.body),
    accent: toon(def.accent),
    belly: toon(def.belly),
    white: toon('#ffffff'),
    dark: new THREE.MeshBasicMaterial({ color: '#1b1b24' }),
    // Drawn on the skin, so a dark body needs them light. White frames would merge into the eyes.
    ink: new THREE.MeshBasicMaterial({ color: isDark(def.body) ? '#f4f4f7' : '#1b1b24' }),
    frames: new THREE.MeshBasicMaterial({ color: isDark(def.body) ? '#e0b84a' : '#1b1b24' }),
    cheek: new THREE.MeshBasicMaterial({ color: '#ff8fa3', transparent: true, opacity: 0.75 }),
    mouth: new THREE.MeshBasicMaterial({ color: '#5a1626' }),
  };
  const geometries: THREE.BufferGeometry[] = [];
  const extraMaterials: THREE.Material[] = [];
  const glowMaterial = (color: string) => {
    const material = new THREE.MeshBasicMaterial({ color });
    extraMaterials.push(material);
    return material;
  };
  const paint = (color: string, opacity?: number) => {
    const material = new THREE.MeshBasicMaterial({ color, transparent: opacity !== undefined, opacity: opacity ?? 1, depthWrite: opacity === undefined });
    extraMaterials.push(material);
    return material;
  };
  const mesh = (geometry: THREE.BufferGeometry, material: THREE.Material, parent: THREE.Object3D = lowerBody) => {
    geometries.push(geometry);
    const m = new THREE.Mesh(geometry, material);
    m.castShadow = true;
    parent.add(m);
    return m;
  };

  const legs = [-1, 1].map((side) => {
    const leg = mesh(new THREE.CapsuleGeometry(0.1, 0.12, 4, 10), materials.accent);
    leg.position.set(side * 0.17, 0.16, 0);
    return leg;
  });

  const torso = new THREE.Group();
  group.add(torso);
  const bodyShape = new THREE.Group();bodyShape.name="body-build-torso";bodyShape.scale.set(width,1,depth);torso.add(bodyShape);
  const body = mesh(new THREE.SphereGeometry(0.42, 24, 18), materials.body, bodyShape);
  body.scale.set(1, 1.08, 0.95);
  body.position.y = 0.55;
  const belly = mesh(new THREE.SphereGeometry(0.3, 20, 14, 0, Math.PI * 2, 0, bellyArc(def.costume)), materials.belly, bodyShape);
  belly.scale.set(1, 1.1, 0.45);
  belly.position.set(0, 0.5, 0.27);
  belly.visible = !def.costume?.shirt;

  const head = new THREE.Group();head.name="character-head";
  head.position.y = 1.08;
  torso.add(head);
  const face=new THREE.Group();head.add(face);
  let faces:Face[]=[buildFace(face,materials,mesh,paint)];

  const pumpkin=def.costume?.head==='head-pumpkin'?attachPumpkin(head,.94):null;

  const limbs: Limb[] = [];
  const arms = [-1, 1].map((side) => {
    const shoulder = new THREE.Group();
    shoulder.position.set(side * 0.38, 0.72, 0);
    bodyShape.add(shoulder);
    const arm = mesh(new THREE.CapsuleGeometry(0.075, 0.2, 4, 10), materials.body, shoulder);
    arm.position.y = -0.14;
    const hand = mesh(new THREE.SphereGeometry(0.085, 12, 10), materials.white, shoulder);
    hand.position.y = -ARM_REACH;
    shoulder.rotation.z = side * 0.35;
    limbs.push({arm, ends: [hand]});
    return shoulder;
  });

  const costume = def.costume;
  const shirtFallback:THREE.Object3D[]=[];
  const jerseyLining = new Set<THREE.Object3D>();
  const shirtMesh=(g:THREE.BufferGeometry,m:THREE.Material,parent:THREE.Object3D)=>{const part=mesh(g,m,parent);shirtFallback.push(part);return part;};
  const hat: Record<string, Feature> = {'hat-cap':'cap','hat-beanie':'beanie','hat-visor':'visor','hat-horns':'horns','hat-ears':'long_ears','hat-tophat':'top_hat','hat-crown':'crown','hat-wizard':'wizard'};
  const headwear = new THREE.Group();headwear.name='headwear';head.add(headwear);
  const holidayHat=costume?.extras?.find(id=>HOLIDAY_HATS.includes(id));
  // A holiday hat goes on in place of the hat or signature headpiece, not over it.
  addFeature(holidayHat ? 'none' : costume?.hat ? hat[costume.hat] ?? 'none' : def.feature, headwear, mesh, materials, glowMaterial);
  const cloth = (color: string) => { const material=toon(color); extraMaterials.push(material); return material; };
  const design=jerseyDesign(costume);
  const garment=costume?.shirt==='shirt-hockey'||!!design;
  if (costume?.shirt) {
    const fabric=garment
      ? new THREE.MeshStandardMaterial({color:design?.body??'#164b77',roughness:.9})
      : cloth(costume.shirt==='shirt-hoodie' ? '#26344c' : '#f5f5ef');
    if(garment)extraMaterials.push(fabric);
    const shirt=shirtMesh(new THREE.SphereGeometry(0.445,24,16,0,Math.PI*2,0,2.25),fabric,bodyShape);
    shirt.scale.set(1,1.08,0.97);shirt.position.y=0.55;
    if(garment)jerseyLining.add(shirt);
    for(const shoulder of arms) {
      const hockey=garment;
      const sleeve=shirtMesh(new THREE.CapsuleGeometry(hockey?0.088:0.095,hockey?0.12:costume.shirt==='shirt-tee'?0.08:0.18,4,10),fabric,shoulder);
      sleeve.position.y=hockey?-0.10:costume.shirt==='shirt-tee'?-0.04:-0.12;
      if(hockey)jerseyLining.add(sleeve);
    }
    const trim=design?cloth(design.hem_stripes?.[0]?.color??design.trim??design.body):materials.accent;
    const hem=shirtMesh(new THREE.TorusGeometry(0.35,0.028,6,32),trim,bodyShape);
    hem.rotation.x=Math.PI/2;hem.position.y=0.262;hem.scale.y=0.97;
    if(costume.shirt==='shirt-hoodie') {
      const hood=shirtMesh(new THREE.SphereGeometry(0.3,20,12),fabric,bodyShape);
      hood.scale.set(0.95,0.55,0.45);hood.rotation.x=-0.5;hood.position.set(0,0.86,-0.24);
      for(const side of [-1,1]) {const string=shirtMesh(new THREE.CylinderGeometry(0.012,0.012,0.16,6),materials.white,bodyShape);string.position.set(side*0.07,0.72,0.41);string.rotation.x=-0.35;}
    }
    if(garment) {
      const crest=shirtMesh(new THREE.CircleGeometry(0.105,6),design?cloth(design.trim??'#ffffff'):materials.accent,bodyShape);crest.position.set(0,0.67,0.445);
      const line=shirtMesh(new THREE.TorusGeometry(0.38,0.035,6,32),trim,bodyShape);line.rotation.x=Math.PI/2;line.position.y=0.32;line.scale.z=0.95;
    }
  }
  if(costume?.pants) {
    const fabric=cloth(costume.pants==='pants-jeans'?'#315b88':costume.pants==='pants-tracksuit'?'#172435':'#444b5d');
    const waist=mesh(new THREE.SphereGeometry(WAIST_RADIUS,20,12,0,Math.PI*2,WAIST_ARC,Math.PI-WAIST_ARC),fabric,bodyShape);waist.scale.set(1,WAIST_STRETCH,0.96);waist.position.y=0.55;waist.name='worn-pants';
    for(const side of [-1,1]) {const leg=mesh(new THREE.CapsuleGeometry(0.113,costume.pants==='pants-shorts'?0.04:0.12,4,10),fabric);leg.position.set(side*0.17,0.19,0);leg.name='worn-pants';}
  }
  if(costume?.shoes) {
    const material=cloth(costume.shoes==='shoes-sneakers'?'#f5f5f0':'#243044');
    for(const side of [-1,1]) {
      const shoe=mesh(new THREE.SphereGeometry(0.145,14,10),material);shoe.scale.set(0.95,0.65,1.5);shoe.position.set(side*0.17,0.09,0.055);shoe.name='worn-shoes';
      const sole=mesh(new THREE.BoxGeometry(0.25,0.035,0.34),costume.shoes==='shoes-skates'?cloth('#bcd1df'):materials.accent);sole.position.set(side*0.17,0.025,0.055);sole.name='worn-shoes';
    }
  }
  if(costume?.gloves) {
    const fabric=costume.gloves==='gloves-hockey'?cloth('#223650'):materials.accent;
    arms.forEach((shoulder,i)=>{const glove=mesh(new THREE.SphereGeometry(0.112,14,10),fabric,shoulder);glove.position.y=-ARM_REACH;glove.scale.y=1.15;limbs[i].ends.push(glove);});
  }
  // Extras sit on the round head; once the bigger pumpkin is on, they move out to its face and below its chin.
  const extras=new THREE.Group();extras.name='worn-head-extras';
  const neckwear=new THREE.Group();neckwear.name='worn-neck-extras';
  head.add(extras);bodyShape.add(neckwear);
  let smoke:((time:number)=>void)|null=null;
  const extrasWorn=new Set(costume?.extras??[]);
  if(extrasWorn.has('extra-cigar')) {
    const cigar=mesh(new THREE.CylinderGeometry(0.026,0.026,0.27,10),cloth('#6f4029'),extras);cigar.rotation.x=Math.PI/2;cigar.position.set(0.045,-0.12,0.51);
    const tip=mesh(new THREE.CylinderGeometry(0.027,0.027,0.025,10),glowMaterial('#f97316'),extras);tip.rotation.x=Math.PI/2;tip.position.set(0.045,-0.12,0.65);
    tip.castShadow=false;
    // A grey rim of ash round a hot ember.
    const ash=mesh(new THREE.TorusGeometry(0.026,0.007,5,12),cloth('#9ca3af'),extras);ash.position.set(0.045,-0.12,0.66);ash.castShadow=false;
    const ember=mesh(new THREE.SphereGeometry(0.02,8,6),glowMaterial('#fde047'),extras);ember.position.set(0.045,-0.12,0.664);ember.scale.z=0.45;ember.castShadow=false;
    const glow=tip.material as THREE.MeshBasicMaterial,core=ember.material as THREE.MeshBasicMaterial;
    const puff=new THREE.SphereGeometry(1,7,5);geometries.push(puff);
    const wisps=Array.from({length:7},()=>{
      const material=new THREE.MeshBasicMaterial({color:'#d4d4d8',transparent:true,opacity:0,depthWrite:false});
      extraMaterials.push(material);
      const wisp=new THREE.Mesh(puff,material);wisp.renderOrder=2;wisp.userData.characterExtra=true;group.add(wisp);
      return {wisp,material,from:new THREE.Vector3(),last:1};
    });
    const at=new THREE.Vector3();
    smoke=(time)=>{
      // The ember breathes, brighter than its own colour so the bloom picks it up.
      const flicker=0.75+0.25*Math.sin(time*7.3)*Math.sin(time*2.1+1)+0.08*Math.sin(time*23);
      glow.color.setRGB(2.2*flicker,0.75*flicker,0.18*flicker);
      core.color.setRGB(3*flicker,2.2*flicker,0.6*flicker);
      ember.getWorldPosition(at);group.worldToLocal(at);
      wisps.forEach((w,i)=>{
        const age=(time*0.55+i/wisps.length)%1;
        // Each puff leaves from wherever the tip is when it starts, then rises on its own.
        if(age<w.last)w.from.copy(at);
        w.last=age;
        const drift=Math.sin(time*1.3+i*1.7)*0.12*age;
        w.wisp.position.set(w.from.x+drift,w.from.y+0.05+age*0.85,w.from.z+Math.cos(time*1.1+i)*0.06*age);
        w.wisp.scale.setScalar(0.025+age*0.11);
        w.material.opacity=0.5*Math.min(1,age*6)*(1-age);
      });
    };
  }
  if(extrasWorn.has('extra-glasses')) {
    for(const side of [-1,1]) {const lens=mesh(new THREE.TorusGeometry(0.105,0.018,6,20),materials.frames,extras);lens.position.set(side*0.14,0.05,0.414);}
    const bridge=mesh(new THREE.BoxGeometry(0.09,0.025,0.025),materials.frames,extras);bridge.position.set(0,0.05,0.42);
  }
  if(extrasWorn.has('extra-bowtie')) {
    for(const side of [-1,1]) {const tie=mesh(new THREE.ConeGeometry(0.07,0.14,3),materials.accent,neckwear);tie.rotation.z=side*Math.PI/2;tie.position.x=side*0.065;}
  }
  if(extrasWorn.has('extra-medal')) {
    const ribbon=cloth('#2563eb');
    for(const side of [-1,1]) {const strap=mesh(new THREE.BoxGeometry(0.035,0.17,0.015),ribbon,neckwear);strap.position.set(side*0.04,-0.08,0);strap.rotation.z=-side*0.35;}
    const medal=mesh(new THREE.CylinderGeometry(0.075,0.075,0.022,20),cloth('#f5c542'),neckwear);medal.rotation.x=Math.PI/2;medal.position.set(0,-0.2,0.01);
  }
  // Under the chin, where the round head no longer covers the body, and clear of a shirt's front.
  if(extrasWorn.has('extra-bowtie')||extrasWorn.has('extra-medal'))neckwear.position.set(0,0.74,costume?.shirt?0.45:0.42);
  // Worn like a hat, so they follow the headwear onto the pumpkin and move to an imported model's head with the other extras.
  const holidayHead=new THREE.Group();holidayHead.name='worn-holiday-head';headwear.add(holidayHead);
  if(holidayHat)addHolidayHat(holidayHat,holidayHead,mesh,cloth);
  if(extrasWorn.has('extra-antlers'))addAntlers(holidayHead,mesh,cloth);
  if(extrasWorn.has('extra-headphones')) {
    const band=mesh(new THREE.TorusGeometry(0.43,0.03,8,28,Math.PI),materials.dark,holidayHead);band.position.y=0.04;
    for(const side of [-1,1]) {const cup=mesh(new THREE.CylinderGeometry(0.12,0.12,0.08,18),materials.accent,holidayHead);cup.rotation.z=Math.PI/2;cup.position.set(side*0.43,0.02,0);}
  }
  if(extrasWorn.has('extra-halo')) {
    const halo=mesh(new THREE.TorusGeometry(0.26,0.035,8,32),glowMaterial('#fde68a'),holidayHead);halo.rotation.x=Math.PI/2;halo.position.y=0.78;halo.castShadow=false;
  }
  const scarf=new THREE.Group();scarf.name='worn-scarf';
  // On a slim build the head is wider than the body where they meet, so the scarf only ever widens with the build. Over a shirt its tail hangs a little further out.
  const fitScarf=(width:number)=>scarf.scale.set(Math.max(1,width),1,Math.max(1,Math.sqrt(width))*(costume?.shirt?1.08:1));
  if(extrasWorn.has('extra-scarf')) {
    addScarf(scarf,SCARF_RING,mesh,cloth);
    scarf.position.set(0,0.84,0);
    fitScarf(width);
    torso.add(scarf);
  }
  // A head that cannot change its face (the pumpkin, an imported model) shows the reaction on a small round face popping up above it instead.
  let bubble:THREE.Group|null=null;
  const popAbove=(parent:THREE.Object3D,top:number)=>{
    bubble?.removeFromParent();
    bubble=new THREE.Group();bubble.name='reaction-bubble';bubble.visible=false;bubble.userData.characterExtra=true;
    bubble.position.y=top+0.32;
    parent.add(bubble);
    faces=[buildFace(bubble,materials,mesh,paint)];
  };
  const pending: Promise<unknown>[] = [];
  pending.push(pumpkin?.ready.then(ready=>{
    if(!ready)return;
    face.visible=false;
    popAbove(head,pumpkin.size.y/2);
    // The pumpkin is taller than the round head; centred where that head was, its narrow base sinks into the body and the body pokes through at the chin.
    head.position.y+=Math.max(0,0.94-(head.position.y-pumpkin.size.y/2));
    // A dark core fills the hollow pumpkin, so its carved holes look dark rather than showing the body inside.
    const core=mesh(new THREE.SphereGeometry(1,16,12),materials.dark,head);
    core.scale.set(pumpkin.size.x*.39,pumpkin.size.y*.44,pumpkin.size.z*.39);
    headwear.scale.set(pumpkin.size.x/.8,1,pumpkin.size.z/.8);
    headwear.position.y=pumpkin.size.y/2-.28;
    headwear.traverse(part=>{const fit=part.userData.pumpkinFit as HatFit|undefined;if(fit){part.rotation.x=-fit.tilt;part.position.y=fit.lift;}});
    core.castShadow=false;
    // Scaled about the head's centre so the glasses and cigar meet the pumpkin's face
    // (a little in from its widest, between the ribs), not its inside.
    extras.scale.setScalar(Math.max(1,pumpkin.size.z/2*0.95/0.4));
    // The pumpkin hangs lower than the old head, so the bowtie drops to just under its chin.
    const chin=head.position.y-pumpkin.size.y/2;
    if(chin<0.98) {
      const y=Math.max(0.6,chin-0.03);
      const surface=0.95*Math.sqrt(Math.max(0,0.42*0.42-((y-0.55)/1.08)**2));
      neckwear.position.set(0,y,surface+0.04);
    }
  }) ?? Promise.resolve());
  let imported: CharacterRig | null = null;
  let disposed = false;
  const abort=new AbortController();
  let disposeJersey:(()=>void)|null=null;
  const atlas=design?jerseyAtlas(design):null;
  if(garment) {
    pending.push(attachJersey(bodyShape,arms,abort.signal,atlas?.texture).then(dispose=>{
      if(disposed){dispose?.();return;}
      disposeJersey=dispose;
      // The authored garment has open armholes. Keep a cloth lining beneath
      // those seams so the round base cannot poke through as the arms move.
      if(dispose)shirtFallback.forEach(part=>part.visible=jerseyLining.has(part));
    }).catch(()=>{ /* Keep the fitted procedural shirt if the garment fails to load. */ }));
  }
  const client=wardrobeClient();
  const catalogModel=costume?.model_id?resolveCatalogModel(costume.model_id):null;
  const model=catalogModel?Promise.resolve(catalogModel):costume?.model_id&&client?client.models.get(costume.model_id):null;
  if(model) {
    pending.push(model.then(model=>loadCustomModel(model,abort.signal,costume?.head==='head-pumpkin',!!costume?.extras?.length)).then(async rig=>{
      if(disposed) {rig.dispose();return;}
      imported=rig;
      // Anything carried (a bag) stays; the built body goes.
      for(const child of [...group.children])if(!child.userData.carried&&!child.userData.characterExtra)group.remove(child);
      group.add(rig.group);
      if(rig.attachments){
        extras.position.set(0,0,0);extras.scale.setScalar(1);rig.attachments.head.add(extras);
        neckwear.position.set(0,0,0);rig.attachments.neck.add(neckwear);
        holidayHead.position.set(0,0,0);holidayHead.scale.setScalar(1);rig.attachments.head.add(holidayHead);
        if(extrasWorn.has('extra-scarf')) {
          scarf.clear();addScarf(scarf,MODEL_SCARF_RING,mesh,cloth);
          scarf.position.set(0,MODEL_SCARF_Y,0);scarf.scale.setScalar(1);rig.attachments.head.add(scarf);
        }
      }
      await rig.ready;
      if(disposed)return;
      group.updateMatrixWorld(true);
      popAbove(group,new THREE.Box3().setFromObject(rig.group).applyMatrix4(group.matrixWorld.clone().invert()).max.y);
    }));
  }


  let reaction:{face:FaceReaction;hold:boolean;started:number|null}|null=null;
  let lastPose:Pose|null=null;
  const react=(which?:FaceReaction|null,hold=false)=>{
    const chosen=which===undefined?chooseReaction(def.costume):which;
    reaction=chosen?{face:chosen,hold,started:null}:null;
  };
  /** Draws the reaction over the pose's own face, and returns how the head turns with it. */
  const showFace=(pose:Pose,time:number)=>{
    if(reactOnCheer&&pose==='cheer'&&lastPose!=='cheer')react();
    lastPose=pose;
    const mood:Mood=pose==='cheer'||pose==='sad'||pose==='fall'?pose:'idle';
    let weight=0;
    if(reaction){
      reaction.started??=time;
      const elapsed=time-reaction.started;
      weight=reactionWeight(elapsed,reaction.hold);
      if(!reaction.hold&&elapsed>=REACTION_LENGTH)reaction=null;
    }
    let turn={x:0,z:0};
    for(const each of faces)turn=each.show(mood,reaction?.face??null,weight,time);
    if(bubble){
      // Pops past full size and settles, then shrinks away as the reaction ends.
      const pop=weight*(1+0.25*Math.sin(Math.min(1,weight)*Math.PI));
      bubble.visible=weight>0;
      bubble.scale.setScalar(BUBBLE*pop);
      return {x:0,z:0};
    }
    return turn;
  };

  let waveStarted: number | null = null;
  const animate = (pose: Pose, time: number, speed: number, waveFor = 3) => {
    if (pose !== 'wave') waveStarted = null;
    if(imported) {imported.animate(pose,time,speed,waveFor);showFace(pose,time);smoke?.(time);return;}
    const turn=showFace(pose,time);
    const bounce = Math.sin(time * 6);
    torso.position.y = 0;
    torso.rotation.set(0, 0, 0);
    head.rotation.set(0, 0, 0);
    legs.forEach((leg) => {leg.position.y=0.16;leg.position.z=0;leg.rotation.set(0,0,0);});
    limbs.forEach(limb=>stretch(limb,ARM_REACH));
    const [left, right] = arms;
    switch (pose) {
      case 'idle':
        torso.position.y = Math.abs(Math.sin(time * 2.2)) * 0.04;
        left.rotation.set(0, 0, -0.35 - Math.sin(time * 2.2) * 0.08);
        right.rotation.set(0, 0, 0.35 + Math.sin(time * 2.2) * 0.08);
        head.rotation.z = Math.sin(time * 1.1) * 0.06;
        break;
      case 'wave': {
        waveStarted ??= time;
        const elapsed = time - waveStarted;
        const weight = Math.max(0, Math.min(1, elapsed / .2, (waveFor - elapsed) / .3));
        left.rotation.set(0, 0, -.35 - Math.sin(time * 2.2) * .08);
        const rest = .35 + Math.sin(time * 2.2) * .08;
        right.rotation.set(-.1 * weight, 0, rest + (2.25 + Math.sin(time * 8) * .3 - rest) * weight);
        head.rotation.z = -.05 * weight;
        break;
      }
      case 'ride': {
        // Balance on the ball: lean into the roll and pinwheel the arms.
        const effort = Math.min(1, speed / 6);
        torso.rotation.x = effort * 0.3;
        torso.position.y = Math.abs(bounce) * 0.03 * effort;
        left.rotation.set(Math.sin(time * 9) * 0.6 * effort, 0, -1.1 + Math.sin(time * 7) * 0.25);
        right.rotation.set(-Math.sin(time * 9) * 0.6 * effort, 0, 1.1 - Math.sin(time * 7) * 0.25);
        legs[0].position.y = 0.16 + Math.max(0, Math.sin(time * 12)) * 0.05 * effort;
        legs[1].position.y = 0.16 + Math.max(0, -Math.sin(time * 12)) * 0.05 * effort;
        break;
      }
      case 'play': {
        // Both hands on the grips of the held controller (controller.ts), thumbs busy, leaning into the game.
        const thumbs = Math.sin(time * 9) * 0.015;
        torso.rotation.z = Math.sin(time * 1.3) * 0.07;
        torso.rotation.x = 0.08 + Math.sin(time * 0.9) * 0.03;
        torso.position.y = Math.abs(Math.sin(time * 1.6)) * 0.015;
        head.rotation.x = 0.12;
        head.rotation.z = Math.sin(time * 1.3) * 0.05;
        // The controller stays still in the group while the torso sways, so each arm re-aims at its grip every frame.
        bodyShape.updateWorldMatrix(true, false);
        arms.forEach((shoulder, i) => {
          // Wrapped round the grip from outside and below, so the hand leaves the buttons in view.
          const side = Math.sign(GRIPS[i].x);
          grip.copy(GRIPS[i]).add(PALM);grip.x += side * PALM_OUT;grip.y -= side * thumbs;
          bodyShape.worldToLocal(group.localToWorld(heldPoint(grip, group.userData.bodyWidth, grip))).sub(shoulder.position);
          const length = grip.length();
          shoulder.quaternion.setFromUnitVectors(DOWN, grip.normalize());
          stretch(limbs[i], length);
        });
        break;
      }
      case 'sit':
        torso.position.y=-.2;torso.rotation.x=.06;
        left.rotation.set(-1.1,0,-.18);right.rotation.set(-1.1,0,.18);
        for(const leg of legs){leg.position.set(leg.position.x,.12,.18);leg.rotation.x=-Math.PI/2;}
        break;
      case 'cheer': {
        const hop = Math.abs(Math.sin(time * 7));
        torso.position.y = hop * 0.25;
        left.rotation.set(0, 0, -2.6 + Math.sin(time * 14) * 0.25);
        right.rotation.set(0, 0, 2.6 - Math.sin(time * 14) * 0.25);
        head.rotation.x = -0.15;
        break;
      }
      case 'sad':
        torso.rotation.x = 0.25;
        head.rotation.x = 0.35;
        left.rotation.set(0, 0, -0.12);
        right.rotation.set(0, 0, 0.12);
        break;
      case 'fall':
        torso.rotation.x = -0.4 + Math.sin(time * 18) * 0.1;
        left.rotation.set(0, 0, -2.2 + Math.sin(time * 22) * 0.9);
        right.rotation.set(0, 0, 2.2 - Math.sin(time * 22 + 1) * 0.9);
        legs[0].position.y = 0.16 + Math.sin(time * 20) * 0.08;
        legs[1].position.y = 0.16 - Math.sin(time * 20) * 0.08;
        break;
    }
    head.rotation.x+=turn.x;head.rotation.z+=turn.z;
    smoke?.(time);
  };

  const dispose = () => {
    disposed=true;abort.abort();pumpkin?.dispose();disposeJersey?.();atlas?.release();imported?.dispose();
    geometries.forEach((g) => g.dispose());
    Object.values(materials).forEach((m) => m.dispose());
    extraMaterials.forEach((m) => m.dispose());
    group.removeFromParent();
  };

  const setBodyWidth = (value:number) => {
    const next=bodyWidth(value),depth=Math.sqrt(next);
    lowerBody.scale.set(next,1,depth);bodyShape.scale.set(next,1,depth);
    if(scarf.parent===torso)fitScarf(next);
    group.userData.bodyWidth=def.costume?.model_id?1:next;
  };
  const ready = Promise.all(pending).then(()=>{});
  // Gameplay keeps its normal fallback; still previews may await and report
  // a failed model instead of displaying the base character as that product.
  void ready.catch(()=>{});
  return { group, animate, dispose, ready, setBodyWidth, react };
}

type Limb = {arm: THREE.Mesh; ends: THREE.Object3D[]};
const PALM = new THREE.Vector3(0, -0.025, -0.02), PALM_OUT = 0.035;
const ARM_REACH = 0.3, DOWN = new THREE.Vector3(0, -1, 0), grip = new THREE.Vector3();
/** A cartoon reach: the arm stretches along itself and the hand moves out, rather than scaling the hand too. */
function stretch(limb: Limb, length: number) {
  const k = length / ARM_REACH;
  limb.arm.scale.y = k;
  limb.arm.position.y = -0.14 * k;
  for (const end of limb.ends) end.position.y = -length;
}

const WAIST_RADIUS=0.428, WAIST_ARC=1.65, WAIST_STRETCH=1.08;
// The belly bulges past the round body, so below the waistband it would poke through the pants; end it at the band instead.
const bellyArc=(costume?:CharacterDef['costume'])=>costume?.pants?Math.acos((0.55+WAIST_RADIUS*WAIST_STRETCH*Math.cos(WAIST_ARC)-0.5)/0.33):Math.PI;

type MeshFactory = (geometry: THREE.BufferGeometry, material: THREE.Material, parent?: THREE.Object3D) => THREE.Mesh;
type Cloth = (color: string) => THREE.Material;

// The scarf rings the join of the round head and body; on an imported model it hangs just under the head's bounds, in head-sized units.
const SCARF_RING = 0.335, MODEL_SCARF_RING = 0.28, MODEL_SCARF_Y = -0.46;
const HOLIDAY_RED = '#c8323c', HOLIDAY_WHITE = '#f4f4f0';

// Head-sized units, as the hats in addFeature: the round head is a 0.4 ball about the origin.
function addHolidayHat(id: string, parent: THREE.Group, mesh: MeshFactory, cloth: Cloth) {
  const santa = id === 'extra-santa-hat';
  const hat = santa
    ? tippedBack(parent, { tilt: HAT_TILT, lift: 0.03 }, { tilt: 0.16, lift: 0 })
    : tippedBack(parent, { tilt: 0.35, lift: 0.04 }, { tilt: 0.12, lift: 0 });
  if (santa) {
    const red = cloth(HOLIDAY_RED), white = cloth(HOLIDAY_WHITE);
    hat.rotation.z = 0.06;
    const brim = mesh(new THREE.TorusGeometry(0.37, 0.075, 10, 32), white, hat);
    brim.rotation.x = Math.PI / 2;
    brim.position.y = 0.2;
    const crown = mesh(new THREE.CylinderGeometry(0.14, 0.36, 0.36, 24), red, hat);
    crown.position.y = 0.38;
    const droop = new THREE.Group();
    droop.position.y = 0.54;
    droop.rotation.z = -1.15;
    hat.add(droop);
    const tip = mesh(new THREE.ConeGeometry(0.14, 0.34, 18), red, droop);
    tip.position.y = 0.16;
    const pom = mesh(new THREE.SphereGeometry(0.085, 12, 10), white, droop);
    pom.position.y = 0.34;
  } else {
    const green = cloth('#2f9e44'), red = cloth(HOLIDAY_RED);
    const band = mesh(new THREE.TorusGeometry(0.37, 0.055, 8, 32), red, hat);
    band.rotation.x = Math.PI / 2;
    band.position.y = 0.2;
    // A zigzag cuff of felt points around the band.
    for (let i = 0; i < 10; i++) {
      const angle = (i / 10) * Math.PI * 2;
      const point = mesh(new THREE.ConeGeometry(0.07, 0.13, 3), red, hat);
      point.position.set(Math.sin(angle) * 0.4, 0.15, Math.cos(angle) * 0.4);
      point.rotation.set(Math.PI, angle, 0);
    }
    const lean = new THREE.Group();
    lean.position.y = 0.22;
    lean.rotation.x = -0.4;
    hat.add(lean);
    const cone = mesh(new THREE.ConeGeometry(0.34, 0.78, 24), green, lean);
    cone.position.y = 0.39;
    const bell = mesh(new THREE.SphereGeometry(0.06, 12, 10), cloth('#f5c542'), lean);
    bell.position.y = 0.8;
  }
}

function addAntlers(parent: THREE.Group, mesh: MeshFactory, cloth: Cloth) {
  const horn = cloth('#8b5a2b');
  const prong = (on: THREE.Object3D, y: number, tilt: number, length: number, radius: number) => {
    const pivot = new THREE.Group();
    pivot.position.y = y;
    pivot.rotation.z = tilt;
    on.add(pivot);
    const stem = mesh(new THREE.CylinderGeometry(radius * 0.7, radius, length, 8), horn, pivot);
    stem.position.y = length / 2;
    const end = mesh(new THREE.SphereGeometry(radius * 0.7, 8, 6), horn, pivot);
    end.position.y = length;
    return pivot;
  };
  for (const side of [-1, 1]) {
    const root = new THREE.Group();
    root.position.set(side * 0.21, 0.28, -0.03);
    parent.add(root);
    const beam = prong(root, 0, side * -0.3, 0.36, 0.036);
    prong(beam, 0.12, side * -0.95, 0.15, 0.024);
    prong(beam, 0.24, side * 0.75, 0.13, 0.022);
  }
}

/** Around the neck at `ring`, knotted at the front with one striped tail hanging down. */
function addScarf(parent: THREE.Group, ring: number, mesh: MeshFactory, cloth: Cloth) {
  const red = cloth(HOLIDAY_RED), white = cloth(HOLIDAY_WHITE);
  const wrap = mesh(new THREE.TorusGeometry(ring, 0.075, 10, 36), red, parent);
  wrap.rotation.x = Math.PI / 2;
  const knot = mesh(new THREE.SphereGeometry(0.08, 12, 10), red, parent);
  knot.scale.set(1, 0.85, 0.7);
  knot.position.set(ring * 0.36, -0.04, ring * 0.95);
  const tail = new THREE.Group();
  tail.position.set(ring * 0.42, -0.06, ring * 0.95 + 0.04);
  tail.rotation.set(-0.25, 0, 0.1);
  parent.add(tail);
  const panel = mesh(new THREE.BoxGeometry(0.12, 0.34, 0.045), red, tail);
  panel.position.y = -0.17;
  for (const y of [-0.24, -0.29]) {
    const stripe = mesh(new THREE.BoxGeometry(0.124, 0.025, 0.049), white, tail);
    stripe.position.y = y;
  }
}
type HatFit = { tilt: number; lift: number };
/** On the round head a hat sits tipped back so its front rim clears the eyes and brows while the back still covers the head. The pumpkin's carved face sits lower, so it keeps `pumpkin`. */
function tippedBack(parent: THREE.Group, round: HatFit, pumpkin: HatFit) {
  const hat = new THREE.Group();
  hat.rotation.x = -round.tilt;
  hat.position.y = round.lift;
  hat.userData.pumpkinFit = pumpkin;
  parent.add(hat);
  return hat;
}
const HAT_TILT = 0.3;

type Materials = Record<'body' | 'accent' | 'belly' | 'white' | 'dark' | 'ink' | 'frames' | 'cheek', THREE.Material>;

const BUBBLE = 0.5;
const isDark = (color: string) => new THREE.Color(color).getHSL({ h: 0, s: 0, l: 0 }).l < 0.25;

function addFeature(feature: Feature, head: THREE.Group, mesh: MeshFactory, m: Materials, glow: (color: string) => THREE.Material) {
  switch (feature) {
    case 'cap': {
      const cap = tippedBack(head, { tilt: HAT_TILT, lift: 0.075 }, { tilt: 0, lift: 0 });
      const crown = mesh(new THREE.SphereGeometry(0.41, 24, 12, 0, Math.PI * 2, 0, Math.PI / 2), m.accent, cap);
      crown.position.y = 0.07;
      const brim = mesh(new THREE.CylinderGeometry(0.26, 0.26, 0.04, 20, 1, false, -Math.PI / 2, Math.PI), m.accent, cap);
      brim.position.set(0, 0.1, 0.24);
      const button = mesh(new THREE.SphereGeometry(0.05, 10, 8), m.white, cap);
      button.position.y = 0.48;
      break;
    }
    case 'leaf': {
      const stem = mesh(new THREE.CylinderGeometry(0.025, 0.03, 0.22, 8), m.accent, head);
      stem.position.set(0, 0.47, 0);
      for (const side of [-1, 1]) {
        const leaf = mesh(new THREE.SphereGeometry(0.16, 14, 10), m.accent, head);
        leaf.scale.set(1, 0.3, 0.55);
        leaf.position.set(side * 0.15, 0.58, 0);
        leaf.rotation.z = side * -0.5;
      }
      break;
    }
    case 'antenna':
      for (const side of [-1, 1]) {
        const stalk = mesh(new THREE.CylinderGeometry(0.02, 0.02, 0.3, 8), m.dark, head);
        stalk.position.set(side * 0.14, 0.48, 0);
        stalk.rotation.z = side * -0.35;
        const tip = mesh(new THREE.SphereGeometry(0.07, 12, 10), m.accent, head);
        tip.position.set(side * 0.2, 0.63, 0);
      }
      break;
    case 'long_ears':
      for (const side of [-1, 1]) {
        const ear = mesh(new THREE.CapsuleGeometry(0.09, 0.4, 4, 12), m.body, head);
        ear.position.set(side * 0.17, 0.55, -0.02);
        ear.rotation.z = side * -0.2;
        const inner = mesh(new THREE.CapsuleGeometry(0.05, 0.3, 4, 10), m.accent, head);
        inner.position.set(side * 0.17, 0.56, 0.05);
        inner.rotation.z = side * -0.2;
        inner.castShadow = false;
      }
      break;
    case 'round_ears':
      for (const side of [-1, 1]) {
        const ear = mesh(new THREE.SphereGeometry(0.15, 16, 12), m.body, head);
        ear.scale.z = 0.5;
        ear.position.set(side * 0.3, 0.3, -0.02);
        const inner = mesh(new THREE.SphereGeometry(0.09, 12, 10), m.accent, head);
        inner.scale.z = 0.4;
        inner.position.set(side * 0.3, 0.3, 0.04);
        inner.castShadow = false;
      }
      break;
    case 'horns':
      for (const side of [-1, 1]) {
        const horn = mesh(new THREE.ConeGeometry(0.08, 0.26, 12), m.accent, head);
        horn.position.set(side * 0.2, 0.4, 0);
        horn.rotation.z = side * -0.45;
      }
      break;
    case 'beanie': {
      const beanie = tippedBack(head, { tilt: HAT_TILT, lift: 0.08 }, { tilt: 0, lift: 0 });
      const cap = mesh(new THREE.SphereGeometry(0.42, 24, 12, 0, Math.PI * 2, 0, Math.PI / 2.1), m.accent, beanie);
      cap.position.y = 0.05;
      const band = mesh(new THREE.TorusGeometry(0.39, 0.06, 10, 28), m.white, beanie);
      band.rotation.x = Math.PI / 2;
      band.position.y = 0.12;
      const pom = mesh(new THREE.SphereGeometry(0.1, 12, 10), m.white, beanie);
      pom.position.y = 0.5;
      break;
    }
    case 'visor': {
      const visor = mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.16, 24, 1, false, -Math.PI * 0.42, Math.PI * 0.84), m.accent, head);
      visor.position.y = 0.06;
      const light = mesh(new THREE.BoxGeometry(0.34, 0.025, 0.02), glow('#7df9ff'), head);
      light.position.set(0, 0.06, 0.425);
      const fin = mesh(new THREE.BoxGeometry(0.05, 0.16, 0.3), m.accent, head);
      fin.position.set(0, 0.42, -0.05);
      break;
    }
    case 'top_hat': {
      const hat = tippedBack(head, { tilt: HAT_TILT * 0.5, lift: 0.3 }, { tilt: 0, lift: 0.1 });
      const brim = mesh(new THREE.CylinderGeometry(0.4, 0.4, 0.04, 28), m.dark, hat);
      brim.position.y = 0.06;
      const crown = mesh(new THREE.CylinderGeometry(0.25, 0.27, 0.42, 24), m.dark, hat);
      crown.position.y = 0.29;
      const band = mesh(new THREE.CylinderGeometry(0.275, 0.275, 0.07, 24), m.accent, hat);
      band.position.y = 0.12;
      break;
    }
    case 'crown': {
      const crown = tippedBack(head, { tilt: HAT_TILT * 0.5, lift: 0.3 }, { tilt: 0, lift: 0.1 });
      const gold = glow('#f5c542');
      const ring = mesh(new THREE.CylinderGeometry(0.27, 0.27, 0.12, 24, 1, true), gold, crown);
      ring.position.y = 0.08;
      for (let i = 0; i < 5; i++) {
        const angle = (i / 5) * Math.PI * 2;
        const point = mesh(new THREE.ConeGeometry(0.06, 0.16, 8), gold, crown);
        point.position.set(Math.sin(angle) * 0.25, 0.21, Math.cos(angle) * 0.25);
        const gem = mesh(new THREE.SphereGeometry(0.035, 8, 6), m.accent, crown);
        gem.position.set(Math.sin(angle) * 0.275, 0.08, Math.cos(angle) * 0.275);
      }
      break;
    }
    case 'wizard': {
      const hat = tippedBack(head, { tilt: HAT_TILT, lift: 0.1 }, { tilt: 0, lift: 0 });
      const brim = mesh(new THREE.CylinderGeometry(0.5, 0.5, 0.03, 28), m.accent, hat);
      brim.position.y = 0.12;
      const cone = mesh(new THREE.ConeGeometry(0.34, 0.7, 24), m.accent, hat);
      cone.position.y = 0.48;
      cone.rotation.z = 0.12;
      const star = mesh(new THREE.OctahedronGeometry(0.06), glow('#fde047'), hat);
      star.position.set(0.06, 0.45, 0.25);
      break;
    }
  }
}
