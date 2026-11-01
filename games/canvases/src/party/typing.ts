/** Text entry owns keyboard, pointer and controller input until focus leaves.
 * Inspect active focus as well as the event target: gamepad reads and synthetic
 * canvas events do not carry the focused textarea as their target. */
export function isTypingTarget(target:EventTarget|null|undefined):boolean{
  const node=target as Node|null|undefined;
  const element=node?.nodeType===1?node as Element:node?.parentElement;
  if(!element?.closest)return false;
  if(element.closest('input,textarea,select,[role="textbox"],[role="combobox"]'))return true;
  const editable=element.closest('[contenteditable]');
  return !!editable&&editable.getAttribute('contenteditable')?.toLowerCase()!=='false';
}
export function isGameplayTyping(target?:EventTarget|null):boolean{
  if(isTypingTarget(target))return true;
  const owner=(target as Node|null|undefined)?.ownerDocument;
  const doc=owner??(typeof document==='undefined'?null:document);
  if(!doc)return false;
  if(doc.designMode?.toLowerCase()==='on')return true;
  let active=doc.activeElement;
  while(active?.shadowRoot?.activeElement)active=active.shadowRoot.activeElement;
  return isTypingTarget(active);
}
