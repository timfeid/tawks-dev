<script lang="ts">
  import { onMount } from 'svelte';
  import { inspectGlb } from './models/validate';
  import ModelPreview from './models/model-preview.svelte';
  import { fetchModelBytes } from './models/custom-model-rig';
  import { MODEL_LIMITS, type ImportedModel, type ModelClipMap, type ModelImportApi, type ModelPose, type ModelReport } from './models/types';

  let { api, onimport = () => {}, onequip, onclose = () => {} }: {
    api: ModelImportApi;
    onimport?: (model: ImportedModel) => void;
    onequip?: (model: ImportedModel) => void | Promise<void>;
    onclose?: () => void;
  } = $props();
  let library = $state<ImportedModel[]>([]);
  let file = $state<File | null>(null);
  let bytes = $state<ArrayBuffer | null>(null);
  let report = $state<ModelReport | null>(null);
  let clips = $state<ModelClipMap>({});
  let name = $state('');
  let error = $state('');
  let busy = $state(false);
  let loadingLibrary = $state(true);
  let pose = $state<ModelPose>('idle');
  let input: HTMLInputElement;
  let generation = 0;
  let previewAbort: AbortController | null = null;
  const canPlay = $derived(!!report?.rigged && !!clips.idle && !!clips.ride && clips.idle !== clips.ride);
  const previewReport = $derived(report ? {...report, clip_map: clips, playable: canPlay} : null);
  onMount(() => {let mounted = true; void api.list().then((models) => {if (mounted) library = models;}).catch((reason) => {if (mounted) error = message(reason);}).finally(() => {if (mounted) loadingLibrary = false;}); return () => {mounted = false; generation++; previewAbort?.abort();};});
  const message = (reason: unknown) => reason instanceof Error ? reason.message : 'The model library could not be reached.';
  async function select(chosen: File | undefined) {
    if (!chosen || busy) return;
    const ticket = ++generation;
    bytes = null; report = null; file = null; error = ''; pose = 'idle';
    if (!chosen.name.toLowerCase().endsWith('.glb')) {error = 'Choose a binary glTF (.glb) file.'; return;}
    if (chosen.size > MODEL_LIMITS.bytes) {error = 'Models can be at most 10 MB.'; return;}
    try {
      const data = await chosen.arrayBuffer();
      const inspected = inspectGlb(data);
      if (ticket !== generation) return;
      file = chosen; bytes = data; report = inspected; clips = {...inspected.clip_map}; name = chosen.name.replace(/\.glb$/i,'').slice(0,80);
    } catch (reason) {if (ticket === generation) error = message(reason);}
  }
  async function upload() {
    if (!file || !bytes || !report || busy || !name.trim()) return;
    busy = true; error = '';
    try {
      const imported = await api.upload(file, name.trim(), clips);
      library = [imported, ...library];
      onimport(imported);
      if (imported.report.playable && onequip) await onequip(imported);
      file = null; bytes = null; report = null;
    } catch (reason) {error = message(reason);}
    finally {busy = false;}
  }
  async function remove(model: ImportedModel) {
    if (busy) return;
    busy = true; error = '';
    try { await api.remove(model.id); library = library.filter((entry) => entry.id !== model.id); }
    catch (reason) {error = message(reason);}
    finally {busy = false;}
  }
  async function previewSaved(model: ImportedModel) {
    if (busy) return;
    const ticket = ++generation;
    busy = true; error = ''; file = null; bytes = null; report = null;
    previewAbort?.abort();
    previewAbort = new AbortController();
    try {
      const data = await fetchModelBytes(model.url, previewAbort.signal);
      const inspected = inspectGlb(data, model.report.clip_map);
      if (ticket !== generation) return;
      bytes = data; report = inspected; clips = {...inspected.clip_map}; name = model.name; pose = 'idle';
    } catch (reason) {if (ticket === generation) error = message(reason);}
    finally {if (ticket === generation) busy = false;}
  }
</script>

