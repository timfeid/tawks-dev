import type{RoomView}from'./protocol';
/** Event participation and purchases are always authorized by the server. */
export function candyVanState(room:RoomView|null,seat:number|null){
  const board=room?.board,van=board?.candy_van;if(!board||!van||seat===null)return null;
  const token=board.tokens.find(t=>t.seat===seat),space=board.spaces.find(s=>s.id===token?.space);
  const run=board.candy_runs?.find(run=>run.seat===seat);
  const feet=run??(space?{x:space.x,z:space.z}:null),distance=feet?Math.hypot(feet.x-van.x,feet.z-van.z):Infinity;
  const joined=van.joined_seats?.includes(seat)??false,bought=van.bought_seats.includes(seat),coins=token?.coins??0;
  const alive=!!token&&token.health>0&&token.resting===0;
  const offer=van.phase==='offer',canJoin=alive&&room?.phase.name==='board'&&offer&&!joined&&van.can_join;
  const canBuy=alive&&room?.phase.name==='board'&&!offer&&joined&&!bought&&coins>=van.price&&van.can_buy;
  const short=Math.max(0,van.price-coins),near=distance<=van.range;
  // What to do next, in the order it matters: you can't afford it, you're not there yet, or press to buy.
  const status=offer?(joined?'You’re in · get ready':short>0?`Candy costs ${van.price} · you need ${short} more coins`:`Join, run to the van, and buy candy for ${van.price} coins`):!joined?'Visit in progress · board paused':bought?'Candy collected · return when the timer ends':short>0?`Need ${short} more coins to buy`:!near?`Run to the van · ${Math.ceil(distance)}m`:'Buy your candy now!';
  return{van,offer,joined,bought,coins,distance,run,canJoin,canBuy,status,short,near};
}
