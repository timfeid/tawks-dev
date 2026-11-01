<script lang="ts">
  import {onDestroy} from 'svelte';
  import {Crosshair,Swords} from '@lucide/svelte';
  import type {ClientMsg,RoomView} from './protocol';
  let {room,seat,send}:{room:RoomView|null;seat:number|null;send:(message:ClientMsg)=>void}=$props();
  const board=$derived(room?.board);
  const raid=$derived(board?.raids?.find(raid=>raid.seat===seat));
  const gun=$derived(raid?.item==='drive_by');
  const token=$derived(board?.tokens.find(token=>token.seat===seat));
  const ownReady=$derived(board?.turn_seat===seat && ['items','done'].includes(board?.turn.step??''));
  const available=$derived(room?.phase.name==='board' && seat!==null && !raid && !board?.driving?.some(car=>car.seat===seat) && board?.positioning?.seat!==seat && token?.resting===0 && token.health>0);
  const slashReady=$derived(available && (board?.turn_seat!==seat||ownReady) && token?.items.includes('slash_rush'));
  const gunReady=$derived(available && ownReady && token?.items.includes('drive_by'));
  const chain=$derived(Math.max(0,...(raid?.hits??[]).map(([,hits])=>hits)));
  let feedback=$state(''),feedbackKind=$state(''),held=$state(false);
  let cursor=0,identity='',timer:ReturnType<typeof setTimeout>|undefined;
  function stopFire(){if(held){held=false;if(raid?.item==='drive_by')send({type:'raid_fire',on:false,yaw:raid.yaw});}}
  function fire(event:PointerEvent){if(!gun||!raid||raid.ammo<=0)return;event.preventDefault();(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);held=true;send({type:'raid_fire',on:true,yaw:raid.yaw});}
  $effect(()=>{
    const key=`${room?.match_id}:${seat}`,log=board?.log??[];
    const last=Number(log.at(-1)?.seq??0);
    if(key!==identity||last<cursor){identity=key;cursor=last;feedback='';return;}
    for(const entry of log){
      if(Number(entry.seq)<=cursor||entry.seat!==seat)continue;
      let text='',kind='';
      if(entry.kind==='raid_shot'){
        text=entry.blocked?'SHIELD':Number(entry.damage)>0?'−1 HEART':entry.target!==null&&entry.target!==undefined?`HIT ${entry.hit_count} / 3`:'MISS';
        kind=entry.blocked?'shield':Number(entry.damage)>0?'damage':entry.target!==null&&entry.target!==undefined?'hit':'miss';
      }else if(entry.kind==='raid_hit'){
        text=entry.quality==='blocked'?'SHIELD':entry.quality==='hit'?'SLASH HIT':'MISS';kind=entry.quality==='hit'?'damage':entry.quality;
      }else if(entry.kind==='raid_slash' && entry.quality==='miss'){text='MISS';kind='miss';}
      if(text){feedback=text;feedbackKind=kind;if(timer)clearTimeout(timer);timer=setTimeout(()=>{feedback='';},1250);}
    }
    cursor=last;
  });
  $effect(()=>{if(!raid||!gun)stopFire();});
  onDestroy(()=>{if(timer)clearTimeout(timer);stopFire();});