<section class="model-import" aria-label="Import a player model" aria-busy={busy}>
  <header><div><span class="eyebrow">YOUR LOOK</span><h2>Import a model</h2></div><button class="close" aria-label="Close model importer" onclick={onclose}>×</button></header>
  <p>Bring a self-contained GLB with its own textures. For gameplay, include a skinned rig plus idle and walk or run animations. Static models can be saved as previews.</p>
  <p class="muted">Custom models keep their own appearance. The base skin’s fitted clothing does not transfer.</p>
  <input bind:this={input} type="file" accept=".glb,model/gltf-binary" class="file-input" onchange={() => select(input.files?.[0])} disabled={busy} aria-label="Choose a GLB model" />
  <button class="choose" disabled={busy} onclick={() => input.click()}>Choose GLB <span>up to 10 MB</span></button>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if bytes && report && previewReport}
    <div class="editor">
      <ModelPreview data={bytes} report={previewReport} {pose} />
      <div class="details">
        <label>Model name<input bind:value={name} maxlength="80" disabled={busy || !file} /></label>
        <div class="budget">{report.triangles.toLocaleString()} triangles · {report.textures} textures · {report.rigged ? 'rigged' : 'static'}</div>
        {#if report.animations.length}
          {#each [['idle','Idle animation'],['ride','Walk / run animation']] as [key,label] (key)}
            <label>{label}<select value={clips[key as 'idle' | 'ride'] ?? ''} disabled={busy || !file} onchange={(event) => {clips = {...clips,[key]:event.currentTarget.value || null};}}><option value="">Choose a clip</option>{#each report.animations as animation (animation.name)}<option value={animation.name}>{animation.name} · {animation.duration.toFixed(1)}s</option>{/each}</select></label>
          {/each}
          <div class="pose"><button class:active={pose === 'idle'} onclick={() => (pose = 'idle')}>Idle</button><button class:active={pose === 'ride'} onclick={() => (pose = 'ride')}>Move</button></div>
        {/if}
        <div class="status" class:ready={canPlay}>{canPlay ? 'Ready for gameplay' : report.rigged ? 'Assign distinct idle and movement clips to play' : 'Preview only · a skinned rig is needed for gameplay'}</div>
        {#if file}<button class="primary" disabled={busy || !name.trim()} onclick={upload}>{busy ? 'Uploading…' : canPlay && onequip ? 'Upload and use' : 'Add to library'}</button>{/if}
      </div>
    </div>
  {/if}
  <div class="library-heading"><h3>Your models</h3><span>{library.length}/8</span></div>
  {#if loadingLibrary}<p class="muted" role="status">Loading your library…</p>{:else if !library.length}<p class="muted">Your uploaded models will appear here.</p>{/if}
  <ul class="library">
    {#each library as model (model.id)}
      <li><div><strong>{model.name}</strong><span>{model.report.playable ? 'Ready to play' : 'Preview only'} · {model.report.triangles.toLocaleString()} triangles</span></div><div class="actions"><button disabled={busy} onclick={() => previewSaved(model)}>Preview</button>{#if model.report.playable && onequip}<button disabled={busy} onclick={async () => {if (busy) return;busy=true;error='';try {await onequip?.(model);}catch(reason){error=message(reason);}finally{busy=false;}}}>Use</button>{/if}<button class="remove" disabled={busy} aria-label="Remove {model.name}" onclick={() => remove(model)}>Remove</button></div></li>
    {/each}
  </ul>
  <footer>60,000 vertices · 30,000 triangles · 8 embedded PNG/JPEG textures, up to 2048² each · 96 bones</footer>
</section>

<style>
  .model-import { width: min(48rem, 100%); padding: 1.25rem; color: #f8fafc; background: #09121e; border-top: 2px solid #67e8f9; font-family: system-ui,sans-serif; }
  header,.library-heading,.actions,.pose { display:flex;align-items:center;gap:0.7rem; }
  header,.library-heading { justify-content:space-between; }
  .eyebrow { color:#67e8f9;font-size:0.55rem;font-weight:900;letter-spacing:0.18em; }
  h2 { margin:0.2rem 0 0;font-size:1.5rem;font-weight:900; }
  h3 { margin:0;font-size:0.85rem;font-weight:800; }
  p { margin:0.65rem 0;color:#cbd5e1;font-size:0.78rem;line-height:1.6; }
  .muted,footer,.budget { color:#94a3b8;font-size:0.7rem; }
  button { border:0;background:#ffffff12;color:white;padding:0.5rem 0.75rem;font-size:0.72rem;font-weight:750;cursor:pointer; }
  button:hover { background:#ffffff22; }
  button:disabled { opacity:0.4;cursor:default; }
  button:focus-visible,input:focus-visible,select:focus-visible { outline:2px solid #67e8f9;outline-offset:2px; }
  .close { font-size:1.5rem;background:transparent;padding:0 0.4rem; }
  .choose { width:100%;display:flex;justify-content:center;gap:0.7rem;margin-top:1rem;border:1px dashed #ffffff40;padding:0.8rem; }
  .choose span { font-weight:400;color:#94a3b8; }
  .file-input { display:none; }
  .editor { display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:1rem;margin-top:1rem; }
  .details { display:flex;flex-direction:column;gap:0.7rem;justify-content:center; }
  label { display:flex;flex-direction:column;gap:0.3rem;font-size:0.7rem;font-weight:700; }
  input,select { box-sizing:border-box;width:100%;min-width:0;border:1px solid #ffffff25;background:#ffffff09;color:white;padding:0.5rem;font-size:0.75rem; }
  select option { background:#0f1f30; }
  .pose button { flex:1; }
  .pose .active { color:#67e8f9;background:#67e8f91a; }
  .status { color:#fcd34d;font-size:0.7rem; }
  .status.ready { color:#6ee7b7; }
  .primary { color:#042f2e;background:#67e8f9; }
  .primary:hover { background:#a5f3fc; }
  .error { color:#fda4af; }
  .library-heading { margin-top:1.2rem;border-top:1px solid #ffffff15;padding-top:1rem; }
  .library-heading span { color:#94a3b8;font-size:0.7rem; }
  .library { padding:0;margin:0.5rem 0;list-style:none; }
  .library li { display:flex;justify-content:space-between;align-items:center;gap:0.7rem;border-bottom:1px solid #ffffff10;padding:0.6rem 0; }
  .library li strong { display:block;font-size:0.78rem;overflow-wrap:anywhere; }
  .library li span { display:block;margin-top:0.2rem;font-size:0.65rem;color:#94a3b8; }
  .remove { color:#fda4af;background:transparent; }
  footer { margin-top:1rem;font-size:0.6rem;line-height:1.5; }
  @media(max-width:600px){.editor{grid-template-columns:1fr}.model-import{padding:1rem}.actions{gap:0.2rem}button{padding:0.45rem 0.6rem}}
</style>
