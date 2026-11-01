import {test}from'node:test';import assert from'node:assert/strict';import{FrameBudget}from'../src/party/frame-budget.ts';
test('slow frames reduce resolution within bounds instead of changing gameplay time',()=>{const b=new FrameBudget(1.25);let n=0,changes=0;for(let i=0;i<900;i++){n+=33.3;if(b.sample(n)!==null)changes++;}assert.equal(b.ratio,.75);assert.equal(changes,4);});
test('fast rendering recovers slowly and never exceeds the device budget',()=>{const b=new FrameBudget(.75,1.25);for(let n=1;n<60000;n+=12)b.sample(n);assert.equal(b.ratio,1.25);});
test('background-tab gaps do not count as expensive rendered frames',()=>{const b=new FrameBudget(1.25);for(let n=1;n<4000;n+=16.7)b.sample(n);b.sample(100000);for(let n=100016;n<105000;n+=16.7)b.sample(n);assert.equal(b.ratio,1.25);});
