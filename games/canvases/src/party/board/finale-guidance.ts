import type{MapFinaleView,FinaleRacer}from'../protocol';
export type RoadArrow={x:number;z:number;yaw:number};
const spacing=2.4;
/** Ordered authoritative waypoints, never a straight shortcut to a distant
 * checkpoint. Arrow positions are anchored to road segments, so they do not
 * swim beneath the runner as room packets arrive. */
export function finaleGuidance(view:MapFinaleView,racer:FinaleRacer|null|undefined){
 if(!racer||racer.finished||racer.dnf||racer.next_checkpoint<=0||racer.next_checkpoint>=view.checkpoints.length)return null;
 const checkpoint=view.checkpoints[racer.next_checkpoint],previous=view.checkpoints[racer.next_checkpoint-1];
 const via=(checkpoint.via?.length?checkpoint.via:[[checkpoint.x,checkpoint.z]]).filter(point=>point.length===2&&point.every(Number.isFinite));
 if(!via.length)return null;
 const tail=via.at(-1)!;if(Math.hypot(tail[0]-checkpoint.x,tail[1]-checkpoint.z)>.01)via.push([checkpoint.x,checkpoint.z]);
 const points:[[number,number],...[number,number][]]=[[previous.x,previous.z],...via.map(point=>[point[0],point[1]]as[number,number])];
 const waypoint=Math.max(0,Math.min(points.length-2,Math.floor(racer.waypoint??0)));
 const arrows:RoadArrow[]=[];let distance=0,progress=0;
 for(let segment=waypoint;segment<points.length-1;segment++){
  const a=points[segment],b=points[segment+1],dx=b[0]-a[0],dz=b[1]-a[1],length=Math.hypot(dx,dz);if(length<.001)continue;
  const along=segment===waypoint?Math.max(0,Math.min(length,((racer.x-a[0])*dx+(racer.z-a[1])*dz)/length)):0;
  if(segment===waypoint){progress=along;distance+=Math.hypot(racer.x-(a[0]+dx*along/length),racer.z-(a[1]+dz*along/length));}
  distance+=length-along;
  for(let step=spacing*.5;step<length-.25&&arrows.length<128;step+=spacing){
   if(step<along+.4)continue;
   arrows.push({x:a[0]+dx*step/length,z:a[1]+dz*step/length,yaw:Math.atan2(dx,dz)});
  }
 }
 return{arrows,distance:Math.max(0,distance-checkpoint.radius),next:points[waypoint+1],waypoint,progressBucket:Math.max(0,Math.ceil((progress+.4-spacing*.5)/spacing)),finish:racer.next_checkpoint===view.checkpoints.length-1};
}
