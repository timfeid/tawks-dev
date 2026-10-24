<script lang="ts">
  import Hls from 'hls.js';

  import { onDestroy } from 'svelte';
  import { interpolate } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';

  let { display, inputs }: Props = $props();

  let videoElement = $state<HTMLVideoElement>();
  let hls: Hls | null = null;
  let loaded = $state(false);

  let video = $derived.by(() => {
    if (typeof display.type === 'string' || !('Video' in display.type)) {
      return;
    }
    return display.type.Video;
  });

  // interpolate() returns the literal template (e.g. "{post.media_urls.0}")
  // when the path doesn't resolve. Treat unresolved templates as missing.
  function safeInterpolate(template: string | undefined | null): string {
    if (!template) return '';
    const result = interpolate(inputs, template);
    // If the result still contains an unresolved {…} placeholder, drop it.
    if (/\{[^{}]+\}/.test(result)) return '';
    return result;
  }

  let resolvedSrc = $derived(safeInterpolate(video?.src));
  let posterSrc = $derived(safeInterpolate(video?.poster_src));

  let isDirectVideo = $derived.by(() => {
    const lower = resolvedSrc.toLowerCase();
    const path = lower.split('?')[0];
    return (
      path.endsWith('.mp4') ||
      path.endsWith('.webm') ||
      path.endsWith('.mov') ||
      path.endsWith('.ogg') ||
      lower.includes('v.redd.it') ||
      lower.includes('/video/')
    );
  });

  let singleClickTimer: ReturnType<typeof setTimeout> | null = null;
  const SEEK_SECONDS = 10;
  const DOUBLE_CLICK_MS = 250;

  // Visual cue state
  let cuePlayPause = $state<'play' | 'pause' | null>(null);
  let cueSeek = $state<'left' | 'right' | null>(null);
  let cueTimer: ReturnType<typeof setTimeout> | null = null;
  let seekTimer: ReturnType<typeof setTimeout> | null = null;

  function flashCue(type: 'play' | 'pause') {
    cuePlayPause = type;
    if (cueTimer) clearTimeout(cueTimer);
    cueTimer = setTimeout(() => {
      cuePlayPause = null;
    }, 500);
  }

  function flashSeek(direction: 'left' | 'right') {
    cueSeek = direction;
    if (seekTimer) clearTimeout(seekTimer);
    seekTimer = setTimeout(() => {
      cueSeek = null;
    }, 500);
  }

  function handleOverlayClick(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!videoElement) return;

    // Defer single-click action so a follow-up dblclick can cancel it
    if (singleClickTimer) clearTimeout(singleClickTimer);
    singleClickTimer = setTimeout(() => {
      if (!videoElement) return;
      if (videoElement.paused) {
        videoElement.play().catch(() => {});
        flashCue('play');
      } else {
        videoElement.pause();
        flashCue('pause');
      }
      singleClickTimer = null;
    }, DOUBLE_CLICK_MS);
  }

  function handleOverlayDblClick(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (singleClickTimer) {
      clearTimeout(singleClickTimer);
      singleClickTimer = null;
    }
    if (!videoElement) return;

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = e.clientX - rect.left;
    const isLeftHalf = x < rect.width / 2;

    if (isLeftHalf) {
      videoElement.currentTime = Math.max(0, videoElement.currentTime - SEEK_SECONDS);
      flashSeek('left');
    } else {
      videoElement.currentTime = Math.min(
        videoElement.duration || Infinity,
        videoElement.currentTime + SEEK_SECONDS,
      );
      flashSeek('right');
    }
  }

  function loadAndPlay(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (!video || loaded) return;
    loaded = true;

    // Wait a tick for the <video> element to mount
    queueMicrotask(() => {
      if (!videoElement) return;

      if (isDirectVideo) {
        videoElement.src = resolvedSrc;
      } else if (Hls.isSupported()) {
        hls = new Hls();
        const baseUrl = (window as any).PUBLIC_BASE_URL || '/';
        const url = new URL(baseUrl + 'play-video');
        url.searchParams.set('url', resolvedSrc);
        url.searchParams.set('base_url', baseUrl);
        hls.loadSource(url.href);
        hls.attachMedia(videoElement);
      } else if (videoElement.canPlayType('application/vnd.apple.mpegurl')) {
        videoElement.src = resolvedSrc;
      } else {
        videoElement.src = resolvedSrc;
      }

      videoElement.play().catch(() => {});
    });
  }

  onDestroy(() => {
    hls?.destroy();
  });
