import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import * as THREE from 'three';
import {buildFace,FACE_REACTIONS,REACTION_LENGTH,REACTION_IN,reactionWeight,type FaceReaction,type Mood} from '../src/party/reactions/face.ts';

const catalog:{id:string;price:number;animation:string}[]=JSON.parse(readFileSync(new URL('../src/party/reactions/catalog.json',import.meta.url),'utf8'));

function face(body='#f7c548'){
  const group=new THREE.Group(),meshes:THREE.Mesh[]=[];
  const basic=(color:string)=>new THREE.MeshBasicMaterial({color});
  const ink=basic(new THREE.Color(body).getHSL({h:0,s:0,l:0}).l<0.25?'#f4f4f7':'#1b1b24');
  const m={body:basic(body),white:basic('#ffffff'),dark:basic('#1b1b24'),ink,cheek:basic('#ff8fa3'),mouth:basic('#5a1626')};
  const mesh=(g:THREE.BufferGeometry,material:THREE.Material,parent:THREE.Object3D=group)=>{const part=new THREE.Mesh(g,material);parent.add(part);meshes.push(part);return part;};
  const built=buildFace(group,m,mesh,(color,opacity)=>new THREE.MeshBasicMaterial({color,transparent:opacity!==undefined,opacity:opacity??1}));
  const shown=()=>{group.updateMatrixWorld(true);return meshes.filter(part=>{let node:THREE.Object3D|null=part;while(node){if(!node.visible)return false;node=node.parent;}return part.scale.x>0.001;});};
  return {group,meshes,built,shown,ink};
}

test('every reaction in the store catalog has a face, and every face is sold',()=>{
  assert.deepEqual(catalog.map(item=>item.animation).sort(),[...FACE_REACTIONS].sort());
  for(const item of catalog){assert.equal(item.id,`reaction-${item.animation}`);assert.equal(item.price,2);}
});

test('a reaction eases in, holds and eases back to nothing',()=>{
  assert.equal(reactionWeight(-0.1),0);
  assert.equal(reactionWeight(0),0);
  assert.ok(reactionWeight(REACTION_IN/2)>0&&reactionWeight(REACTION_IN/2)<1);
  assert.equal(reactionWeight(1),1);
  assert.ok(reactionWeight(REACTION_LENGTH-0.1)<1);
  assert.equal(reactionWeight(REACTION_LENGTH),0);
  assert.equal(reactionWeight(0,true),1,'a held reaction is on from the first frame');
  assert.equal(reactionWeight(60,true),1);
});

test('each reaction changes the face and the face returns exactly to the mood after it',()=>{
  for(const mood of ['idle','cheer','sad','fall'] as Mood[]){
    const {built,shown}=face();
    built.show(mood,null,0,0);
    const before=shown().map(part=>part.uuid+part.scale.toArray().join()+part.position.toArray().join());
    for(const reaction of FACE_REACTIONS){
      built.show(mood,reaction,1,0.7);
      const during=shown().map(part=>part.uuid+part.scale.toArray().join()+part.position.toArray().join());
      assert.notDeepEqual(during,before,`${reaction} over ${mood}`);
      built.show(mood,null,0,1);
      assert.deepEqual(shown().map(part=>part.uuid+part.scale.toArray().join()+part.position.toArray().join()),before,`${reaction} eases back to ${mood}`);
    }
  }
});

test('closed and replaced eyes hide the open ones, and the wink keeps one',()=>{
  const eyesOpen=(f:ReturnType<typeof face>)=>f.shown().filter(part=>part.geometry instanceof THREE.SphereGeometry&&(part.geometry.parameters.radius===0.05)).length;
  const expected:Partial<Record<FaceReaction,number>>={wink:1,'big-laugh':0,'starry-eyes':0,'heart-eyes':0,shocked:2,'smug-grin':2};
  for(const [reaction,pupils] of Object.entries(expected)){
    const f=face();
    f.built.show('idle',reaction as FaceReaction,1,0);
    assert.equal(eyesOpen(f),pupils,reaction);
  }
});

test('reaction strokes use the skin-aware ink, so they read on a light or a dark body',()=>{
  for(const reaction of ['wink','big-laugh','smug-grin','blush'] as FaceReaction[]){
    const f=face('#2b2b35');
    f.built.show('idle',reaction,1,0);
    const strokes=f.shown().filter(part=>part.material===f.ink);
    assert.ok(strokes.length>=3,`${reaction}: its eyes or mouth and the brows`);
  }
});

test('the steam and the z drift on their own while the face is held',()=>{
  for(const reaction of ['angry-steam','sleepy'] as FaceReaction[]){
    const f=face();
    f.built.show('idle',reaction,1,0.1);
    const at=f.shown().filter(part=>(part.material as THREE.MeshBasicMaterial).transparent).map(part=>part.getWorldPosition(new THREE.Vector3()).y);
    f.built.show('idle',reaction,1,0.6);
    const later=f.shown().filter(part=>(part.material as THREE.MeshBasicMaterial).transparent).map(part=>part.getWorldPosition(new THREE.Vector3()).y);
    assert.ok(at.length>0,reaction);
    assert.notDeepEqual(at,later,reaction);
  }
});
