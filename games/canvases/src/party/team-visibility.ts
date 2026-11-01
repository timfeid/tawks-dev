import type{RoomView}from'./protocol';
// Mirrors the public board palette; the contract test catches drift.
const PAIR_COLORS=['#f97316','#22d3ee','#a3e635','#e879f9'];
/** Small structural subset avoids coupling the HUD helper to a bout renderer. */
type TeamHud={teams?:readonly string[];scores?:{seat:number;team?:number}[]}|null|undefined;
export type VisibleTeam={id:number;name:string;label:string;color:string;seats:number[];kind:'pair'|'bout'};
const PAIR_NAMES=['Orange','Cyan','Lime','Violet'];
const SIDE_COLORS=['#f87171','#60a5fa','#a3e635','#e879f9'];
/** Board partnerships and the current minigame's sides are independent. In a
 * bout, evaluated scores override the room assignment (roles may change). */
export function visibleTeams(room:RoomView|null|undefined,hud:TeamHud,scope:'board'|'bout'):VisibleTeam[]{
  if(!room)return[];
  if(scope==='board'){
    if(!room.pairs_on)return[];
    const pairs=room.pairs??[],ids=[...new Set(pairs.map(pair=>pair.team))].sort((a,b)=>a-b);
    return ids.map(id=>({id,name:PAIR_NAMES[id%PAIR_NAMES.length],label:`${PAIR_NAMES[id%PAIR_NAMES.length]} pair${id>=PAIR_NAMES.length?` ${id+1}`:''}`,color:PAIR_COLORS[id%PAIR_COLORS.length],seats:pairs.filter(pair=>pair.team===id).map(pair=>pair.seat),kind:'pair'}));
  }
  const scored=(hud?.scores??[]).filter((score):score is {seat:number;team:number}=>Number.isInteger(score.team)&&typeof score.team==='number'&&score.team>=0);
  const assignments=scored.length?scored:room.teams??[];
  const ids=[...new Set(assignments.map(player=>player.team))].filter(Number.isInteger).sort((a,b)=>a-b);
  // A free-for-all must not acquire board pair colors just because pairs are on.
  if(!ids.length)return[];
  return ids.map(id=>{
    const raw=hud?.teams?.[id]??(['Red','Blue'][id]??`Team ${id+1}`);
    const name=raw.toUpperCase()===raw?raw.slice(0,1)+raw.slice(1).toLowerCase():raw;
    return{id,name,label:/team/i.test(name)?name:`${name} team`,color:SIDE_COLORS[id%SIDE_COLORS.length],seats:assignments.filter(player=>player.team===id).map(player=>player.seat),kind:'bout'};
  });
}
export const visibleTeamFor=(teams:VisibleTeam[],seat:number|null|undefined)=>seat==null?null:teams.find(team=>team.seats.includes(seat))??null;
