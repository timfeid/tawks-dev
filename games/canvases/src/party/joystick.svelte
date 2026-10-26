<script lang="ts">
  import type { StickValue } from './input';

  let { onchange }: { onchange: (value: StickValue) => void } = $props();

  const RADIUS = 56;
  let base: HTMLDivElement;
  let pointer: number | null = null;
  let knob = $state({ x: 0, y: 0 });

  function move(event: PointerEvent) {
    if (pointer !== event.pointerId) return;
    const rect = base.getBoundingClientRect();
    let x = event.clientX - (rect.left + rect.width / 2);
    let y = event.clientY - (rect.top + rect.height / 2);
    const length = Math.hypot(x, y);
    if (length > RADIUS) {
      x = (x / length) * RADIUS;
      y = (y / length) * RADIUS;
    }
    knob = { x, y };
    onchange({ x: x / RADIUS, z: y / RADIUS });
  }

  function release(event: PointerEvent) {
    if (pointer !== event.pointerId) return;
    pointer = null;
    knob = { x: 0, y: 0 };
    onchange({ x: 0, z: 0 });
  }
</script>

<div
  bind:this={base}
  class="relative size-36 touch-none select-none rounded-full border-4 border-white/50 bg-slate-900/35 backdrop-blur-sm"
  role="slider"
  aria-label="Roll direction"
  aria-valuenow={0}
  tabindex="-1"
  onpointerdown={(event) => {
    pointer = event.pointerId;
    base.setPointerCapture(event.pointerId);
    move(event);
  }}
  onpointermove={move}
  onpointerup={release}
  onpointercancel={release}
>
  <div
    class="absolute left-1/2 top-1/2 size-14 rounded-full border-4 border-white bg-white/80 shadow-lg"
    style="transform: translate(calc(-50% + {knob.x}px), calc(-50% + {knob.y}px));"
  ></div>
</div>
