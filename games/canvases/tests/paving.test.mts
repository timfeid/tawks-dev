import {test} from 'node:test';
import assert from 'node:assert/strict';
import {pavingTiles} from '../src/party/board/paving.ts';

test('crossing streets have one continuous surface without overlapping faces',()=>{
  const tiles=pavingTiles([{x0:-10,x1:10,z0:-2,z1:2},{x0:-2,x1:2,z0:-10,z1:10}]);
  assert.equal(tiles.reduce((area,r)=>area+(r.x1-r.x0)*(r.z1-r.z0),0),144);
  for(let i=0;i<tiles.length;i++)for(const b of tiles.slice(i+1)){
    const a=tiles[i];assert.ok(Math.min(a.x1,b.x1)<=Math.max(a.x0,b.x0)||Math.min(a.z1,b.z1)<=Math.max(a.z0,b.z0));
  }
  assert.ok(tiles.every(r=>r.x1-r.x0<=2&&r.z1-r.z0<=2),'short patches follow terrain rather than floating across it');
});
test('sidewalk and snow surfaces stop at the asphalt, including corner overlaps',()=>{
  const road={x0:-2,x1:2,z0:-10,z1:10};
  const tiles=pavingTiles([{x0:-8,x1:8,z0:3,z1:5},{x0:3,x1:5,z0:-8,z1:8}],[road]);
  for(const r of tiles)assert.ok(r.x1<=road.x0||r.x0>=road.x1||r.z1<=road.z0||r.z0>=road.z1);
});
