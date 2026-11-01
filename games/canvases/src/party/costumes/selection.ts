import type {Costume} from '../protocol';
import seasonalHeads from './heads.json' with {type:'json'};

export const CLOTHING_SLOTS=['hat','shirt','pants','shoes','gloves','extra','bag'] as const;
/** The slots that hold one item; extras stack. */
export const SINGLE_SLOTS=['hat','shirt','pants','shoes','gloves','bag'] as const;

/** Extras worn in place of the hat, one at a time. */
export const HOLIDAY_HATS:readonly string[]=['extra-santa-hat','extra-elf-hat'];

/** Puts an extra on or takes it off; a holiday hat replaces the other one. */
export function toggleExtra(worn:readonly string[],id:string):string[] {
  if(worn.includes(id))return worn.filter(extra=>extra!==id);
  return [...worn.filter(extra=>!(HOLIDAY_HATS.includes(id)&&HOLIDAY_HATS.includes(extra))),id];
}

export type Head={id:string|null;name:string;from?:string;until?:string};
/** Free for everyone; `null` is the round head. Seasonal heads can be put on from `from` to `until` (MM-DD, both whole days, US Eastern) each year. */
export const HEADS:readonly Head[]=[{id:null,name:'Round'},...seasonalHeads];

/** Month and day in US Eastern time, where the head seasons turn over. The server's wardrobe.rs agrees. */
export function easternMonthDay(now:Date):string {
  const parts=new Intl.DateTimeFormat('en-US',{timeZone:'America/New_York',month:'2-digit',day:'2-digit'}).formatToParts(now);
  const part=(type:string)=>parts.find(each=>each.type===type)!.value;
  return `${part('month')}-${part('day')}`;
}

export function inSeason(head:Head,now=new Date()) {
  if(!head.from||!head.until)return true;
  const day=easternMonthDay(now);
  return head.from<=head.until?head.from<=day&&day<=head.until:day>=head.from||day<=head.until;
}

/** The heads that can be picked now, with `worn` kept on offer after its season so it is never taken off for you. */
export function headChoices(worn:string|null|undefined,now=new Date()):Head[] {
  return HEADS.filter(head=>head.id===(worn??null)||inSeason(head,now));
}

const MONTHS=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
/** "Until Nov 15" for a seasonal head, nothing for the round head. */
export function seasonNote(head:Head) {
  if(!head.until)return '';
  const [month,day]=head.until.split('-').map(Number);
  return `Until ${MONTHS[month-1]} ${day}`;
}

/** Whole characters use their authored appearance, independent of fitted clothes. */
export function characterSelection(costume:Costume,modelId:string):Costume {
  return {color:costume.color,model_id:modelId,bag:costume.bag??null,head:costume.head??null,...(costume.body_width == null ? {} : {body_width:costume.body_width}),...(costume.extras?.length?{extras:[...costume.extras]}:{}),...(costume.reaction?{reaction:costume.reaction}:{}),...(costume.shuffle_reactions?{shuffle_reactions:true}:{})};
}

/** Imported personal models use their library ownership; store models use entitlements. */
export function hasUnownedSelection(costume:Costume,owned:readonly string[],catalogIds:readonly string[]) {
  if(costume.model_id&&catalogIds.includes(costume.model_id)&&!owned.includes(costume.model_id))return true;
  return SINGLE_SLOTS.some(slot=>!!costume[slot]&&!owned.includes(costume[slot]!))||(costume.extras??[]).some(id=>!owned.includes(id))||(!!costume.reaction&&!owned.includes(costume.reaction));
}
