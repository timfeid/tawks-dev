/** Rectangular street footprints. A shared grid gives each patch of paving
 * one owner, including intersections, and lets the mesh follow the ground. */
export type PavingRect={x0:number;x1:number;z0:number;z1:number};
export function pavingTiles(rectangles:PavingRect[],cutouts:PavingRect[]=[],maxSpan=2):PavingRect[]{
  if(!rectangles.length)return[];
  const all=[...rectangles,...cutouts];
  const split=(values:number[])=>{
    const edges=[...new Set(values)].sort((a,b)=>a-b),out:number[]=[];
    for(let i=1;i<edges.length;i++){
      const a=edges[i-1],b=edges[i],n=Math.ceil((b-a)/maxSpan);
      if(i===1)out.push(a);
      for(let step=1;step<=n;step++)out.push(a+(b-a)*step/n);
    }
    return out;
  };
  const xs=split(all.flatMap(r=>[r.x0,r.x1])),zs=split(all.flatMap(r=>[r.z0,r.z1]));
  const contains=(r:PavingRect,x:number,z:number)=>x>r.x0&&x<r.x1&&z>r.z0&&z<r.z1;
  const tiles:PavingRect[]=[];
  for(let i=1;i<xs.length;i++)for(let j=1;j<zs.length;j++){
    const x=(xs[i-1]+xs[i])/2,z=(zs[j-1]+zs[j])/2;
    if(rectangles.some(r=>contains(r,x,z))&&!cutouts.some(r=>contains(r,x,z)))tiles.push({x0:xs[i-1],x1:xs[i],z0:zs[j-1],z1:zs[j]});
  }
  return tiles;
}