</script>

{#if video}
  {#if !loaded && posterSrc}
    <button
      type="button"
      class="relative w-full h-full group cursor-pointer block bg-black rounded-md overflow-hidden border-0 p-0"
      onclick={loadAndPlay}
      aria-label="Play video"
    >
      <img src={posterSrc} alt="Video thumbnail" class="w-full h-auto block" />
      <div class="absolute inset-0 flex items-center justify-center bg-black/30 group-hover:bg-black/40 transition-colors">
        <div class="w-16 h-16 rounded-full bg-white/90 flex items-center justify-center shadow-2xl group-hover:scale-110 transition-transform">
          <svg viewBox="0 0 24 24" class="w-8 h-8 text-black ml-1" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        </div>
      </div>
    </button>
  {:else if !loaded}
    <button
      type="button"
      class="w-full aspect-video bg-black flex items-center justify-center cursor-pointer rounded-md border-0"
      onclick={loadAndPlay}
      aria-label="Play video"
    >
      <div class="w-16 h-16 rounded-full bg-white/90 flex items-center justify-center shadow-2xl hover:scale-110 transition-transform">
        <svg viewBox="0 0 24 24" class="w-8 h-8 text-black ml-1" fill="currentColor">
          <path d="M8 5v14l11-7z" />
        </svg>
      </div>
    </button>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="relative"
      role="presentation"
      onmousedown={(e) => e.stopPropagation()}
    >
      <video
        bind:this={videoElement}
        poster={posterSrc}
        controls
        playsinline
        class="w-full h-auto rounded-md"
      >
        <track kind="captions" />
      </video>

      <!-- Click overlay covers the video except the bottom controls bar -->
      <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
      <div
        class="absolute inset-x-0 top-0 cursor-pointer"
        style="bottom: 50px;"
        onclick={handleOverlayClick}
        ondblclick={handleOverlayDblClick}
      ></div>

      <!-- Play/Pause cue (center) -->
      {#if cuePlayPause}
        <div class="absolute inset-x-0 top-0 flex items-center justify-center pointer-events-none" style="bottom: 50px;">
          <div class="w-20 h-20 rounded-full bg-black/60 flex items-center justify-center video-cue-fade">
            {#if cuePlayPause === 'play'}
              <svg viewBox="0 0 24 24" class="w-10 h-10 text-white ml-1" fill="currentColor">
                <path d="M8 5v14l11-7z" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" class="w-10 h-10 text-white" fill="currentColor">
                <path d="M6 4h4v16H6zM14 4h4v16h-4z" />
              </svg>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Seek cue (left or right side) -->
      {#if cueSeek}
        <div
          class="absolute top-0 flex items-center justify-center pointer-events-none {cueSeek === 'left' ? 'left-0' : 'right-0'}"
          style="bottom: 50px; width: 50%;"
        >
          <div class="flex items-center gap-1 px-4 py-2 rounded-full bg-black/60 video-cue-fade">
            {#if cueSeek === 'left'}
              <svg viewBox="0 0 24 24" class="w-6 h-6 text-white" fill="currentColor">
                <path d="M11 18V6l-8.5 6 8.5 6zm.5-6l8.5 6V6l-8.5 6z" />
              </svg>
              <span class="text-white font-mono text-sm">-{SEEK_SECONDS}s</span>
            {:else}
              <span class="text-white font-mono text-sm">+{SEEK_SECONDS}s</span>
              <svg viewBox="0 0 24 24" class="w-6 h-6 text-white" fill="currentColor">
                <path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z" />
              </svg>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .video-cue-fade {
    animation: video-cue-fade 500ms ease-out forwards;
  }
  @keyframes video-cue-fade {
    0% { opacity: 0; transform: scale(0.7); }
    20% { opacity: 1; transform: scale(1); }
    100% { opacity: 0; transform: scale(1.15); }
  }
</style>