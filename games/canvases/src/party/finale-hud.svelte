<script lang="ts">
  import { Flag, Trophy } from '@lucide/svelte';
  import type { MapFinaleView, Member } from './protocol';
  import{finaleGuidance}from'./board/finale-guidance';
  let { finale, seat, members, spectating=false, touch=false, oncountdown=()=>{} }: {
    finale:MapFinaleView; seat:number|null; members:Member[]; spectating?:boolean; touch?:boolean; oncountdown?:(count:number)=>void;
  }=$props();
  const mine=$derived(finale.racers.find(racer=>racer.seat===seat)??null);
  const places=$derived([...finale.racers].sort((a,b)=>a.place-b.place));
  const next=$derived(mine?finale.checkpoints[mine.next_checkpoint]:null);
  const guidance=$derived(finaleGuidance(finale,mine));
  const distance=$derived(guidance?.distance??null);
  const total=$derived(Math.max(1,finale.checkpoints.length-1));
  const completed=$derived(mine?Math.max(0,Math.min(total,mine.next_checkpoint-1)):0);
  const light=$derived(Math.max(0,Math.ceil(finale.ready)));
  const seconds=$derived(Math.max(0,Math.ceil(finale.remaining)));
  const timer=$derived(`${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`);
  const accent=$derived(finale.map==='island'?'#bef264':finale.map==='volcano'?'#fb923c':finale.map==='shipwreck'?'#38bdf8':'#67e8f9');
  const gaugeLabel=$derived(finale.map==='island'?'COCONUT BALANCE':finale.map==='volcano'?'HEAT':null);
  const gauge=$derived(Math.max(0,Math.min(1,mine?.gauge??0)));
  const name=(value:number)=>members.find(member=>member.seat===value)?.name??`Player ${value+1}`;
  let lastLight=-1;
  $effect(()=>{if(light!==lastLight){lastLight=light;if(light>0||finale.elapsed<1)oncountdown(light);}});
</script>

