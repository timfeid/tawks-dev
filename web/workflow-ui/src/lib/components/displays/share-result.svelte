<script lang="ts">
  import { Share2 } from '@lucide/svelte';
  import { getContext } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { getNestedValue } from '../../utils';
  import type { Props } from '../workflow-display.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  type Share = { path: string; title: string; text: string };

  let { display, inputs }: Props = $props();

  const spec = $derived(
    typeof display.type !== 'string' && 'ShareResult' in display.type ? display.type.ShareResult : null,
  );
  const share = $derived((spec ? getNestedValue(inputs, spec.value) : null) as Share | null);
  const provider = getContext<WorkflowStateProvider | undefined>('workflow-provider');

  // Made ahead of the tap: Safari only lets a tap copy or open the share
  // sheet if nothing slow runs in between.
  let short = $state<{ path: string; url: string } | null>(null);
  $effect(() => {
    const path = share?.path;
    if (!path || !provider?.shortenLink || short?.path === path) return;
    provider
      .shortenLink(path)
      .then((url) => (short = { path, url }))
      .catch(() => {});
  });

  /** The desktop app's own origin is not a web address anyone else can open. */
  function webUrl(path: string) {
    const origin = window.location.protocol.startsWith('http') ? window.location.origin : 'https://tawks.app';
    return new URL(path, origin).toString();
  }

  async function send() {
    if (!share) return;
    const url = short?.path === share.path ? short.url : webUrl(share.path);
    if (navigator.share && matchMedia('(pointer: coarse)').matches) {
      try {
        await navigator.share({ title: share.title, text: share.text, url });
        return;
      } catch (e) {
        if ((e as Error).name === 'AbortError') return;
      }
    }
    try {
      await navigator.clipboard.writeText(`${share.text}\n${url}`);
      toast.success('Result copied, paste it anywhere');
    } catch {
      toast.error('Could not copy the result');
    }
  }
</script>

{#if share?.path}
  <button
    type="button"
    class={display.class ||
      'bg-primary text-primary-foreground inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm font-bold'}
    onclick={send}
  >
    <Share2 class="size-4" />
    {spec?.label || 'Share result'}
  </button>
{/if}
