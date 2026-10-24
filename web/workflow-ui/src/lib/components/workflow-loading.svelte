<script lang="ts">
  import { onMount } from 'svelte';
  import gsap from 'gsap';

  let orb: HTMLDivElement;
  let pulse: HTMLDivElement;

  onMount(() => {
    gsap.set([orb, pulse], { transformOrigin: 'center center' });

    // Glow pulse animation
    gsap.to(pulse, {
      scale: 2.5,
      opacity: 0,
      duration: 1.5,
      repeat: -1,
      ease: 'power2.out',
    });

    // Subtle breathing glow
    gsap.to(orb, {
      scale: 1.1,
      duration: 1.2,
      yoyo: true,
      repeat: -1,
      ease: 'sine.inOut',
    });
  });
</script>

<div class="flex h-[24rem] w-full items-center justify-center">
  <div class="relative flex h-24 w-24 items-center justify-center">
    <div
      bind:this={pulse}
      class="absolute h-24 w-24 rounded-full bg-purple-500 opacity-30 blur-2xl"
    ></div>
    <div
      bind:this={orb}
      class="relative z-10 h-12 w-12 rounded-full bg-gradient-to-br from-teal-400 to-green-500 shadow-lg"
    >
      <div class="absolute inset-0 animate-ping rounded-full bg-white opacity-5"></div>
    </div>
  </div>
</div>
