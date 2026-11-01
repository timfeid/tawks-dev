import type {Costume} from '../protocol';

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

/** Free for everyone; `null` is the round head. */
export const HEADS=[{id:null,name:'Round'},{id:'head-pumpkin',name:'Pumpkin'}] as const;

/** Whole characters use their authored appearance, independent of fitted clothes. */
export function characterSelection(costume:Costume,modelId:string):Costume {
  return {color:costume.color,model_id:modelId,bag:costume.bag??null,head:costume.head??null,...(costume.body_width == null ? {} : {body_width:costume.body_width}),...(costume.extras?.length?{extras:[...costume.extras]}:{}),...(costume.reaction?{reaction:costume.reaction}:{}),...(costume.shuffle_reactions?{shuffle_reactions:true}:{})};
}

/** Imported personal models use their library ownership; store models use entitlements. */
export function hasUnownedSelection(costume:Costume,owned:readonly string[],catalogIds:readonly string[]) {
  if(costume.model_id&&catalogIds.includes(costume.model_id)&&!owned.includes(costume.model_id))return true;
  return SINGLE_SLOTS.some(slot=>!!costume[slot]&&!owned.includes(costume[slot]!))||(costume.extras??[]).some(id=>!owned.includes(id))||(!!costume.reaction&&!owned.includes(costume.reaction));
}
