import {test} from 'node:test';import assert from 'node:assert/strict';
import {Bone,Group,Quaternion,Vector3} from 'three';
import {seatedPose} from '../src/party/models/seated.ts';

test('seated pose uses model directions despite arbitrary bind axes and restores exactly',()=>{
 const root=new Group();root.rotation.y=.8;
 const hip=new Bone();hip.rotation.set(.2,.3,.1);root.add(hip);
 const thigh=new Bone();thigh.name='UpperLeg.L';thigh.rotation.set(.3,.1,.7);hip.add(thigh);
 const calf=new Bone();calf.name='LowerLeg.L';calf.position.set(.2,-1,.1);thigh.add(calf);
 const foot=new Bone();foot.position.set(.1,-1,-.2);calf.add(foot);
 const before=[thigh.quaternion.clone(),calf.quaternion.clone()],seat=seatedPose(root);
 for(let i=0;i<20;i++){
  seat.apply();root.updateMatrixWorld(true);
  const direction=calf.getWorldPosition(new Vector3()).sub(thigh.getWorldPosition(new Vector3())).normalize();
  const desired=new Vector3(-.08,0,1).normalize().applyQuaternion(root.getWorldQuaternion(new Quaternion()));
  assert.ok(direction.distanceTo(desired)<1e-6);
  const down=foot.getWorldPosition(new Vector3()).sub(calf.getWorldPosition(new Vector3())).normalize();assert.ok(down.distanceTo(new Vector3(0,-1,0))<1e-6);
  seat.restore();assert.ok(thigh.quaternion.angleTo(before[0])<1e-6);assert.ok(calf.quaternion.angleTo(before[1])<1e-6);
 }
});
