<script lang="ts">
  import type { Hud } from '../lib/game';

  let { hud, units }: { hud: Hud; units: 'mph' | 'kmh' } = $props();

  const speed = $derived(Math.round(Math.abs(hud.speed) * (units === 'mph' ? 2.23694 : 3.6)));
  const reverse = $derived(hud.speed < -0.5);
  const label = $derived(hud.street ?? (hud.offRoad ? 'Off road' : 'Unnamed road'));
</script>

<div class="hud">
  <div class="sign" class:empty={!hud.street} title={label} aria-live="polite">{label}</div>
  <div class="speed panel">
    <span class="value">{speed}</span>
    <span class="unit">{reverse ? 'R ' : ''}{units === 'mph' ? 'mph' : 'km/h'}</span>
  </div>
</div>

<style>
  .hud {
    position: absolute;
    left: 50%;
    bottom: calc(18px + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    pointer-events: none;
    max-width: calc(100% - 32px);
  }
  .sign {
    padding: 8px 16px 7px;
    border-radius: 8px;
    background: var(--sign);
    border: 2px solid rgba(255, 255, 255, 0.85);
    box-shadow:
      0 0 0 2px var(--sign),
      0 8px 24px rgba(0, 0, 0, 0.4);
    font-family: var(--font-sign);
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.03em;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    max-width: min(440px, 60vw);
  }
  .sign.empty {
    background: #39424d;
    box-shadow:
      0 0 0 2px #39424d,
      0 8px 24px rgba(0, 0, 0, 0.4);
    color: #d6dde5;
  }
  .speed {
    display: flex;
    align-items: baseline;
    gap: 5px;
    padding: 6px 14px;
    flex-shrink: 0;
  }
  .value {
    font-family: var(--font-sign);
    font-size: 30px;
    font-weight: 700;
    min-width: 2ch;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .unit {
    color: var(--text-dim);
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  @media (max-width: 640px) {
    .hud {
      bottom: auto;
      top: calc(112px + env(safe-area-inset-top));
      left: 12px;
      transform: none;
      max-width: calc(100% - 150px);
      flex-direction: column;
      align-items: flex-start;
      gap: 8px;
    }
    .sign {
      font-size: 17px;
      padding: 6px 12px 5px;
    }
    .value {
      font-size: 22px;
    }
  }
</style>
