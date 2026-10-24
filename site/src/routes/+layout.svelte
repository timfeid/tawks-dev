<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import { Toaster } from 'svelte-sonner';

  let { children } = $props();
  const links = [
    { href: '/docs/overview', label: 'Docs', match: '/docs' },
    { href: '/playground', label: 'Playground', match: '/playground' },
  ];
</script>

<div class="flex min-h-screen flex-col bg-background text-foreground">
  <header class="sticky top-0 z-20 flex h-14 shrink-0 items-center gap-6 border-b bg-background/90 px-4 backdrop-blur md:px-6">
    <a href="/" class="font-semibold tracking-tight">tawks <span class="text-primary">dev</span></a>
    <nav class="flex items-center gap-4 text-sm">
      {#each links as link (link.href)}
        <a
          href={link.href}
          class="transition-colors {page.url.pathname.startsWith(link.match) ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'}"
        >
          {link.label}
        </a>
      {/each}
    </nav>
    <a
      href="https://github.com/timfeid/tawks-dev"
      class="ml-auto text-sm text-muted-foreground hover:text-foreground"
      target="_blank"
      rel="noopener noreferrer">GitHub</a
    >
  </header>
  <main class="flex min-h-0 flex-1 flex-col">
    {@render children()}
  </main>
  <Toaster theme="dark" position="bottom-right" />
</div>
