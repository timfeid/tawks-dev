import {test} from 'node:test';
import assert from 'node:assert/strict';
import {characterSelection,hasUnownedSelection,toggleExtra} from '../src/party/costumes/selection.ts';
import {itemFocus} from '../src/party/costumes/framing.ts';
import catalog from '../src/party/costumes/catalog.json' with {type:'json'};

const models=['model-robot','model-ninja','model-cowboy','model-goblin','model-knight','model-pirate'];

test('previewing a whole character preserves color, its carried bag and its head while removing fitted clothing',()=>{
  const original={color:'vex',hat:'hat-cap',shirt:'shirt-hockey',pants:'pants-jeans',shoes:'shoes-skates',gloves:'gloves-hockey',extras:['extra-cigar'],bag:'bag-small',head:'head-pumpkin'};
  assert.deepEqual(characterSelection(original,'model-robot'),{color:'vex',model_id:'model-robot',bag:'bag-small',head:'head-pumpkin',extras:['extra-cigar']});
  assert.deepEqual(characterSelection({color:'vex'},'model-robot'),{color:'vex',model_id:'model-robot',bag:null,head:null});
  assert.equal(original.shirt,'shirt-hockey','preview does not mutate the saved base look');
});

test('every store character needs its own entitlement before Apply can enable',()=>{
  for(const id of models) {
    const draft={color:'sunny',model_id:id};
    assert.equal(hasUnownedSelection(draft,[],models),true);
    assert.equal(hasUnownedSelection(draft,['model-unrelated'],models),true);
    assert.equal(hasUnownedSelection(draft,[id],models),false);
  }
});

test('free originals and personal imported models retain their existing ownership paths',()=>{
  assert.equal(hasUnownedSelection({color:'blip'},[],models),false);
  assert.equal(hasUnownedSelection({color:'blip',model_id:'import-personal-123'},[],models),false,'personal model access is validated by the host API');
  assert.equal(hasUnownedSelection({color:'blip',hat:'hat-cap'},[],models),true);
  assert.equal(hasUnownedSelection({color:'blip',hat:'hat-cap'},['hat-cap'],models),false);
});

test('a holiday hat swaps out the other one while every other extra stacks',()=>{
  assert.deepEqual(toggleExtra(['extra-glasses','extra-santa-hat'],'extra-elf-hat'),['extra-glasses','extra-elf-hat']);
  assert.deepEqual(toggleExtra(['extra-santa-hat'],'extra-antlers'),['extra-santa-hat','extra-antlers']);
  assert.deepEqual(toggleExtra(['extra-scarf','extra-elf-hat'],'extra-elf-hat'),['extra-scarf']);
});

test('a whole character keeps the chosen reaction and its shuffle',()=>{
  assert.deepEqual(characterSelection({color:'vex',hat:'hat-cap',reaction:'reaction-wink',shuffle_reactions:true},'model-robot'),{color:'vex',model_id:'model-robot',bag:null,head:null,reaction:'reaction-wink',shuffle_reactions:true});
});

test('achievement costumes say how to earn them and frame on the part they are worn',()=>{
  const earned=(catalog as {id:string;unlocked_by?:string;unlock?:string}[]).filter(item=>item.unlocked_by);
  assert.deepEqual(earned.map(item=>item.id),['hat-crown','hat-wizard','extra-medal','extra-halo']);
  for(const item of earned) {
    assert.ok(item.unlock&&!item.unlock.includes('—'),`${item.id} names its achievement`);
    const worn=item.id.startsWith('hat-')?{color:'blip',hat:item.id}:{color:'blip',extras:[item.id]};
    assert.equal(hasUnownedSelection(worn,[],models),true,'earned items still need ownership');
  }
  assert.equal(itemFocus('extra-medal'),'neck');
  assert.equal(itemFocus('extra-halo'),'head');
  assert.equal(itemFocus('extra-headphones'),'head');
});
