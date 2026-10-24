<script lang="ts">
  let { data } = $props();
</script>

<svelte:head>
  <title>{data.title} · tawks developers</title>
</svelte:head>

<div class="mx-auto flex w-full max-w-7xl gap-8 px-4 py-8 md:px-6">
  <aside class="sticky top-20 hidden h-[calc(100vh-6rem)] w-56 shrink-0 overflow-y-auto md:block">
    <nav class="flex flex-col gap-1 text-sm">
      {#each data.nav as item (item.slug)}
        <a
          href="/docs/{item.slug}"
          class="rounded-md px-2 py-1.5 {item.slug === data.slug ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:text-foreground'}"
        >
          {item.title}
        </a>
      {/each}
    </nav>
  </aside>
  <article class="prose prose-invert min-w-0 max-w-3xl flex-1 prose-headings:scroll-mt-20 prose-a:text-primary prose-code:before:content-none prose-code:after:content-none prose-pre:border prose-pre:bg-card prose-img:rounded-lg prose-img:border">
    <!-- The docs are this repository's own markdown, rendered with raw HTML disabled. -->
    {@html data.html}
    <div class="not-prose mt-12 flex justify-between gap-4 border-t pt-6 text-sm">
      {#if data.prev}<a class="text-muted-foreground hover:text-foreground" href="/docs/{data.prev.slug}">Previous: {data.prev.title}</a>{:else}<span></span>{/if}
      {#if data.next}<a class="text-muted-foreground hover:text-foreground" href="/docs/{data.next.slug}">Next: {data.next.title}</a>{/if}
    </div>
  </article>
  <aside class="sticky top-20 hidden h-[calc(100vh-6rem)] w-52 shrink-0 overflow-y-auto xl:block">
    <div class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">On this page</div>
    <nav class="flex flex-col gap-1 text-sm">
      {#each data.toc as h (h.id)}
        <a href="#{h.id}" class="text-muted-foreground hover:text-foreground {h.level === 3 ? 'pl-3' : ''}">{h.text}</a>
      {/each}
    </nav>
  </aside>
</div>
