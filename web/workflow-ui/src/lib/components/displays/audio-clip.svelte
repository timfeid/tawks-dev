<script lang="ts">
  import { onDestroy } from 'svelte';
  import { interpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';
  import { Play, Square } from '@lucide/svelte';

  let { display, inputs }: Props = $props();

  let clip = $derived.by(() => {
    if (typeof display.type === 'string' || !('AudioClip' in display.type)) return;
    return display.type.AudioClip;
  });

  function resolved(template: string | undefined): string {
    if (!template) return '';
    const out = interpolate(inputs, template);
    return /\{[^{}]+\}/.test(out) ? '' : out;
  }

  let src = $derived(resolved(clip?.src));
  let limit = $derived(Number(resolved(clip?.seconds)) || 0);
  let marks = $derived(clip?.marks ?? []);
  let span = $derived(Math.max(...marks, 1));

  let audio = $state<HTMLAudioElement>();
  let playing = $state(false);
  let position = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;

  function stop() {
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
    }
    playing = false;
    position = 0;
    if (timer) clearInterval(timer);
    timer = null;
  }

  async function play() {
    if (!audio || !src) return;
    if (playing) {
      stop();
      return;
    }
    audio.currentTime = 0;
    try {
      await audio.play();
    } catch {
      return;
    }
    playing = true;
    // Poll finely: the timeupdate event fires too rarely to cut a one-second clip cleanly.
    timer = setInterval(() => {
      if (!audio) return;
      position = audio.currentTime;
      if (audio.currentTime >= limit || audio.ended) stop();
    }, 40);
  }

  // A new day, a new song: never keep an old clip running.
  $effect(() => {
    src;
    limit;
    stop();
  });

  onDestroy(stop);

  const pct = (s: number) => `${Math.min(100, (s / span) * 100)}%`;
  const label = (s: number) => `0:${String(Math.floor(s)).padStart(2, '0')}`;
</script>

{#if clip && src}
  <div class="flex flex-col gap-2 {display.class}" data-audio-clip data-limit={limit}>
    <audio bind:this={audio} {src} preload="auto"></audio>
    <div class="relative h-3 w-full overflow-hidden rounded-full bg-white/10">
      <div class="absolute inset-y-0 left-0 bg-white/25" style="width: {pct(Math.min(limit, span))}"></div>
      <div class="absolute inset-y-0 left-0 bg-primary transition-[width] duration-75" style="width: {pct(position)}"></div>
      {#each marks as mark (mark)}
        <div class="absolute inset-y-0 w-px bg-black/50" style="left: {pct(mark)}"></div>
      {/each}
    </div>
    <div class="flex items-center gap-3">
      <button
        type="button"
        class="flex size-12 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground transition hover:brightness-110"
        onclick={play}
        aria-label={playing ? 'Stop' : 'Play'}
      >
        {#if playing}<Square class="size-5" />{:else}<Play class="size-5" />{/if}
      </button>
      <span class="font-mono text-sm tabular-nums text-zinc-300">{label(position)} / {label(Math.min(limit, span))}</span>
    </div>
  </div>
{/if}
