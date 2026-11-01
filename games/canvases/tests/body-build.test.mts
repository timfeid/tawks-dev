import {test} from 'node:test';
import assert from 'node:assert/strict';
import {bodyWidth} from '../src/party/costumes/body-build.ts';
import {characterAppearanceKey} from '../src/party/appearance.ts';
import {characterSelection} from '../src/party/costumes/selection.ts';
import {pumpkinFit} from '../src/party/costumes/pumpkin-fit.ts';
import {readFileSync} from 'node:fs';

test('old outfits keep the same build; malformed and extreme values cannot distort a character',()=>{
  for(const value of [null,undefined,NaN,Infinity])assert.equal(bodyWidth(value),1);
  assert.equal(bodyWidth(.1),.7);assert.equal(bodyWidth(10),1.4);
  for(const width of [.7,1,1.4])assert.equal(bodyWidth(width),width);
  const def={id:'clover',name:'Green',body:'#00ff00',accent:'#fff',belly:'#fff',feature:'none' as const};
  assert.equal(characterAppearanceKey(def),characterAppearanceKey({...def,costume:{color:'clover',body_width:1}}));
  assert.notEqual(characterAppearanceKey(def),characterAppearanceKey({...def,costume:{color:'clover',body_width:1.4}}));
});

test('switching to a whole character preserves the chosen build for returning to Original',()=>{
  assert.equal(characterSelection({color:'clover',body_width:.8,shirt:'shirt-tee'},'model-robot').body_width,.8);
});

test('the bundled pumpkin is centered on its skull, not its protruding stalk',()=>{
  const file=readFileSync(new URL('../src/party/costumes/models/pumpkin-head.glb',import.meta.url));
  const jsonLength=file.readUInt32LE(12), gltf=JSON.parse(file.subarray(20,20+jsonLength).toString());
  const accessor=gltf.accessors[gltf.meshes[0].primitives[0].attributes.POSITION];
  const view=gltf.bufferViews[accessor.bufferView], start=28+jsonLength+(view.byteOffset??0)+(accessor.byteOffset??0);
  const points=Array.from({length:accessor.count},(_,i)=>({x:file.readFloatLE(start+i*(view.byteStride??12)),y:file.readFloatLE(start+i*(view.byteStride??12)+4),z:file.readFloatLE(start+i*(view.byteStride??12)+8)}));
  const fit=pumpkinFit(points,.94);
  assert.ok(Math.abs(fit.center.y-.55)<.005,'Skull center must not be pulled upward by the stalk');
  assert.ok(Math.abs(fit.size.x-.94)<.001);
  assert.ok(fit.size.y>.68&&fit.size.y<.71,'Hat and neck fit use the skull height');
  const stalkTop=Math.max(...points.map(p=>p.y));
  assert.ok(stalkTop>fit.center.y+fit.size.y/(2*fit.scale)+.15,'The stalk remains above the skull');
});
