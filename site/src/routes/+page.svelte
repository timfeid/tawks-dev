<script lang="ts">
  import { BookOpen, Code2, Github } from '@lucide/svelte';
</script>

<svelte:head>
  <title>tawks developers</title>
  <meta name="description" content="Build bots, pages and games for tawks: docs, a live playground, SDKs and examples." />
</svelte:head>

<section class="mx-auto flex w-full max-w-4xl flex-col gap-10 px-4 py-16 md:py-24">
  <div class="flex flex-col gap-4">
    <h1 class="text-4xl font-extrabold tracking-tight md:text-5xl">Build for tawks</h1>
    <p class="max-w-2xl text-lg text-muted-foreground">
      Bots on tawks run pages, daily puzzles and multiplayer games inside communities. Describe a page as data, answer
      its actions from your bot, and tawks draws it on web, desktop and mobile.
    </p>
  </div>
  <div class="grid gap-4 md:grid-cols-3">
    <a href="/docs/overview" class="flex flex-col gap-2 rounded-xl border bg-card p-5 transition-colors hover:border-primary/60">
      <BookOpen class="size-5 text-primary" />
      <span class="font-semibold">Docs</span>
      <span class="text-sm text-muted-foreground">From your first bot to lobby games and Tawkathon bouts.</span>
    </a>
    <a href="/playground" class="flex flex-col gap-2 rounded-xl border bg-card p-5 transition-colors hover:border-primary/60">
      <Code2 class="size-5 text-primary" />
      <span class="font-semibold">Playground</span>
      <span class="text-sm text-muted-foreground">Build a page, see it drawn by the real renderer, and copy the code.</span>
    </a>
    <a
      href="https://github.com/timfeid/tawks-dev"
      target="_blank"
      rel="noopener noreferrer"
      class="flex flex-col gap-2 rounded-xl border bg-card p-5 transition-colors hover:border-primary/60"
    >
      <Github class="size-5 text-primary" />
      <span class="font-semibold">SDKs and examples</span>
      <span class="text-sm text-muted-foreground">The Rust and TypeScript SDKs, example games and a local stand-in for tawks.</span>
    </a>
  </div>
</section>
