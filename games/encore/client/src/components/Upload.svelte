<script lang="ts">
  import { FileMusic, FolderOpen, LoaderCircle, Upload as UploadIcon, X } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { fetchMe, uploadSong } from '../lib/songs';
  import type { Me } from '../lib/types';

  let { conn, onClose, onUploaded }: { conn: RoomConnection; onClose: () => void; onUploaded: () => void } = $props();

  const KEEP = /^(song\.ini|notes\.(mid|chart)|album\.(png|jpe?g)|[a-z_0-9]+\.(ogg|opus|mp3))$/;

  let me = $state<Me | null>(null);
  let meFailed = $state<string | null>(null);
  let files = $state<File[]>([]);
  let name = $state('');
  let artist = $state('');
  let rights = $state(false);
  let sending = $state(false);
  let problem = $state<string | null>(null);
  let skipped = $state<string[]>([]);

  fetchMe(conn)
    .then((value) => (me = value))
    .catch((error: Error) => (meFailed = error.message));

  const status = $derived(me?.status ?? null);
  const blocked = $derived(!!status && !status.can_upload);
  const totalMb = $derived(files.reduce((sum, f) => sum + f.size, 0) / (1024 * 1024));
  const tooLarge = $derived(!!me && totalMb > me.limits.max_total_mb);
  const ready = $derived(
    !!me && !blocked && !sending && files.length > 0 && !tooLarge && rights && !!name.trim() && !!artist.trim(),
  );

  function iniValue(text: string, key: string) {
    const line = text.split(/\r?\n/).find((l) => l.split('=')[0]?.trim().toLowerCase() === key);
    return line ? line.slice(line.indexOf('=') + 1).replace(/<[^>]*>/g, '').trim() : '';
  }

  async function choose(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const picked = Array.from(input.files ?? []);
    input.value = '';
    files = picked.filter((f) => KEEP.test(f.name.toLowerCase()));
    skipped = picked.filter((f) => !KEEP.test(f.name.toLowerCase())).map((f) => f.name);
    problem = files.length ? null : 'None of those files are ones Encore plays.';
    const ini = files.find((f) => f.name.toLowerCase() === 'song.ini');
    if (ini) {
      const text = await ini.text();
      name = name.trim() || iniValue(text, 'name');
      artist = artist.trim() || iniValue(text, 'artist');
    }
  }

  async function send() {
    if (!ready) return;
    sending = true;
    problem = null;
    try {
      const result = await uploadSong(conn, files, { name, artist, rights });
      onUploaded();
      conn.flash(`${result.song.name} is uploaded. It shows up for the people in rooms you are in.`);
      onClose();
    } catch (error) {
      problem = (error as Error).message;
    } finally {
      sending = false;
    }
  }
</script>

<div class="absolute inset-0 z-20 flex items-center justify-center bg-black/70 p-4">
  <div class="flex max-h-full w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-white/10 bg-background shadow-2xl">
    <header class="flex items-center border-b border-white/10 px-5 py-3">
      <h2 class="font-semibold">Upload a song</h2>
      <button class="ml-auto rounded-md p-1 text-neutral-400 hover:bg-white/10 hover:text-white" onclick={onClose} aria-label="Close">
        <X class="size-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto p-5 text-sm">
      {#if meFailed}
        <p class="text-neutral-400">{meFailed}</p>
      {:else if !me}
        <p class="flex items-center gap-2 text-neutral-400"><LoaderCircle class="size-4 animate-spin" /> Checking your uploads</p>
      {:else if blocked}
        <p class="rounded-lg border border-white/10 bg-white/5 p-3 text-neutral-300">{status?.reason}</p>
      {:else}
        <p class="leading-relaxed text-neutral-400">
          Pick a Clone Hero song folder: the chart (<span class="font-mono">notes.mid</span> or
          <span class="font-mono">notes.chart</span>) and the audio as Ogg, Opus or MP3, either one
          <span class="font-mono">song.ogg</span> or stems like <span class="font-mono">guitar.ogg</span>.
          <span class="font-mono">song.ini</span> and <span class="font-mono">album.png</span> are optional. Up to
          {me.limits.max_total_mb} MB and {me.limits.max_minutes} minutes.
          {#if status}You have {status.songs} of {status.max_songs} songs.{/if}
        </p>

        <div class="flex flex-wrap gap-2">
          <label class="flex cursor-pointer items-center gap-2 rounded-lg border border-white/10 px-3 py-2 hover:border-white/30">
            <FolderOpen class="size-4" /> Choose a folder
            <input type="file" class="hidden" webkitdirectory multiple onchange={choose} />
          </label>
          <label class="flex cursor-pointer items-center gap-2 rounded-lg border border-white/10 px-3 py-2 hover:border-white/30">
            <FileMusic class="size-4" /> Choose files
            <input type="file" class="hidden" multiple accept=".mid,.chart,.ogg,.opus,.mp3,.ini,.png,.jpg,.jpeg" onchange={choose} />
          </label>
        </div>

        {#if files.length}
          <div class="space-y-1 rounded-lg bg-white/[0.04] p-3">
            {#each files as file, i (i)}
              <div class="flex justify-between gap-3 font-mono text-xs">
                <span class="truncate">{file.name}</span>
                <span class="shrink-0 text-neutral-500">{(file.size / (1024 * 1024)).toFixed(1)} MB</span>
              </div>
            {/each}
            {#if skipped.length}
              <div class="pt-1 text-xs text-neutral-500">Left out: {skipped.join(', ')}</div>
            {/if}
            {#if tooLarge}
              <div class="pt-1 text-xs text-red-400">That is {totalMb.toFixed(0)} MB; the limit is {me.limits.max_total_mb} MB.</div>
            {/if}
          </div>
        {/if}

        <div class="grid grid-cols-2 gap-3">
          <label class="space-y-1">
            <span class="text-xs text-neutral-500">Title</span>
            <input bind:value={name} maxlength="120" class="w-full rounded-lg bg-white/5 px-3 py-1.5 outline-none focus:bg-white/10" />
          </label>
          <label class="space-y-1">
            <span class="text-xs text-neutral-500">Artist</span>
            <input bind:value={artist} maxlength="120" class="w-full rounded-lg bg-white/5 px-3 py-1.5 outline-none focus:bg-white/10" />
          </label>
        </div>

        <label class="flex cursor-pointer items-start gap-2.5 rounded-lg border border-white/10 p-3">
          <input type="checkbox" bind:checked={rights} class="mt-0.5 accent-current" />
          <span>I own this or have the right to use it</span>
        </label>

        <p class="text-xs leading-relaxed text-neutral-500">
          Your song stays private. It only shows up in rooms you are in, and the people playing with you hear it. Songs
          reported for copyright are taken down, and accounts with repeated takedowns lose upload access. Upload rules:
          tawks.app/encore-uploads
        </p>
      {/if}

      {#if problem}
        <p class="text-red-400">{problem}</p>
      {/if}
    </div>

    <footer class="flex justify-end gap-2 border-t border-white/10 px-5 py-3">
      <button class="rounded-lg px-4 py-1.5 text-sm text-neutral-300 hover:bg-white/10" onclick={onClose}>Cancel</button>
      <button
        class="flex items-center gap-2 rounded-lg bg-white px-4 py-1.5 text-sm font-semibold text-black hover:bg-neutral-200 disabled:opacity-50"
        disabled={!ready}
        onclick={send}
      >
        {#if sending}<LoaderCircle class="size-4 animate-spin" /> Uploading{:else}<UploadIcon class="size-4" /> Upload{/if}
      </button>
    </footer>
  </div>
</div>
