<script lang="ts">
  import { Flag, LoaderCircle, X } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { reportSong } from '../lib/songs';
  import { REPORT_REASONS, type ReportReason, type SongInfo } from '../lib/types';

  let { conn, song, onClose }: { conn: RoomConnection; song: SongInfo; onClose: () => void } = $props();

  let reason = $state<ReportReason>('copyright');
  let details = $state('');
  let sending = $state(false);
  let problem = $state<string | null>(null);

  async function send() {
    sending = true;
    problem = null;
    try {
      await reportSong(conn, song.id, reason, details);
      conn.flash('Thanks. The report went to the Tawks team.');
      onClose();
    } catch (error) {
      problem = (error as Error).message;
    } finally {
      sending = false;
    }
  }
</script>

<div class="absolute inset-0 z-20 flex items-center justify-center bg-black/70 p-4">
  <div class="flex max-h-full w-full max-w-md flex-col overflow-hidden rounded-2xl border border-white/10 bg-background shadow-2xl">
    <header class="flex items-center border-b border-white/10 px-5 py-3">
      <h2 class="font-semibold">Report {song.name}</h2>
      <button class="ml-auto rounded-md p-1 text-neutral-400 hover:bg-white/10 hover:text-white" onclick={onClose} aria-label="Close">
        <X class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-5 text-sm">
      <p class="text-neutral-400">
        Uploaded by {song.uploader_name ?? 'a player'}. The Tawks team reviews every report and takes down songs that
        break the rules.
      </p>
      <div class="space-y-1.5">
        {#each REPORT_REASONS as option (option.key)}
          <label class="flex cursor-pointer items-center gap-2.5 rounded-lg border px-3 py-2 {reason === option.key ? 'border-primary bg-primary/10' : 'border-white/10'}">
            <input type="radio" name="reason" value={option.key} bind:group={reason} class="accent-current" />
            {option.label}
          </label>
        {/each}
      </div>
      <label class="block space-y-1">
        <span class="text-xs text-neutral-500">
          {reason === 'copyright' ? 'If you own the rights, say who you are and what the work is' : 'Anything that helps (optional)'}
        </span>
        <textarea bind:value={details} maxlength="1000" rows="3" class="w-full resize-none rounded-lg bg-white/5 px-3 py-2 outline-none focus:bg-white/10"></textarea>
      </label>
      {#if problem}
        <p class="text-red-400">{problem}</p>
      {/if}
    </div>

    <footer class="flex justify-end gap-2 border-t border-white/10 px-5 py-3">
      <button class="rounded-lg px-4 py-1.5 text-sm text-neutral-300 hover:bg-white/10" onclick={onClose}>Cancel</button>
      <button
        class="flex items-center gap-2 rounded-lg bg-white px-4 py-1.5 text-sm font-semibold text-black hover:bg-neutral-200 disabled:opacity-50"
        disabled={sending}
        onclick={send}
      >
        {#if sending}<LoaderCircle class="size-4 animate-spin" />{:else}<Flag class="size-4" />{/if} Send report
      </button>
    </footer>
  </div>
</div>
