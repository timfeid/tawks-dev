<script lang="ts">
  import { Footprints, Hand, Shield, TriangleAlert, Users } from "@lucide/svelte";
  import Handcuffs from "./cuffs.svelte";
  import type { CityView, Member } from "../protocol";
  import {
    figureEight,
    inBand,
    LOOP,
    myArrest,
    strugglePhase,
    type TraceCursor,
  } from "./arrest";
  let {
    city,
    seat,
    members,
    cursor,
    onstruggle,
    pad = false,
  }: {
    city: CityView;
    seat: number | null;
    members: Member[];
    cursor: TraceCursor;
    onstruggle: () => void;
    pad?: boolean;
  } = $props();
  const name = (s: number) => members.find((m) => m.seat === s)?.name ?? "Player";
  const mine = $derived(myArrest(city, seat));
  const others = $derived(
    city.arrests.filter((a) => a.officer !== seat && a.robber !== seat),
  );
  const tracing = $derived(mine?.officer && mine.arrest.stage === "cuffing");
  const curve = (() => {
    const points: string[] = [];
    for (let i = 0; i <= 128; i++) {
      const [x, y] = figureEight((i / 128) * LOOP);
      points.push(`${x.toFixed(3)},${(-y).toFixed(3)}`);
    }
    return points.join(" ");
  })();
  const done = $derived.by(() => {
    if (!mine) return "";
    const points: string[] = [];
    const end = mine.arrest.trace;
    for (let t = 0; t <= end; t += 0.05) {
      const [x, y] = figureEight(t);
      points.push(`${x.toFixed(3)},${(-y).toFixed(3)}`);
    }
    return points.join(" ");
  });
  const guide = $derived(mine ? figureEight(mine.arrest.trace) : [0, 0]);
  const arrow = $derived.by(() => {
    if (!mine) return 0;
    const [x0, y0] = figureEight(mine.arrest.trace);
    const [x1, y1] = figureEight(mine.arrest.trace + 0.2);
    return (Math.atan2(-(y1 - y0), x1 - x0) * 180) / Math.PI;
  });
  let receivedAt = $state(performance.now());
  let now = $state(performance.now());
  let cursorView = $state<[number, number]>([0, 0]);
  $effect(() => {
    void city.arrests;
    receivedAt = performance.now();
  });
  $effect(() => {
    let frame = 0;
    const loop = () => {
      now = performance.now();
      cursorView = [cursor.x, cursor.y];
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  });
  const phase = $derived(
    mine ? strugglePhase(mine.arrest, (now - receivedAt) / 1000) : 0,
  );
  const ready = $derived(mine ? mine.arrest.lockout - (now - receivedAt) / 1000 <= 0 : false);
  let padEl = $state<SVGSVGElement | null>(null);
  function move(event: PointerEvent) {
    if (!tracing || !padEl || event.pointerType === "touch") return;
    if (document.pointerLockElement) {
      const box = padEl.getBoundingClientRect();
      cursor.pointerDelta(
        (event.movementX / box.width) * 2.8,
        (-event.movementY / box.height) * 1.8,
      );
      return;
    }
    place(event);
  }
  function place(event: PointerEvent) {
    if (!padEl) return;
    const box = padEl.getBoundingClientRect();
    cursor.pointer(
      ((event.clientX - box.left) / box.width) * 2.8 - 1.4,
      -(((event.clientY - box.top) / box.height) * 1.8 - 0.9),
    );
  }
  const STATIONS = {
    power: "Cut the power",
    vault: "Crack the vault",
    jail: "Pick the jail lock",
    van: "Toss the bag in the van",
    loot: "Seize the bag",
  } as const;
  let tapAt = $state(performance.now());
  $effect(() => {
    void city.tap;
    tapAt = performance.now();
  });
  const tapSince = $derived(((now - tapAt) / 1000) * city.slow);
  const tapPhase = $derived(
    city.tap ? (city.tap.phase + tapSince / city.tap.period) % 1 : 0,
  );
  const tapReady = $derived(city.tap ? city.tap.lockout - tapSince <= 0 : false);
  const slowDepth = $derived(Math.max(0, Math.min(1, (1 - city.slow) / 0.65)));
</script>

<svelte:window onpointermove={move} />
{#if slowDepth > 0.01}<div
    class="slowmo"
    style={`opacity:${slowDepth}`}
    aria-hidden="true"
  ></div>{/if}
{#if mine}
  {@const arrest = mine.arrest}
  <section
    class="arrest"
    class:officer={mine.officer}
    class:mistake={arrest.flash > 0}
    aria-live="polite"
    aria-label={mine.officer ? "Cuffing challenge" : "You are being arrested"}
  >
    <header>
      <Handcuffs size={20} />
      <strong
        >{arrest.stage === "cuffing"
          ? mine.officer
            ? `Cuffing ${name(arrest.robber)}`
            : `${name(arrest.officer)} is cuffing you`
          : mine.officer
            ? `Escorting ${name(arrest.robber)}`
            : `${name(arrest.officer)} is dragging you away`}</strong
      >
      <small>{Math.max(0, Math.ceil(arrest.remaining))}s</small>
    </header>
    {#if tracing}
      <svg
        bind:this={padEl}
        class="pad"
        viewBox="-1.4 -0.9 2.8 1.8"
        role="application"
        aria-label="Trace pad. Follow the glowing dot around the figure eight."
        onpointerdown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          place(event);
        }}
        onpointermove={(event) => {
          if (event.pointerType === "touch") place(event);
        }}
      >
        <polyline class="track" points={curve} />
        <polyline class="done" points={done} />
        <g transform={`translate(${guide[0]} ${-guide[1]}) rotate(${arrow})`}>
          <circle class="guide" r="0.09" />
          <path class="ahead" d="M0.13 -0.07 0.24 0 0.13 0.07" />
        </g>
        <circle class="cursor" cx={cursorView[0]} cy={-cursorView[1]} r="0.06" />
      </svg>
      <div class="meter" aria-label={`Cuffed ${Math.round(arrest.progress * 100)} percent`}>
        <i style={`width:${arrest.progress * 100}%`}></i>
      </div>
      <p>
        {arrest.flash > 0
          ? "Off the line. Progress lost."
          : pad
            ? "Trace the figure eight with the left stick. Stay on the line."
            : "Trace the figure eight with the mouse, or steer with WASD. Stay on the line."}
      </p>
    {:else if mine.officer}
      <div class="meter danger" aria-label={`Escape ${Math.round(arrest.escape * 100)} percent`}>
        <i style={`width:${arrest.escape * 100}%`}></i>
      </div>
      <p>
        <Footprints size={15} /> Drag them to your patrol car or the jail. If the red bar fills, they break free.
      </p>
    {:else}
      {@const band = inBand(arrest, phase)}
      <div
        class="beat"
        class:hot={band && ready}
        class:locked={!ready}
        aria-label="Struggle timing bar"
      >
        <span
          class="band"
          style={`left:${(0.5 - arrest.window) * 100}%;width:${arrest.window * 200}%`}
        ></span>
        <span class="ring" style={`left:${phase * 100}%`}></span>
      </div>
      {#if arrest.stage === "cuffing"}
        <div class="meter danger" aria-label={`Cuffed ${Math.round(arrest.progress * 100)} percent`}>
          <i style={`width:${arrest.progress * 100}%`}></i>
        </div>
      {:else}
        <div class="meter free" aria-label={`Escape ${Math.round(arrest.escape * 100)} percent`}>
          <i style={`width:${arrest.escape * 100}%`}></i>
        </div>
      {/if}
      <p>
        {#if arrest.jolt > 0}Good struggle!{:else if !ready}Mistimed. Wait for the next beat.{:else}Press <kbd>E</kbd> or <kbd>Space</kbd>{pad ? ", or A," : ""} as the ring crosses the band.{/if}
        {arrest.stage === "cuffing"
          ? "Each hit pushes their cuffing back."
          : "Fill the bar to break free. Teammates can hold E on the officer."}
      </p>
      <button
        class="struggle"
        aria-label="Struggle"
        onpointerdown={(event) => {
          event.preventDefault();
          onstruggle();
        }}><Hand size={20} />Struggle</button
      >
    {/if}
  </section>
{:else}
  {#if city.tap}<section class="station" aria-live="polite" aria-label="Work station beat">
      <strong>{STATIONS[city.tap.station]}</strong>
      <div class="beat" class:hot={tapReady && Math.abs(tapPhase - 0.5) <= city.tap.window} class:locked={!tapReady}>
        <span class="band" style={`left:${(0.5 - city.tap.window) * 100}%;width:${city.tap.window * 200}%`}></span>
        <span class="ring" style={`left:${tapPhase * 100}%`}></span>
      </div>
      <p>
        {tapReady
          ? `Hold ${pad ? "A" : "E"} to work. Tap it again as the ring crosses the band to work much faster.`
          : "Off the beat. Wait for the next one."}
      </p>
    </section>{/if}
  {#if city.grab > 0}<div class="grab" aria-label="Grabbing">
      <Handcuffs size={16} />Grabbing<span class="track"><i style={`width:${city.grab * 100}%`}></i></span>
    </div>{:else if city.cuff_ready > 0}<div class="grab cooling">
      <Handcuffs size={16} />Cuffs ready in {Math.ceil(city.cuff_ready)}s
    </div>{/if}
  {#if others.length}<div class="others" aria-label="Arrests nearby">
      {#each others as other (other.robber)}<span
          ><Shield size={14} />{name(other.officer)}
          {other.stage === "cuffing" ? "cuffing" : "dragging"}
          {name(other.robber)}
          {#if other.stage === "escort"}<Users size={14} />{:else}<TriangleAlert size={14} />{/if}<i
            style={`width:${(other.stage === "cuffing" ? other.progress : other.escape) * 100}%`}
          ></i></span
        >{/each}
    </div>{/if}
{/if}

<style>
  .slowmo {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(ellipse at center, #0000 45%, #0a1b3acc 100%);
    backdrop-filter: saturate(0.55);
    transition: opacity 0.12s;
  }
  .arrest {
    position: absolute;
    left: 50%;
    bottom: 3.4rem;
    transform: translateX(-50%);
    width: min(360px, calc(100vw - 2rem));
    padding: 0.8rem 1rem 1rem;
    border-radius: 10px;
    background: #08131ef2;
    border: 1px solid #ffce7b55;
    pointer-events: auto;
    display: grid;
    gap: 0.6rem;
    color: #f3f8ff;
  }
  .arrest.officer {
    border-color: #7acfff66;
  }
  .arrest.mistake {
    animation: shake 0.3s;
    border-color: #ff6b81;
  }
  @keyframes shake {
    25% {
      transform: translateX(calc(-50% - 6px));
    }
    75% {
      transform: translateX(calc(-50% + 6px));
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  header strong {
    flex: 1;
    font-size: 0.95rem;
  }
  header small {
    font-variant-numeric: tabular-nums;
    color: #a8bacb;
  }
  .pad {
    width: 100%;
    aspect-ratio: 2.8 / 1.8;
    background: #0d1c2a;
    border-radius: 8px;
    touch-action: none;
    cursor: crosshair;
  }
  .track,
  .done {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .track {
    stroke: #2c4558;
    stroke-width: 0.48;
  }
  .done {
    stroke: #7acfff;
    stroke-width: 0.12;
  }
  .guide {
    fill: #baff52;
  }
  .ahead {
    fill: none;
    stroke: #baff52;
    stroke-width: 0.04;
  }
  .cursor {
    fill: #fff;
    stroke: #07111c;
    stroke-width: 0.02;
  }
  .meter {
    height: 8px;
    border-radius: 4px;
    background: #ffffff1a;
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: #7acfff;
    transition: width 0.1s linear;
  }
  .meter.danger i {
    background: #ff6b81;
  }
  .meter.free i {
    background: #baff52;
  }
  p {
    margin: 0;
    font-size: 0.82rem;
    line-height: 1.45;
    color: #d6e2ee;
  }
  p :global(svg) {
    vertical-align: -3px;
  }
  kbd {
    font: inherit;
    font-size: 0.7rem;
    padding: 0 0.3rem;
    border-radius: 3px;
    background: #ffffff20;
  }
  .beat {
    position: relative;
    height: 26px;
    border-radius: 13px;
    background: #ffffff12;
  }
  .beat.locked {
    opacity: 0.45;
  }
  .band {
    position: absolute;
    top: 3px;
    bottom: 3px;
    border-radius: 10px;
    background: #baff5255;
    border: 1px solid #baff52;
  }
  .beat.hot .band {
    background: #baff52aa;
  }
  .ring {
    position: absolute;
    top: 50%;
    width: 18px;
    height: 18px;
    margin: -9px 0 0 -9px;
    border-radius: 50%;
    border: 3px solid #fff;
  }
  .struggle {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem;
    border-radius: 8px;
    border: 1px solid #ffce7b88;
    background: #ffce7b22;
    color: inherit;
    font: inherit;
    font-weight: 700;
    touch-action: manipulation;
  }
  .station {
    position: absolute;
    left: 50%;
    bottom: 3.4rem;
    transform: translateX(-50%);
    width: min(320px, calc(100vw - 2rem));
    display: grid;
    gap: 0.45rem;
    padding: 0.6rem 0.8rem 0.7rem;
    border-radius: 10px;
    background: #08131ee6;
    border: 1px solid #baff5244;
  }
  .station strong {
    font-size: 0.85rem;
  }
  .grab {
    position: absolute;
    left: 50%;
    bottom: 8rem;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.7rem;
    border-radius: 7px;
    background: #08131ee0;
    font-size: 0.8rem;
  }
  .grab .track {
    width: 80px;
    height: 6px;
    border-radius: 3px;
    background: #ffffff1a;
    overflow: hidden;
  }
  .grab i {
    display: block;
    height: 100%;
    background: #7acfff;
  }
  .grab.cooling {
    color: #a8bacb;
  }
  .others {
    position: absolute;
    top: 8.6rem;
    left: 50%;
    transform: translateX(-50%);
    display: grid;
    gap: 0.3rem;
  }
  .others span {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.3rem 0.6rem 0.45rem;
    border-radius: 6px;
    background: #08131ed9;
    font-size: 0.75rem;
    overflow: hidden;
  }
  .others i {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    background: #ff6b81;
  }
</style>
