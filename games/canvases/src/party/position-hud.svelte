<script lang="ts">
  import { Crosshair } from '@lucide/svelte';
  import ItemIcon from './item-icon.svelte';
  import { ITEMS } from './board/items';
  import type { ClientMsg, PositionView } from './protocol';

  let { position, send }: { position: PositionView; send: (message: ClientMsg) => void } = $props();
  let submitted = $state(false);
  const item = $derived(ITEMS[position.item]);
  const seconds = $derived(Math.max(0, Math.ceil(position.remaining)));
  const progress = $derived(Math.max(0, Math.min(1, position.remaining / 10)));
  const activate = () => {
    if (submitted || position.active) return;
    submitted = true;
    send({ type: 'activate_item' });
  };
  const keydown = (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.repeat || event.ctrlKey || event.metaKey || event.altKey || event.code !== 'KeyQ') return;
    const target = event.target;
    if (target instanceof HTMLElement && (target.isContentEditable || target.closest('input,textarea,select,[contenteditable="true"],[contenteditable=""]'))) return;
    event.preventDefault();
    activate();
  };
</script>

<svelte:window onkeydown={keydown}/>

<section class="positioning" aria-label="Get in position" style="--item-color:{item.color}">
  <div class="positioning__heading">
    <span class="positioning__label"><Crosshair size={14}/> GET IN POSITION</span>
    <span class="positioning__clock" aria-label={`${seconds} seconds remaining`}>{seconds}<small>s</small></span>
  </div>
  <div class="positioning__main">
    <div class="positioning__help"><strong>Find your angle.</strong><span>Move with WASD. Your {item.label.toLowerCase()} is ready.</span></div>
    <button class="positioning__use" onclick={activate} disabled={submitted || position.active}>
      <kbd>Q</kbd><ItemIcon item={position.item}/><span>{submitted ? 'Ready…' : `Use ${item.label}`}</span>
    </button>
  </div>
  <div class="positioning__timer" role="progressbar" aria-label="Time to get in position" aria-valuemin={0} aria-valuemax={10} aria-valuenow={Math.max(0,position.remaining)}><i style={`transform:scaleX(${progress})`}></i></div>
</section>

<style>
  .positioning{color:white;min-width:0;font-family:inherit}
  .positioning__heading{display:flex;align-items:center;justify-content:space-between;gap:1rem}
  .positioning__label{display:flex;align-items:center;gap:.5rem;font-size:.61rem;font-weight:900;letter-spacing:.15em;color:var(--item-color)}
  .positioning__clock{font-size:1.55rem;line-height:1;font-weight:950;font-variant-numeric:tabular-nums}
  .positioning__clock small{margin-left:.15rem;font-size:.65rem;color:#ffffff70}
  .positioning__main{display:flex;align-items:center;justify-content:space-between;gap:1rem;margin:.7rem 0 .95rem}
  .positioning__help{display:flex;flex-direction:column;gap:.25rem}
  .positioning__help strong{font-size:.9rem;font-weight:850}.positioning__help>span{font-size:.7rem;color:#ffffff90}
  .positioning__use{display:flex;align-items:center;justify-content:center;gap:.55rem;flex-shrink:0;border:1px solid color-mix(in srgb,var(--item-color) 65%,transparent);background:color-mix(in srgb,var(--item-color) 15%,#07101c);padding:.65rem .85rem;color:white;font-size:.75rem;font-weight:850;cursor:pointer}
  .positioning__use:hover{background:color-mix(in srgb,var(--item-color) 26%,#07101c)}.positioning__use:disabled{opacity:.55;cursor:default}
  kbd{display:grid;place-items:center;min-width:1.35rem;height:1.35rem;padding:0 .2rem;background:#ffffff15;border:1px solid #ffffff40;border-bottom-width:2px;font-family:inherit;font-size:.65rem}
  .positioning__timer{height:3px;background:#ffffff12;overflow:hidden}.positioning__timer i{display:block;width:100%;height:100%;background:var(--item-color);transform-origin:left;transition:transform .1s linear}
  @media(max-width:500px){.positioning__main{gap:.6rem}.positioning__help>span{max-width:10rem;font-size:.65rem}.positioning__use{gap:.35rem;padding:.6rem;font-size:.7rem}}
</style>