</script>
{#if raid}
  <section class="raid-hud" class:gun aria-label={gun?'Drive-by':'Slash rush'}>
    <div class="raid-hud__title">{#if gun}<Crosshair size={18}/>{:else}<Swords size={18}/>{/if}<strong>{gun?'DRIVE-BY':'SLASH RUSH'}</strong><b>{Math.ceil(raid.remaining)}<small>s</small></b></div>
    <div class="raid-clock"><i style={`width:${Math.min(100,raid.remaining*10)}%`}></i></div>
    {#if gun}
      <div class="ammo" aria-label={`${raid.ammo} rounds remaining`}>{#each Array(10) as _,i}<i class:loaded={i<raid.ammo}></i>{/each}<b>{raid.ammo}<small> / 10</small></b></div>
      <div class="hit-chain"><span>{#each Array(3) as _,i}<i class:filled={i<chain}></i>{/each}</span> 3 hits = 1 heart</div>
      <p>WASD move · Mouse aim · Hold click / Space to fire</p>
      <button class:held onpointerdown={fire} onpointerup={stopFire} onpointercancel={stopFire} onlostpointercapture={stopFire} disabled={raid.ammo<=0}>{held?'Firing':raid.shot_cooldown>0?'Cycling…':'Hold to fire'}</button>
    {:else}
      <p>WASD move · Mouse aim · Click / Space slash</p>
      <button onclick={()=>send({type:'raid_slash'})} disabled={raid.cooldown>0}>{raid.cooldown>0?'Recovering…':'Slash'}</button>
    {/if}
    <small class="return-note">You return to your spot when this ends.</small>
  </section>
{:else if slashReady||gunReady}
  <div class="raid-ready">
    {#if slashReady}<button title="Run and slash for up to 10 seconds, then return to your spot." onclick={()=>send({type:'use_item',item:'slash_rush',target:null})}><Swords size={17}/> Slash rush</button>{/if}
    {#if gunReady}<button title="Move on foot with 10 rounds. Three hits take one heart." onclick={()=>send({type:'use_item',item:'drive_by',target:null})}><Crosshair size={17}/> Drive-by <span>10 rounds</span></button>{/if}
  </div>
{/if}
{#if feedback}<div class="raid-feedback" class:damage={feedbackKind==='damage'} class:shield={feedbackKind==='shield'} class:miss={feedbackKind==='miss'} role="status">{feedback}</div>{/if}
<style>
.raid-hud,.raid-ready{position:absolute;z-index:45;bottom:1rem;left:50%;transform:translateX(-50%);color:white;font-family:inherit}.raid-hud{width:min(23rem,calc(100% - 2rem));padding:.9rem 1rem;border-radius:.8rem;background:linear-gradient(140deg,#17202af5,#0a0e16f2);border:1px solid #fb718577;box-shadow:0 6px 24px #0008}.raid-hud.gun{border-color:#fbbf2477}.raid-hud__title{display:flex;align-items:center;gap:.6rem;color:#fda4af;font-size:.8rem;letter-spacing:.08em}.gun .raid-hud__title{color:#fcd34d}.raid-hud__title b{margin-left:auto;font-size:1.25rem}.raid-hud__title b small{font-size:.7rem;margin-left:.15rem}.raid-clock{height:3px;background:#ffffff16;margin:.55rem 0 .7rem;overflow:hidden}.raid-clock i{display:block;height:100%;background:currentColor;color:#fda4af;transition:width .1s linear}.gun .raid-clock i{color:#fcd34d}.raid-hud p{font-size:.67rem;color:#e2e8f0;margin:.65rem 0}.raid-hud button,.raid-ready button{border:0;color:white;border-radius:.4rem;padding:.5rem 1rem;font-weight:800;cursor:pointer;font-family:inherit}.raid-hud button{background:#e11d48;touch-action:none;min-width:7rem}.gun button{background:#fbbf24;color:#1c1917}.gun button.held{background:#fef3c7}.raid-hud button:disabled{opacity:.5}.return-note{display:block;font-size:.6rem;color:#94a3b8;margin-top:.6rem}.ammo{display:flex;align-items:center;gap:.3rem}.ammo i{width:.42rem;height:1rem;display:block;background:#ffffff18;border-radius:2px}.ammo i.loaded{background:#fcd34d;box-shadow:0 0 6px #fbbf2433}.ammo b{margin-left:auto;font-size:1.15rem;line-height:1}.ammo small{font-size:.65rem;color:#94a3b8}.hit-chain{display:flex;align-items:center;gap:.5rem;margin:.55rem 0;font-size:.65rem;color:#cbd5e1}.hit-chain span{display:flex;gap:.2rem}.hit-chain i{width:.45rem;height:.45rem;border:1px solid #fda4af88;transform:rotate(45deg)}.hit-chain i.filled{background:#fb7185}.raid-ready{display:flex;gap:.5rem}.raid-ready button{display:flex;align-items:center;gap:.5rem;background:#111827ee;border:1px solid #fb718577;font-size:.75rem}.raid-ready span{font-size:.6rem;color:#fcd34d}.raid-feedback{position:absolute;z-index:46;left:50%;top:38%;transform:translateX(-50%);color:#fcd34d;text-shadow:0 2px 5px #000,0 0 14px #000;font-size:clamp(1.15rem,3vw,1.8rem);font-weight:950;letter-spacing:.1em;pointer-events:none}.raid-feedback.damage{color:#fb7185}.raid-feedback.shield{color:#93c5fd}.raid-feedback.miss{color:#cbd5e1}
</style>
