import {test} from 'node:test';
import assert from 'node:assert/strict';
import {characterAppearanceKey} from '../src/party/appearance.ts';
const base:any={id:'clover',name:'Green',body:'#48c16e',accent:'#338855',belly:'#aaffbb',feature:'none',costume:{color:'clover',model_id:'model-robot'}};

test('identical room snapshots, display-name updates and costume field order keep one appearance identity',()=>{
  const key=characterAppearanceKey(base);
  for(let i=0;i<100;i++){
    const packet={...structuredClone(base),name:'Display name '+i,costume:{model_id:'model-robot',hat:null,color:'clover'}};
    assert.equal(characterAppearanceKey(packet),key);
  }
});

test('every actual fitted outfit or whole-model change invalidates the appearance identity',()=>{
  const key=characterAppearanceKey(base);
  for(const [field,value] of Object.entries({color:'vex',hat:'hat-cap',shirt:'shirt-hockey',pants:'pants-jeans',shoes:'shoes-skates',gloves:'gloves-hockey',extras:['extra-cigar'],bag:'bag-small',model_id:'model-knight',head:'head-pumpkin'}))assert.notEqual(characterAppearanceKey({...base,costume:{...base.costume,[field]:value}}),key,field);
  for(const field of ['id','body','accent','belly','feature'])assert.notEqual(characterAppearanceKey({...base,[field]:'changed'}),key,field);
});

test('a changed PuckLab club design rebuilds the character, other shirts ignore it',()=>{
  const worn={...base,costume:{color:'clover',shirt:'shirt-pucklab'}};
  const synced={...worn,costume:{...worn.costume,jersey:{body:'#123B74',sleeve_stripes:[],hem_stripes:[]}}};
  assert.notEqual(characterAppearanceKey(worn),characterAppearanceKey(synced),'default to synced');
  assert.notEqual(characterAppearanceKey(synced),characterAppearanceKey({...synced,costume:{...synced.costume,jersey:{...synced.costume.jersey,body:'#FFFFFF'}}}));
  const tee={...base,costume:{color:'clover',shirt:'shirt-tee'}};
  assert.equal(characterAppearanceKey(tee),characterAppearanceKey({...tee,costume:{...tee.costume,jersey:{body:'#123B74',sleeve_stripes:[],hem_stripes:[]}}}));
});

test('default and explicit empty fitted slots have the same appearance',()=>{
  const free={...base,costume:null};
  assert.equal(characterAppearanceKey(free),characterAppearanceKey({...base,costume:{color:base.id,hat:null,shirt:undefined,pants:null,shoes:null,gloves:null,extras:[],bag:null,model_id:null}}));
});

// Extras are a set of worn items; packet ordering must not rebuild the character.
test('stacked extras keep a stable identity when packet order changes',()=>{
  const dressed={...base,costume:{...base.costume,extras:['extra-cigar','extra-scarf']}};
  assert.equal(characterAppearanceKey(dressed),characterAppearanceKey({...dressed,costume:{...dressed.costume,extras:['extra-scarf','extra-cigar']}}));
});

test('choosing another reaction or shuffling rebuilds the character, a pool alone does not',()=>{
  const plain={...base,costume:{color:'clover'}};
  const winking={...plain,costume:{...plain.costume,reaction:'reaction-wink'}};
  assert.notEqual(characterAppearanceKey(plain),characterAppearanceKey(winking));
  const shuffling={...winking,costume:{...winking.costume,shuffle_reactions:true,reaction_pool:['reaction-wink','reaction-blush']}};
  assert.notEqual(characterAppearanceKey(winking),characterAppearanceKey(shuffling));
  assert.equal(characterAppearanceKey(shuffling),characterAppearanceKey({...shuffling,costume:{...shuffling.costume,reaction_pool:['reaction-blush','reaction-wink']}}));
  assert.equal(characterAppearanceKey(winking),characterAppearanceKey({...winking,costume:{...winking.costume,reaction_pool:['reaction-blush']}}));
});
