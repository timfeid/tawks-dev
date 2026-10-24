<script lang="ts">
  import { onMount } from "svelte";
  import { createTextureUtils } from "../textures";
  import type { CardView } from "../types";

  let {
    card,
    width = 240,
  }: {
    card: CardView | null | undefined;
    width?: number;
  } = $props();

  let containerEl: HTMLDivElement | null = $state(null);

  // Card aspect ratio matches the 3D texture: 300 x 440
  let height = $derived(Math.round(width * (440 / 300)));

  // Build a single texture utils instance for this preview.
  // The 3D scene uses cardW=0.84, cardH=1.18 with cardTextureScale=4.
  // We use the same scale for crisp rendering.
  const utils = createTextureUtils({
    cardTextureScale: 4,
    cardW: 0.84,
    cardH: 1.18,
  });

  // Re-render the card whenever it changes.
  $effect(() => {
    if (!containerEl || !card) return;

    // Clear any previous canvas
    while (containerEl.firstChild) {
      containerEl.removeChild(containerEl.firstChild);
    }

    const texture = utils.createCardTexture(card as any);
    if (!texture) return;

    const sourceCanvas = texture.image as HTMLCanvasElement | undefined;
    if (!sourceCanvas) return;

    // Mirror the canvas into a fresh DOM node so we can scale it freely.
    const display = document.createElement("canvas");
    display.width = sourceCanvas.width;
    display.height = sourceCanvas.height;
    display.style.width = "100%";
    display.style.height = "100%";
    display.style.display = "block";
    const ctx = display.getContext("2d");
    if (ctx) {
      ctx.drawImage(sourceCanvas, 0, 0);
    }
    containerEl.appendChild(display);

    // The texture asynchronously redraws itself when the card image loads.
    // Re-mirror after a short delay so the art shows up.
    const checkInterval = setInterval(() => {
      if (!containerEl || !ctx) return;
      ctx.drawImage(sourceCanvas, 0, 0);
    }, 80);

    const stop = setTimeout(() => clearInterval(checkInterval), 3000);

    return () => {
      clearInterval(checkInterval);
      clearTimeout(stop);
    };
  });
</script>

{#if card}
  <div
    bind:this={containerEl}
    class="rounded-xl overflow-hidden shadow-2xl"
    style={`width: ${width}px; height: ${height}px;`}
  ></div>
{/if}
