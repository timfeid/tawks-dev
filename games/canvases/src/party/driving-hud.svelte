<script lang="ts">
  import {Car,Volume2} from '@lucide/svelte';
  import type {DriverView} from './protocol';
  let {driver,onhorn}:{driver:DriverView;onhorn:(held:boolean)=>void}=$props();
</script>
<section class="driving-hud" aria-label="Driving">
  <div class="driving-hud__top"><Car size={19}/><strong>{driver.arriving>0?'YOUR RIDE IS HERE':'TAKE THE WHEEL'}</strong><b>{Math.ceil(driver.remaining)}<small>s</small></b></div>
  <div class="driving-hud__speed"><strong>{Math.round(Math.abs(driver.speed)*8).toString().padStart(2,'0')}</strong><span>SPEED</span><button onpointerdown={event=>{event.currentTarget.setPointerCapture(event.pointerId);onhorn(true)}} onpointerup={()=>onhorn(false)} onpointercancel={()=>onhorn(false)}><Volume2 size={17}/> Horn <kbd>Space</kbd></button></div>
  <p>W / S accelerate &amp; brake · A / D steer</p>
  <div class="driving-hud__bar"><i style={`transform:scaleX(${Math.max(0,Math.min(1,driver.remaining/10))})`}></i></div>
</section>
<style>
.driving-hud{position:absolute;z-index:45;left:50%;bottom:1rem;transform:translateX(-50%);width:min(24rem,calc(100% - 2rem));padding:1rem;color:white;background:linear-gradient(110deg,#061526ed,#08121bda);border-top:3px solid #67e8f9;box-shadow:0 6px 24px #0007;font-family:inherit}.driving-hud__top{display:flex;align-items:center;gap:.6rem;color:#a5f3fc;font-size:.7rem;letter-spacing:.1em}.driving-hud__top b{margin-left:auto;font-size:1.4rem;letter-spacing:0}.driving-hud__top small{font-size:.65rem;margin-left:.15rem}.driving-hud__speed{display:flex;align-items:baseline;gap:.4rem;margin:.3rem 0}.driving-hud__speed>strong{font-size:2.7rem;font-weight:950;font-variant-numeric:tabular-nums;line-height:1}.driving-hud__speed>span{color:#ffffff70;font-size:.55rem;letter-spacing:.1em}.driving-hud__speed button{margin-left:auto;display:flex;align-items:center;gap:.4rem;background:#ffffff10;border:1px solid #ffffff25;border-radius:.4rem;padding:.55rem .65rem;color:#e0f2fe;font-size:.7rem;cursor:pointer}.driving-hud__speed button:active{background:#67e8f950}.driving-hud p{margin:.6rem 0;color:#ffffff99;font-size:.65rem}kbd{font:inherit;color:#ffffff60}.driving-hud__bar{height:3px;background:#ffffff20}.driving-hud__bar i{display:block;height:100%;background:#67e8f9;transform-origin:left;transition:transform .1s linear}
</style>
