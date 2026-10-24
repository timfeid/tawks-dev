<script lang="ts">
  import { Toaster as Sonner, type ToasterProps as SonnerProps } from 'svelte-sonner';
  import { mode } from 'mode-watcher';

  let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
  class="toaster group"
  toastOptions={{
    duration: 7000,
    classes: {
      toast: `
        group toast
        group-[.toaster]:bg-zinc-950/80
        group-[.toaster]:backdrop-blur-xl
        group-[.toaster]:border-white/10
        group-[.toaster]:text-white
        group-[.toaster]:shadow-2xl
        group-[.toaster]:rounded-2xl
        group-[.toaster]:p-4
      `,
      description: 'group-[.toast]:text-zinc-400 group-[.toast]:font-medium',
      actionButton: 'group-[.toast]:bg-white group-[.toast]:text-zinc-950 group-[.toast]:font-bold',
      cancelButton:
        'group-[.toast]:bg-white/10 group-[.toast]:text-white hover:group-[.toast]:bg-white/20',
    },
  }}
  {...restProps}
/>
