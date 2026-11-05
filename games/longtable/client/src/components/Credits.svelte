<script lang="ts">
  import { X } from '@lucide/svelte';

  let { attribution, compatibility, onClose }: { attribution: string; compatibility: string; onClose: () => void } = $props();
</script>

<div class="scrim" role="presentation" onclick={onClose}>
  <div class="card" role="dialog" aria-modal="true" aria-labelledby="credits-title" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && onClose()}>
    <button class="close btn quiet icon" aria-label="Close" onclick={onClose}><X size={18} /></button>
    <h2 id="credits-title">Credits</h2>
    <p class="lead">Longtable is a table for {compatibility} games: dice, initiative and character sheets, shared live in your lobby.</p>
    <h3>Rules content</h3>
    <p class="attribution">{attribution}</p>
    <h3>Made for tawks</h3>
    <p>Dice, sheets and the table itself are drawn for Longtable. The code is open source in the tawks-dev repository, under games/longtable.</p>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(8, 5, 2, 0.7);
  }

  .card {
    position: relative;
    max-width: 560px;
    max-height: 100%;
    overflow-y: auto;
    padding: 22px;
    border-radius: var(--radius);
    color: var(--ink);
    background: linear-gradient(180deg, var(--parchment), var(--parchment-deep));
    border: 1px solid var(--parchment-edge);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  }

  .close {
    position: absolute;
    top: 10px;
    right: 10px;
    color: var(--ink-soft);
  }

  h2 {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: 26px;
  }

  h3 {
    margin: 16px 0 4px;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  p {
    margin: 0;
  }

  .lead {
    font-size: 15px;
  }

  .attribution {
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
</style>
