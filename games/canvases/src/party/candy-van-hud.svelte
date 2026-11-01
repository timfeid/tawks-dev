<script lang="ts">
 import{Candy,Check,Coins}from'@lucide/svelte';import type{ClientMsg,RoomView}from'./protocol';import{candyVanState}from'./candy-van-state';import{settings,keyName,watchSettings}from'./settings';import{onMount}from'svelte';import{padButton,watchInputDevice,type InputDevice}from'./input-device';
 let{room,seat,send,bearing=0}:{room:RoomView|null;seat:number|null;send:(m:ClientMsg)=>void;bearing?:number}=$props();
 let useKey=$state(keyName(settings().keys.interact[0]??'KeyE'));let padAction=$state(padButton(settings().pad.action));let device=$state<InputDevice>('keys');
 onMount(()=>{const stopSettings=watchSettings(s=>{useKey=keyName(s.keys.interact[0]??'KeyE');padAction=padButton(s.pad.action);});const stopDevice=watchInputDevice(d=>device=d);return()=>{stopSettings?.();stopDevice();};});
 // On a controller its action button does both; on a keyboard, B joins and the use key buys.
 const joinKey=$derived(device==='pad'?padAction:'B');const buyKey=$derived(device==='pad'?padAction:useKey);
 const visit=$derived(candyVanState(room,seat));
</script>
{#if visit&&room?.phase.name==='board'}
 <aside class="candy-van" aria-label="Candy van">
  <div class="title"><Candy size={16}/><strong>CANDY VAN</strong><b>{Math.ceil(visit.van.remaining)}s</b></div>
  <div class="offer"><strong>{visit.offer?'Join the candy run':'Candy run'}</strong><span><Coins size={13}/>{visit.van.price}</span></div>
  <small>{visit.status}</small>
  {#if visit.offer}
   <button disabled={!visit.canJoin} onclick={()=>send({type:'join_candy_visit'})}>{visit.joined?'Joined!':'Join visit'}<kbd>{joinKey}</kbd></button>
   <div class="note">{visit.van.joined_seats.length} joined · 15 seconds to visit</div>
  {:else if visit.joined&&!visit.bought}
   <button disabled={!visit.canBuy} onclick={()=>send({type:'buy_candy'})}>Buy candy<kbd>{buyKey}</kbd></button>
  {:else if visit.bought}<div class="collected"><Check size={14}/> Added to your score</div>{/if}
  <div class="pause">BOARD PAUSED</div>
 </aside>
 {#if visit.run}
  <div class="candy-compass" aria-label="Direction to candy van">
   {#if !visit.bought}<svg class="road-arrow" viewBox="0 0 120 124" role="img" aria-label="Follow this arrow" style={`transform:rotate(${bearing}rad)`}>
    <path d="M60 9 111 59 85 59 85 109 35 109 35 59 9 59Z" fill="#163913" stroke="#102515" stroke-width="9" stroke-linejoin="round" transform="translate(0 8)"/>
    <path d="M60 9 111 59 85 59 85 109 35 109 35 59 9 59Z" fill="#b5f335" stroke="#f4ffb1" stroke-width="4" stroke-linejoin="round"/>
    <path d="M60 16 100 55H79V102H60Z" fill="#6acb28"/>
   </svg>{:else}<Check size={56}/>{/if}
   <strong>{visit.bought?'CANDY COLLECTED':`${Math.ceil(visit.distance)}m TO CANDY VAN`}</strong>
   <span class:ready={visit.canBuy}>{visit.canBuy?`${buyKey} · BUY CANDY`:visit.bought?'NICE HAUL!':visit.short>0?`NEED ${visit.short} MORE COINS`:visit.near?'BUY CANDY':'FOLLOW THE ARROW'} · {Math.ceil(visit.van.remaining)}s</span>
  </div>
 {/if}
{/if}
<style>
.candy-van{position:absolute;z-index:42;right:1rem;top:50%;transform:translateY(-50%);width:14rem;padding:.8rem .9rem;color:#fff1f2;background:linear-gradient(135deg,#311c2af5,#16121bf0);border-top:3px solid #f9a8d4;box-shadow:0 6px 24px #0006;font-family:inherit}.title,.offer{display:flex;align-items:center;gap:.4rem}.title{color:#fbcfe8;font-size:.62rem;letter-spacing:.09em}.title b{margin-left:auto;font-size:.9rem;font-variant-numeric:tabular-nums}.offer{justify-content:space-between;margin:.6rem 0 .35rem;font-size:1.05rem}.offer>span{display:flex;align-items:center;gap:.25rem;color:#fde68a;font-size:.78rem}.candy-van small{display:block;color:#fce7f3bf;font-size:.67rem;line-height:1.4}.candy-van button{display:flex;align-items:center;justify-content:space-between;width:100%;margin-top:.65rem;border:0;border-radius:.35rem;background:#f9a8d4;color:#311426;padding:.48rem .65rem;font:inherit;font-size:.7rem;font-weight:850;cursor:pointer}.candy-van button:disabled{opacity:.45;cursor:default}.candy-van kbd{font:inherit;font-size:.6rem;border:1px solid currentColor;border-radius:.2rem;padding:0 .2rem}.collected{display:flex;gap:.3rem;align-items:center;margin-top:.55rem;font-size:.65rem;color:#bbf7d0}.note{margin-top:.45rem;color:#fce7f399;font-size:.6rem}.pause{margin-top:.5rem;color:#fde68a;font-size:.5rem;letter-spacing:.15em;font-weight:900}.candy-compass{position:absolute;z-index:43;top:.65rem;left:50%;transform:translateX(-50%);display:flex;align-items:center;flex-direction:column;gap:.2rem;color:#eaffb8;pointer-events:none;filter:drop-shadow(0 3px 2px #0009)}.road-arrow{width:106px;height:110px;transition:transform 60ms linear}.candy-compass strong{font-size:.76rem;font-weight:950;letter-spacing:.07em;text-shadow:0 2px 3px #000}.candy-compass span.ready{background:#f9a8d4;color:#311426;animation:ready-pulse .7s ease-in-out infinite alternate}@keyframes ready-pulse{to{transform:scale(1.08)}}.candy-compass span{font-size:.73rem;font-weight:850;color:white;background:#172414db;padding:.25rem .6rem;border-radius:.2rem}@media(max-width:1000px){.candy-van{right:.65rem;width:11.5rem;padding:.65rem .7rem}.offer{font-size:.9rem;margin:.4rem 0 .25rem}.candy-van button{margin-top:.45rem}}@media(max-width:640px){.candy-compass{top:.4rem;gap:.1rem}.candy-compass .road-arrow{width:76px;height:80px}.candy-compass strong{font-size:.52rem}.candy-compass span.ready{background:#f9a8d4;color:#311426;animation:ready-pulse .7s ease-in-out infinite alternate}@keyframes ready-pulse{to{transform:scale(1.08)}}.candy-compass span{font-size:.68rem}}
</style>