<div class="finale-hud" class:touch style="--finale-accent:{accent}">
  <header class="finale-heading"><span>MAP FINALE</span><strong>{finale.title}</strong><p>{finale.rules}</p></header>
  <div class="race-clock" class:urgent={seconds<=10&&light===0}><small>TIME LEFT</small><strong>{timer}</strong></div>

  {#if light>0}
    <div class="start"><span>GET READY</span><div class="lights" aria-label={`Starting in ${light}`}>
      {#each [3,2,1] as threshold}<i class:lit={light<=threshold}></i>{/each}
    </div><strong>{light}</strong></div>
  {:else if finale.elapsed<0.8}
    <div class="go">GO!</div>
  {/if}

  {#if mine&&!spectating}
    <section class="course" aria-label="Race progress">
      <div class="course-top"><span><Flag size={15}/> {mine.finished?'COURSE COMPLETE':`GATE ${Math.min(total,completed+1)} / ${total}`}</span><strong>#{mine.place}</strong></div>
      <div class="course-progress"><i style={`transform:scaleX(${mine.finished?1:completed/total})`}></i></div>
      {#if mine.finished}
        <div class="course-next">Final time <b>{mine.finish_time?.toFixed(2)??'—'}s</b></div>
      {:else if mine.dnf}
        <div class="course-next">Course closed <b>{completed} gates</b></div>
      {:else if mine.stun>0}
        <div class="course-next caution">{finale.map==='frozen'?'TRAFFIC HIT':finale.map==='island'?'COCONUT DROPPED':finale.map==='volcano'?'TOO HOT':'RECOVERING'}<b>{Math.ceil(mine.stun)}s</b></div>
      {:else if mine.waiting}
        <div class="course-next tide">HIGH TIDE · WAIT FOR THE GATE <b>HOLD</b></div>
      {:else if mine.slipping}
        <div class="course-next tide">ICE · STEER EARLY <b>SLIPPERY</b></div>
      {:else}
        <div class="course-next">{guidance?.finish?'Finish line':'Next checkpoint'} <b>{distance===null?'—':`${Math.ceil(distance)}m`}</b></div>
      {/if}
      {#if gaugeLabel&&!mine.finished&&!mine.dnf}
        <div class="gauge-label"><span>{gaugeLabel}</span><b class:danger={gauge>.75}>{gauge>.8?(finale.map==='island'?'STEADY NOW':'REACH A GATE'):`${Math.round(gauge*100)}%`}</b></div>
        <div class="gauge"><i class:danger={gauge>.75} style={`transform:scaleX(${gauge})`}></i></div>
      {/if}
      <p class="road-hint">FOLLOW THE GREEN ROAD ARROWS</p>
      <p class="controls">WASD / LEFT STICK <span>Move</span></p>
    </section>
  {/if}

  <aside class="field" aria-label="Live race positions"><div class="field-label">LIVE POSITIONS</div>
    {#each places.slice(0,3) as racer(racer.seat)}
      <div class="place-row" class:you={racer.seat===seat}><b>{racer.place}</b><span>{name(racer.seat)}{racer.seat===seat?' · YOU':''}</span><small>{racer.finished?'FINISH':racer.dnf?'DNF':`${Math.max(0,racer.next_checkpoint-1)}/${total}`}</small></div>
    {/each}
    {#if mine&&mine.place>3}<div class="place-row you below"><b>{mine.place}</b><span>{name(mine.seat)} · YOU</span><small>{mine.finished?'FINISH':`${completed}/${total}`}</small></div>{/if}
  </aside>

  {#if mine?.finished&&!spectating}
    <div class="finished"><span><Trophy size={18}/> FINISH</span><strong>#{mine.place}</strong><p>{mine.finish_time?.toFixed(2)??'—'}<small>seconds</small></p></div>
  {:else if mine?.dnf&&!spectating}
    <div class="finished dnf"><span>TIME</span><strong>#{mine.place}</strong><p>{completed}<small>gates completed</small></p></div>
  {/if}
</div>

<style>
  .finale-hud{position:absolute;inset:0;pointer-events:none;user-select:none;color:white;font-family:inherit;text-shadow:0 2px 6px #0008}
  .finale-heading{position:absolute;top:4.8rem;left:50%;transform:translateX(-50%);width:min(32rem,calc(100% - 17rem));text-align:center;padding:.45rem .8rem .55rem;background:linear-gradient(100deg,#06101be0,#06101b98);border-top:2px solid var(--finale-accent);clip-path:polygon(9px 0,100% 0,calc(100% - 9px) 100%,0 100%)}
  .finale-heading>span{font-size:.55rem;font-weight:950;letter-spacing:.24em;color:var(--finale-accent)}.finale-heading>strong{display:block;font-size:1.5rem;font-weight:950;font-style:italic;letter-spacing:-.03em;text-transform:uppercase}.finale-heading p{margin:.25rem 0 0;font-size:.65rem;color:#ecf3ff}
  .race-clock{position:absolute;right:1.2rem;top:4.7rem;border-top:2px solid var(--finale-accent);padding:.55rem .8rem;background:linear-gradient(110deg,#07111de8,#07111d60);text-align:right}.race-clock small{display:block;font-size:.5rem;font-weight:850;letter-spacing:.16em;color:#ffffff90}.race-clock strong{font-size:2rem;font-weight:950;font-variant-numeric:tabular-nums;line-height:1.2}.race-clock.urgent{color:#fda4af;border-color:#fb7185}
  .start,.go{position:absolute;top:34%;left:50%;transform:translate(-50%,-50%);text-align:center}.start>span{font-size:.65rem;font-weight:900;letter-spacing:.2em}.lights{display:flex;gap:.65rem;padding:.8rem 1rem;margin:.5rem 0;background:#040910d9;border-bottom:2px solid #ffffff30;transform:skewX(-7deg)}.lights i{width:1.1rem;height:1.1rem;border-radius:50%;background:#42212a;border:1px solid #fb718540}.lights i.lit{background:#fb7185;box-shadow:0 0 18px #fb718590}.start>strong{font-size:4rem;line-height:1;font-weight:950;font-style:italic}.go{font-size:5rem;font-style:italic;font-weight:1000;color:#a3ff52;filter:drop-shadow(0 4px 0 #142e10)}
  .course{position:absolute;left:1.2rem;bottom:1.2rem;width:min(19rem,calc(100% - 2.4rem));padding:.8rem 1rem;background:linear-gradient(105deg,#07111dec,#07111d75);border-left:3px solid var(--finale-accent);clip-path:polygon(0 0,calc(100% - 9px) 0,100% 9px,100% 100%,0 100%)}
  .course-top{display:flex;align-items:center;justify-content:space-between;gap:.8rem}.course-top>span{display:flex;align-items:center;gap:.4rem;font-size:.68rem;font-weight:950;letter-spacing:.08em;color:var(--finale-accent)}.course-top>strong{font-size:1.8rem;font-weight:950;font-style:italic;line-height:1}.course-progress,.gauge{height:3px;margin:.55rem 0;background:#ffffff20;overflow:hidden}.course-progress i,.gauge i{display:block;height:100%;width:100%;transform-origin:left;background:var(--finale-accent);transition:transform .12s linear}.course-next{display:flex;align-items:center;justify-content:space-between;gap:.7rem;font-size:.65rem;color:#ffffffa0}.course-next b{font-size:.9rem;color:white;font-variant-numeric:tabular-nums}.course-next.caution,.course-next.caution b{color:#fda4af}.course-next.tide,.course-next.tide b{color:#7dd3fc}.controls{font-size:.5rem;letter-spacing:.09em;font-weight:850;margin:.7rem 0 0;color:#ffffffa0}.controls span{margin-left:.4rem;font-size:.6rem;letter-spacing:0;font-weight:500;color:#ffffff70}.gauge-label{display:flex;justify-content:space-between;gap:.4rem;margin-top:.75rem;font-size:.5rem;font-weight:850;letter-spacing:.07em;color:#ffffff90}.gauge-label b{font-variant-numeric:tabular-nums;color:var(--finale-accent)}.gauge{height:3px;margin:.3rem 0 0}.gauge i.danger{background:#fb7185}.gauge-label b.danger{color:#fda4af}
  .road-hint{font-size:.6rem;letter-spacing:.07em;font-weight:950;color:#baff52;margin:.6rem 0 0;}
  .field{position:absolute;right:1.2rem;bottom:1.2rem;min-width:11rem;max-width:17rem;background:linear-gradient(90deg,#07111dbd,#07111d45);padding:.6rem .7rem;border-top:1px solid #ffffff30}.field-label{font-size:.48rem;letter-spacing:.15em;font-weight:900;color:#ffffff80;margin-bottom:.4rem}.place-row{display:flex;align-items:center;gap:.55rem;min-height:1.6rem;font-size:.62rem}.place-row>b{width:.85rem;font-style:italic;font-size:.9rem}.place-row>span{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#ffffffb0}.place-row>small{font-size:.48rem;font-weight:850;letter-spacing:.04em;color:#ffffff80}.place-row.you>b,.place-row.you>span{color:var(--finale-accent);font-weight:850}.place-row.below{border-top:1px solid #ffffff20;margin-top:.2rem;padding-top:.2rem}
  .finale-hud.touch .course{bottom:10rem}.finished{position:absolute;left:50%;top:41%;transform:translate(-50%,-50%);padding:1rem 2.5rem;text-align:center;background:linear-gradient(100deg,#07111deb,#07111d55);border-top:3px solid #a3ff52;clip-path:polygon(12px 0,100% 0,calc(100% - 12px) 100%,0 100%)}.finished>span{display:flex;align-items:center;justify-content:center;gap:.6rem;font-weight:950;letter-spacing:.18em;font-size:.7rem;color:#a3ff52}.finished>strong{display:block;font-size:4.7rem;font-style:italic;line-height:1.15;font-weight:1000}.finished>p{margin:0;font-size:1rem;font-weight:850;font-variant-numeric:tabular-nums}.finished small{display:block;margin-top:.2rem;font-size:.55rem;font-weight:500;color:#ffffff90}.finished.dnf{border-color:#94a3b8}.finished.dnf>span{color:#cbd5e1}
  @media(max-width:650px){.finale-heading{left:1rem;transform:none;text-align:left;width:calc(100% - 8.5rem);top:4.5rem}.finale-heading>strong{font-size:1.1rem}.finale-heading p{font-size:.55rem}.race-clock{top:4.4rem;right:.9rem}.race-clock strong{font-size:1.55rem}.course{left:.8rem;bottom:.8rem;width:min(15rem,calc(100% - 1.6rem));padding:.65rem .75rem}.field{right:.8rem;top:10.5rem;bottom:auto;min-width:9rem;max-width:12rem;padding:.4rem .55rem}.field-label{font-size:.42rem}.place-row{font-size:.55rem;min-height:1.4rem}.course-top>strong{font-size:1.55rem}.finished{padding:.8rem 1.7rem}.finished>strong{font-size:3.6rem}}
</style>
