import{test}from'node:test';import assert from'node:assert/strict';import{readFileSync}from'node:fs';
import * as THREE from'three';
import{Terrain}from'../src/party/board/terrain.ts';import type{Design}from'../src/party/board/design.ts';
test('flat city ground uses a street-scale mesh while retaining the movement surface',()=>{
 const design=JSON.parse(readFileSync(new URL('../src/party/board/maps/frozen.json',import.meta.url),'utf8')) as Design;
 const terrain=new Terrain({cx:0,cz:0,halfW:86,halfD:86},[], -3,1,'frozen',design);const mesh=terrain.build();
 const vertices=mesh.geometry.attributes.position;assert.ok(vertices.count<25000,'flat streets do not need the mountain mesh');
 for(let i=0;i<vertices.count;i+=37){const x=vertices.getX(i),z=vertices.getZ(i);assert.ok(Math.abs(vertices.getY(i)-terrain.height(x,z))<0.00001);}
 // Compare the rendered triangle height against the authoritative surface
 // between vertices throughout the playable streets, not only at vertices.
 const ray=new THREE.Raycaster();mesh.updateMatrixWorld();let worst=0;
 for(let x=-65;x<66;x+=7.1)for(let z=-65;z<66;z+=7.1){ray.set(new THREE.Vector3(x,10,z),new THREE.Vector3(0,-1,0));const hit=ray.intersectObject(mesh)[0];assert.ok(hit);worst=Math.max(worst,Math.abs(hit.point.y-terrain.height(x,z)));}
 assert.ok(worst<0.1,`rendered floor must stay within 10cm of the movement surface: ${worst}`);
 mesh.geometry.dispose();(mesh.material as import('three').Material).dispose();
});
