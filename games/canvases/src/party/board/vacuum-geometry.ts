export type Point3 = {x:number;y:number;z:number};

/** The server's hit region is a horizontal circular sector, not a triangle
 * with a flat end. Keep every particle inside that exact angle and radius. */
export function vacuumPoint(origin:Point3,yaw:number,range:number,cone:number,distance:number,spread:number,lift=0):Point3 {
  const radius=Math.max(0,range)*Math.max(0,Math.min(1,distance));
  const angle=yaw+Math.max(-1,Math.min(1,spread))*cone;
  return {x:origin.x+Math.sin(angle)*radius,y:origin.y+lift,z:origin.z+Math.cos(angle)*radius};
}
