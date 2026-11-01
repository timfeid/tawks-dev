import {test} from 'node:test';
import assert from 'node:assert/strict';
import {vacuumPoint} from '../src/party/board/vacuum-geometry.ts';

test('particles stay within the authoritative circular sector at all aim angles',()=>{
  const origin={x:7,y:1.3,z:-11};
  for(const yaw of [-7,-Math.PI,-1,0,1,Math.PI,7])for(const cone of [.22,.33])for(let i=0;i<=100;i++)for(const spread of [-1,-.5,0,.5,1]){
    const p=vacuumPoint(origin,yaw,14,cone,i/100,spread,2);
    const dx=p.x-origin.x,dz=p.z-origin.z;
    assert.ok(Math.hypot(dx,dz)<=14+1e-10,'outer particles must not suggest extra reach');
    if(i>0)assert.ok(Math.abs(Math.atan2(Math.sin(Math.atan2(dx,dz)-yaw),Math.cos(Math.atan2(dx,dz)-yaw)))<=cone+1e-10,'no particles outside the valid cone');
  }
});
test('world reticle marks exactly the centre of the far end, even for upgraded cones',()=>{
  for(const cone of [.22,.33]){
    const p=vacuumPoint({x:3,y:2,z:5},Math.PI/2,14,cone,1,0);
    assert.ok(Math.abs(p.x-17)<1e-10);assert.equal(p.y,2);assert.ok(Math.abs(p.z-5)<1e-10);
  }
});
