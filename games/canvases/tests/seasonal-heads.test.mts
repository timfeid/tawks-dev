import {test} from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import {HEADS,headChoices,inSeason,seasonNote,easternMonthDay} from '../src/party/costumes/selection.ts';
import {BUILT,buildHead} from '../src/party/costumes/built-heads.ts';
import {HEAD_PROPORTIONS,swappedHeadWidth} from '../src/party/costumes/head-sizes.ts';

const shade=(color:string)=>new THREE.MeshToonMaterial({color});
const ids=(now:string,worn:string|null=null)=>headChoices(worn,new Date(now)).map(head=>head.id);

test('the pumpkin can be picked until the end of November 15 in New York, then only kept',()=>{
  assert.ok(ids('2026-11-16T04:59:59Z').includes('head-pumpkin'),'11:59pm EST on the 15th');
  assert.ok(!ids('2026-11-16T05:00:00Z').includes('head-pumpkin'));
  assert.ok(ids('2026-11-16T05:00:00Z','head-pumpkin').includes('head-pumpkin'),'still offered to someone wearing it');
  assert.ok(ids('2027-10-01T12:00:00Z').includes('head-pumpkin'),'back next October');
});

test('each season opens and closes on Eastern days, winter across the new year',()=>{
  assert.deepEqual(ids('2026-11-01T12:00:00Z'),[null,'head-pumpkin','head-turkey']);
  assert.deepEqual(ids('2026-11-26T20:00:00Z'),[null,'head-turkey','head-gingerbread'],'Thanksgiving Day');
  assert.deepEqual(ids('2026-12-25T15:00:00Z'),[null,'head-gingerbread','head-snowman','head-penguin']);
  assert.deepEqual(ids('2027-01-01T05:00:00Z'),[null,'head-snowman','head-penguin']);
  assert.deepEqual(ids('2027-03-21T03:59:00Z'),[null,'head-snowman','head-penguin'],'11:59pm EDT on March 20');
  assert.deepEqual(ids('2027-03-21T04:00:00Z'),[null]);
  assert.equal(easternMonthDay(new Date('2026-11-01T03:59:59Z')),'10-31','EDT before the clocks go back');
  assert.ok(inSeason(HEADS[0],new Date('2027-07-04T12:00:00Z')),'the round head is always on offer');
});

test('seasonal heads say when they leave',()=>{
  assert.equal(seasonNote(HEADS.find(head=>head.id==='head-pumpkin')!),'Until Nov 15');
  assert.equal(seasonNote(HEADS.find(head=>head.id==='head-snowman')!),'Until Mar 20');
  assert.equal(seasonNote(HEADS[0]),'');
});

test('every seasonal head besides the pumpkin is built, at the proportions models fit it to',()=>{
  for(const head of HEADS.slice(1)) {
    assert.ok(HEAD_PROPORTIONS[head.id!],`${head.id} has proportions`);
    if(head.id==='head-pumpkin')continue;
    assert.ok(head.id! in BUILT,`${head.id} is built`);
    const built=buildHead(head.id!,shade)!;
    const bounds=new THREE.Box3().setFromObject(built.group.getObjectByName('head-skull')!);
    const size=bounds.getSize(new THREE.Vector3());
    for(const axis of ['x','y','z'] as const)assert.ok(Math.abs(size[axis]-built.size[axis])<0.01,`${head.id} skull ${axis}`);
    assert.ok(Math.abs(built.size.y/built.size.x-HEAD_PROPORTIONS[head.id!].y)<1e-3);
    assert.ok(Math.abs(built.size.z/built.size.x-HEAD_PROPORTIONS[head.id!].z)<1e-3);
    // Feathers and the reaction bubble above them: the face pops up clear of everything drawn.
    const whole=new THREE.Box3().setFromObject(built.group);
    assert.ok(built.top>=whole.max.y-0.001,`${head.id} reaches ${whole.max.y}, past top ${built.top}`);
    // Big enough to replace the round head, small enough to keep its footing on the body.
    assert.ok(size.x>0.8&&size.x<0.9&&whole.min.y>-0.44,head.id!);
    built.dispose();
    assert.equal(built.group.parent,null);
  }
  assert.equal(buildHead('head-pumpkin',shade),null);
  assert.equal(buildHead('head-skull',shade),null);
});

test('a swapped head covers an imported model head of any shape',()=>{
  for(const id of Object.keys(HEAD_PROPORTIONS)) {
    const tall=new THREE.Vector3(0.3,0.6,0.3),wide=new THREE.Vector3(0.7,0.4,0.5);
    for(const size of [tall,wide]) {
      const width=swappedHeadWidth(id,size);
      assert.ok(width>=size.x&&width*HEAD_PROPORTIONS[id].y>=size.y,id);
      if(id!=='head-pumpkin')assert.ok(width*HEAD_PROPORTIONS[id].z>=size.z,`${id} is deep enough`);
    }
  }
});
