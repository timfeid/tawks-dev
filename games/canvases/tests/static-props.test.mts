import {test}from'node:test';import assert from'node:assert/strict';import * as THREE from'three';
import{StaticProps}from'../src/party/board/static-props.ts';
test('building batches retain camera hiding and leave cached GLTF resources owned by the cache',()=>{
 const scene=new THREE.Group(),geometry=new THREE.BoxGeometry(1,2,1),material=new THREE.MeshStandardMaterial();
 let freed=0;geometry.addEventListener('dispose',()=>freed++);material.addEventListener('dispose',()=>freed++);
 const roots=Array.from({length:3},(_,i)=>{const root=new THREE.Group();root.name='board-model-town-building-a';root.position.x=i*3;root.add(new THREE.Mesh(geometry,material));scene.add(root);return root;});
 const batches=new StaticProps(scene,new Set(roots));batches.update(1);
 const instances=scene.children.find(o=>(o as THREE.InstancedMesh).isInstancedMesh) as THREE.InstancedMesh;
 assert.ok(instances);assert.equal(instances.count,3);assert.ok(roots.every(r=>!r.children[0].visible));
 roots[1].visible=false;batches.update(2);const matrix=new THREE.Matrix4();instances.getMatrixAt(1,matrix);assert.equal(matrix.elements[0],0,'hidden buildings must not render through the camera');
 roots[1].visible=true;batches.update(3);instances.getMatrixAt(1,matrix);assert.equal(matrix.elements[0],1);assert.equal(matrix.elements[12],3);
 batches.dispose();assert.ok(roots.every(r=>r.children[0].visible));assert.equal(freed,0,'shared geometry and materials survive scene disposal');geometry.dispose();material.dispose();
});
