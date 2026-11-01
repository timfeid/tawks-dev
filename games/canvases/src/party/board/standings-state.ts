import type {BoardView,Member} from '../protocol';
/** Seats keep their screen corner when the minigame changes turn order. */
export function boardStandings(board:BoardView,members:Member[]){
  const known=board.tokens.every(t=>t.coins!==null);
  const ranking=board.standings??(known?[...board.tokens].sort((a,b)=>b.prizes-a.prizes||(b.coins??0)-(a.coins??0)).reduce<number[][]>((groups,t,_,tokens)=>{
    const previous=groups.at(-1)?.[0],last=tokens.find(t=>t.seat===previous);
    if(last&&last.prizes===t.prizes&&last.coins===t.coins)groups[groups.length-1].push(t.seat);else groups.push([t.seat]);
    return groups;
  },[]):[]);
  const places=new Map<number,number>();let place=1;
  for(const group of ranking){for(const seat of group)places.set(seat,place);place+=group.length;}
  return members.filter(m=>board.tokens.some(t=>t.seat===m.seat)).sort((a,b)=>a.seat-b.seat).map((member,index)=>({
    member,token:board.tokens.find(t=>t.seat===member.seat)!,place:places.get(member.seat)??null,corner:index%4,
    current:board.turn_seat===member.seat,turn:board.order.indexOf(member.seat)+1,
  }));
}
export function placeLabel(place:number|null){if(place===null)return'—';return`${place}${place===1?'st':place===2?'nd':place===3?'rd':'th'}`;}
