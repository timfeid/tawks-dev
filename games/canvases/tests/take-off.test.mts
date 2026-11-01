import {test} from 'node:test';
import assert from 'node:assert/strict';
import {takeOff,toggleExtra} from '../src/party/costumes/selection.ts';

const worn={color:'sunny',hat:'hat-cap',shirt:'shirt-tee',pants:'pants-jeans',shoes:'shoes-boots',gloves:'gloves-mitts',bag:'bag-tote',extras:['extra-cigar','extra-glasses','extra-bowtie'],head:'head-pumpkin',reaction:'reaction-wink'};

test('taking off a single-slot item empties only that slot',()=>{
  for(const [slot,id] of [['hat','hat-cap'],['shirt','shirt-tee'],['pants','pants-jeans'],['shoes','shoes-boots'],['gloves','gloves-mitts'],['bag','bag-tote']] as const){
    const look=takeOff(worn,id);
    assert.equal(look[slot],null);
    assert.deepEqual({...look,[slot]:worn[slot]},worn,`${slot} is the only change`);
  }
});

test('taking off one extra keeps the others on, in order',()=>{
  assert.deepEqual(takeOff(worn,'extra-glasses').extras,['extra-cigar','extra-bowtie']);
  assert.deepEqual(takeOff(takeOff(worn,'extra-glasses'),'extra-cigar').extras,['extra-bowtie']);
  assert.deepEqual(toggleExtra(worn.extras,'extra-cigar'),['extra-glasses','extra-bowtie'],'toggling a worn extra agrees');
});

test('a character falls back to Original and a head to the round head, never to nothing',()=>{
  const character=takeOff({color:'clover',model_id:'model-robot',bag:'bag-tote',extras:['extra-cigar']},'model-robot');
  assert.deepEqual(character,{color:'clover',model_id:null,bag:'bag-tote',extras:['extra-cigar']});
  assert.equal(takeOff(worn,'head-pumpkin').head,null);
  assert.equal(takeOff(worn,'head-pumpkin').color,'sunny');
});

test('a worn reaction goes back to the plain face',()=>{
  assert.equal(takeOff(worn,'reaction-wink').reaction,null);
});

test('the club jersey design goes with the shirt',()=>{
  const look=takeOff({color:'sunny',shirt:'shirt-pucklab',jersey:{} as never},'shirt-pucklab');
  assert.equal(look.shirt,null);
  assert.equal(look.jersey,null);
  const design={} as never;
  assert.equal(takeOff({color:'sunny',shirt:'shirt-pucklab',jersey:design},'hat-cap').jersey,design,'kept while the shirt stays');
});

test('an item not worn changes nothing',()=>{
  assert.deepEqual(takeOff(worn,'hat-crown'),worn);
});

test('the input costume is left untouched',()=>{
  const before=structuredClone(worn);
  takeOff(worn,'extra-cigar');
  takeOff(worn,'hat-cap');
  assert.deepEqual(worn,before);
});
