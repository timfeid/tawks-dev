<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { parseCustomModel } from './custom-model-rig';
  import type { CharacterRig } from '../characters';
  import type { ModelPose, ModelReport } from './types';

  let { data, report, pose = 'idle' }: {data: ArrayBuffer; report: ModelReport; pose?: ModelPose} = $props();
  let canvas: HTMLCanvasElement;
  let scene: THREE.Scene;
  let mounted = $state(false);
  let loading = $state(true);
  let error = $state('');
  let current: CharacterRig | null = null;
  let rotation = 0.2;
  let dragX: number | null = null;

  $effect(() => {
    if (!mounted) return;
    const bytes = data;
    const metadata = report;
    let disposed = false;
    let rig: CharacterRig | null = null;
    loading = true;
    error = '';
    void parseCustomModel(bytes, metadata).then((loaded) => {
      if (disposed) { loaded.dispose(); return; }
      rig = loaded;
      current = loaded;
      scene.add(loaded.group);
      loading = false;
    }).catch((reason) => { if (!disposed) { loading = false; error = reason instanceof Error ? reason.message : 'This model could not be previewed.'; } });
    return () => { disposed = true; rig?.dispose(); if (current === rig) current = null; };
  });
  onMount(() => {
    scene = new THREE.Scene();
    let renderer: THREE.WebGLRenderer;
    try { renderer = new THREE.WebGLRenderer({canvas, antialias: true, alpha: true}); }
    catch { error = 'Your browser could not preview 3D models.'; loading = false; return; }
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.2;
    const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 20);
    camera.position.set(0, 1.2, 4.3);
    camera.lookAt(0, 0.95, 0);
    scene.add(new THREE.HemisphereLight('#fff7ef', '#486177', 2.6));
    const sun = new THREE.DirectionalLight('#ffffff', 2.2);
    sun.position.set(3, 5, 4);
    scene.add(sun);
    const base = new THREE.Mesh(new THREE.CylinderGeometry(0.85, 0.95, 0.08, 48), new THREE.MeshStandardMaterial({color:'#1d3347',roughness:0.8}));
    base.position.y = -0.05;
    scene.add(base);
    const fit = () => {
      const rect = canvas.getBoundingClientRect();
      renderer.setSize(rect.width, rect.height, false);
      camera.aspect = rect.width / Math.max(rect.height, 1);
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(fit);
    observer.observe(canvas);
    fit();
    mounted = true;
    let frame = 0;
    const draw = (time: number) => {
      if (current) { current.group.rotation.y = rotation; current.animate(pose, time/1000, pose === 'ride' ? 4 : 0); }
      renderer.render(scene, camera);
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); base.geometry.dispose(); base.material.dispose(); renderer.dispose(); renderer.forceContextLoss(); };
  });
</script>

<div class="model-preview" role="img" aria-label="Uploaded model preview. Drag to turn the model.">
  <canvas
    bind:this={canvas}
    onpointerdown={(event) => {dragX = event.clientX; canvas.setPointerCapture(event.pointerId);}}
    onpointermove={(event) => {if (dragX !== null) {rotation += (event.clientX-dragX)*0.01; dragX = event.clientX;}}}
    onpointerup={() => (dragX = null)}
    onpointercancel={() => (dragX = null)}
    aria-hidden="true"
  ></canvas>
  {#if loading}<div class="preview-message" role="status">Loading preview…</div>{/if}
  {#if error}<div class="preview-message error" role="alert">{error}</div>{/if}
  {#if !loading && !error}<span class="turn-hint">DRAG TO TURN</span>{/if}
</div>

<style>
  .model-preview { position: relative; min-height: 16rem; background: radial-gradient(ellipse at center, #18394780, transparent 70%); }
  canvas { display: block; width: 100%; height: 18rem; touch-action: none; cursor: grab; }
  canvas:active { cursor: grabbing; }
  .preview-message { position: absolute; inset: 0; display: grid; place-content: center; padding: 1rem; color: #cbd5e1; text-align: center; font-size: 0.8rem; pointer-events: none; }
  .preview-message.error { color: #fda4af; background: #06131bd9; }
  .turn-hint { position: absolute; right: 0; bottom: 0.3rem; left: 0; color: #ffffff60; font-size: 0.55rem; font-weight: 800; letter-spacing: 0.18em; text-align: center; pointer-events: none; }
</style>
