<script lang="ts">
  import {Skull}from'@lucide/svelte';
  import type{CrawlerView,Member}from'./protocol';
  import{isGameplayTyping}from'./typing';
  let{crawler,seat,members,onexit}:{crawler:CrawlerView;seat:number|null;members:Member[];onexit?:()=>void}=$props();
  const own=$derived(seat!==null&&crawler.controller===seat);
  const controller=$derived(members.find(member=>member.seat===crawler.controller));
  function key(event:KeyboardEvent){if(own&&onexit&&event.code==='KeyQ'&&!event.repeat&&!isGameplayTyping(event.target)){event.preventDefault();onexit();}}
</script>
<svelte:window onkeydown={key}/>
{#if own}
  <section class="zombie-control" aria-label="Zombie controls">
    <div><Skull size={19}/><strong>YOU ARE THE ZOMBIE</strong><span>LAST PLACE</span></div>
    <p><kbd>W A S D</kbd> Move · Catch the other players</p>
    {#if onexit}<button class="zombie-exit" onclick={onexit}>Return to player <kbd>Q</kbd></button>{:else}<small>Yours to control until your next turn.</small>{/if}
    {#if crawler.pulled_by!==null}<b class="zombie-pull">{crawler.blowing?'Blown back!':'Caught in a vacuum!'}</b>{/if}
  </section>
{:else if controller}
  <div class="zombie-watcher"><Skull size={14}/><span><b>{controller.name}</b> controls the zombie</span></div>
{/if}
<style>
.zombie-exit{pointer-events:auto;display:flex;align-items:center;gap:.5rem;margin-top:.7rem;border:1px solid #bef26477;border-radius:.25rem;padding:.4rem .6rem;background:#a3e635;color:#17250b;font:inherit;font-size:.7rem;font-weight:850;cursor:pointer}
.zombie-control{position:absolute;z-index:44;bottom:1rem;left:50%;transform:translateX(-50%);width:min(24rem,calc(100% - 2rem));padding:.85rem 1rem;color:#f7fee7;background:linear-gradient(120deg,#18250bf5,#101a12e8);border-top:3px solid #a3e635;box-shadow:0 6px 24px #0007;font-family:inherit}.zombie-control>div{display:flex;align-items:center;gap:.5rem;color:#bef264;font-size:.72rem;letter-spacing:.06em}.zombie-control>div span{margin-left:auto;font-size:.5rem;color:#d9f99d99;white-space:nowrap}.zombie-control p{font-size:.7rem;margin:.65rem 0 .35rem;color:#ecfccb}.zombie-control kbd{font:inherit;font-weight:800;letter-spacing:.13em;padding:.2rem .3rem;background:#ffffff12;border-bottom:2px solid #ffffff25;border-radius:.2rem}.zombie-control small{font-size:.62rem;color:#d9f99d99}.zombie-pull{display:block;color:#fef08a;font-size:.65rem;margin-top:.4rem}.zombie-watcher{position:absolute;z-index:30;left:50%;top:3.5rem;transform:translateX(-50%);display:flex;align-items:center;gap:.4rem;max-width:calc(100% - 2rem);padding:.35rem .6rem;color:#d9f99d;background:#14220cbd;border-radius:1rem;font-size:.6rem;white-space:nowrap;pointer-events:none;text-shadow:0 1px 2px #000}.zombie-watcher b{font-weight:750}
</style>
