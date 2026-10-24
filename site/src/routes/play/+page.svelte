<script lang="ts">
  import Play from '$lib/play/Play.svelte';
</script>

<svelte:head>
  <title>Play · tawks developers</title>
  <meta name="description" content="Play the games connected to the tawks sandbox, every seat side by side, in your browser." />
  <!-- The display fonts game pages use, as tawks loads them. -->
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link href="https://fonts.googleapis.com/css2?family=Fjalla+One&family=Jolly+Lodger&display=swap" rel="stylesheet" />
  {@html `<style>.font-heading{font-family:'Fjalla One'}.font-halloween{font-family:'Jolly Lodger'}</style>`}
</svelte:head>

<Play />
