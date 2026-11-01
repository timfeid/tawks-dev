import {Bone,Object3D,Quaternion,Vector3} from 'three';

/** Pose recognized humanoid joints in model space, independent of their bind
 * axes. Restore before the mixer updates so leaving the seat never leaves
 * residual bone rotations on a model with sparse animation tracks. */
export function seatedPose(root:Object3D){
  const joints:{bone:Bone;child:Bone;direction:Vector3}[]=[];
  root.traverse(node=>{
    if(!(node as Bone).isBone)return;
    const bone=node as Bone,child=bone.children.find(node=>(node as Bone).isBone) as Bone|undefined;
    if(!child)return;
    const name=bone.name.toLowerCase().replace(/[_ .-]/g,'');
    const side=/(left|l)$/.test(name)?-1:1;
    const direction=/upperleg|upleg|thigh/.test(name)?new Vector3(side*.08,0,1):/lowerleg|calf|shin/.test(name)?new Vector3(0,-1,0):/upperarm/.test(name)?new Vector3(side*.2,-.25,.7):/lowerarm|forearm/.test(name)?new Vector3(side*.04,.12,1):null;
    if(direction)joints.push({bone,child,direction:direction.normalize()});
  });
  const original=new Map<Bone,Quaternion>();
  return {
    restore(){for(const[bone,rotation]of original)bone.quaternion.copy(rotation);original.clear();},
    apply(){
      root.updateWorldMatrix(true,true);
      const rotation=root.getWorldQuaternion(new Quaternion());
      for(const{bone,child,direction}of joints){
        original.set(bone,bone.quaternion.clone());
        const current=child.getWorldPosition(new Vector3()).sub(bone.getWorldPosition(new Vector3())).normalize();
        const target=direction.clone().applyQuaternion(rotation);
        const world=new Quaternion().setFromUnitVectors(current,target).multiply(bone.getWorldQuaternion(new Quaternion()));
        const parent=bone.parent?.getWorldQuaternion(new Quaternion()).invert()??new Quaternion();
        bone.quaternion.copy(parent.multiply(world));bone.updateWorldMatrix(false,true);
      }
    },
  };
}
