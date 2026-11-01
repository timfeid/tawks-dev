<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import * as THREE from 'three';
  import { buildCharacter, type CharacterRig, type Pose } from './characters';
  import { buildBag, hangBag } from './bags';
  import { buildController, holdController, type HeldController } from './controller';
  import { characterAppearanceKey } from './appearance';
  import type { CharacterDef } from './protocol';
  import {bodyWidth} from './costumes/body-build';
  import {focusBounds, type FocusSlot} from './costumes/framing';
  import type {FaceReaction} from './reactions/face';
  import {buildPet, type PetRig} from './pets/pet-model';
  import {shownPet, shownPetKey} from './pets/store';

  /** `compact` drops the caption and fills its box, for a profile card; `stage` backs a compact camera off so the whole platform shows; `paused` holds a still frame instead of animating; `focus` frames a compact view on where an item is worn; `playing` puts a controller in their hands; `bag` false leaves the carried bag off; `platform` false drops the round base for a stand drawn around it; `reaction` holds that face; `hoverReaction` makes the look's own reaction when the pointer comes over, and `reacting` when it turns true; `pet` is the id of the pet they hatched, standing at their feet unless their costume puts out another, and dressed as the costume says; `focus` of `pet` frames it. */
  let { character, others = [], compact = false, pose = 'idle', stage = false, paused = false, focus = null, playing = false, bag = true, platform = true, reaction = null, hoverReaction = false, reacting = false, pet = null }: { character: CharacterDef | null; others?: {name:string;character:CharacterDef}[]; compact?: boolean; pose?: Pose; stage?: boolean; paused?: boolean; focus?: FocusSlot | null; playing?: boolean; bag?: boolean; platform?: boolean; reaction?: FaceReaction | null; hoverReaction?: boolean; reacting?: boolean; pet?: string | null } = $props();
  let platformMeshes: THREE.Object3D[] = [];
  $effect(() => {
    const show = platform;
    platformMeshes.forEach(mesh => mesh.visible = show);
    stillDrawn = false;
  });
  let controller: HeldController | null = null;
  let visible = true;
  let canvas: HTMLCanvasElement;
  let ready = $state(false);
  let scene: THREE.Scene;
  let rig: CharacterRig | null = null;
  let peers: CharacterRig[] = [];
  let heroKey = '';
  let heroBuild: number | null = null;
  let unhang: (() => void) | null = null;
  let peerRigs = new Map<string, { appearance: string; rig: CharacterRig }>();
  let refit: (() => void) | null = null;
  let companion: PetRig | null = null;
  let companionKey = '';
  let stillDrawn = false;
  // A close-up of a worn item leaves the bag off; at the side it would cover legs and feet.
  const carry = (group: THREE.Group, def: CharacterDef) => focus || !bag ? null : hangBag(group, buildBag(def.costume?.bag), 1.1);

  $effect(() => {
    if (!ready) return;
    if (!character) {
      rig?.dispose(); rig = null; heroKey = ''; unhang?.(); unhang = null;
      peerRigs.forEach(peer => peer.rig.dispose()); peerRigs.clear(); peers = [];
      refit?.();
      return;
    }
    // Dragging the build slider changes transforms, not the loaded meshes.
    const appearance = characterAppearanceKey({...character,costume:{...character.costume,color:character.costume?.color??character.id,body_width:1}});
    if (appearance !== heroKey || !rig) {
      rig?.dispose();
      unhang?.();
      rig = buildCharacter(character, {reactOnCheer: false});
      // With a couple of things in it, so a bought bag shows what it is.
      unhang = carry(rig.group, character);
      scene.add(rig.group);
      void rig.ready?.then(() => refit?.());
      heroKey = appearance;
      heroBuild = null;
    }
    const width=bodyWidth(character.costume?.body_width);
    if(heroBuild!==width){
      rig.setBodyWidth?.(width);
      if(heroBuild!==null){unhang?.();unhang=carry(rig.group,character);}
      heroBuild=width;
    }
    const party=others.slice(0,7);
    const span=party.length*0.65;
    // Compact, shift over so the bag at their side is in frame too.
    rig.group.position.x=compact?(bag?0.3:0):-span/2;
    const retained = new Map<string, { appearance: string; rig: CharacterRig }>();
    const occurrences = new Map<string, number>();
    peers=party.map((peer,index)=>{
      const occurrence = occurrences.get(peer.name) ?? 0;
      occurrences.set(peer.name, occurrence + 1);
      const identity = JSON.stringify([peer.name, occurrence]);
      const appearance = characterAppearanceKey(peer.character);
      const previous = peerRigs.get(identity);
      const peerRig = previous?.appearance === appearance ? previous.rig : buildCharacter(peer.character, {reactOnCheer: false});
      if (previous && previous.rig !== peerRig) previous.rig.dispose();
      retained.set(identity, { appearance, rig: peerRig });
      peerRig.group.scale.setScalar(0.7);
      peerRig.group.position.set(-span/2+0.85+index*0.65,0,-0.15);
      if (peerRig.group.parent !== scene) scene.add(peerRig.group);
      return peerRig;
    });
    peerRigs.forEach((peer, identity) => { if (!retained.has(identity)) peer.rig.dispose(); });
    peerRigs = retained;
    refit?.();
  });

  // A close-up of a worn item has no room for it.
  $effect(() => {
    const wanted = ready && character && (!focus || focus === 'pet') ? shownPet(pet, character.costume?.pet) : null;
    const key = shownPetKey(wanted);
    untrack(() => {
      if (key === companionKey) return;
      companion?.dispose(); companion = null; companionKey = key;
      if (wanted) {
        companion = buildPet(wanted.id, undefined, wanted);
        // At the platform's front edge on the side away from the bag, turned partly to its owner.
        companion.group.rotation.y = -0.45;
        scene.add(companion.group);
        void companion.ready.then(() => refit?.());
      }
      stillDrawn = false;
      refit?.();
    });
  });

  $effect(() => {
    void pose;
    void paused;
    stillDrawn = false;
  });

  $effect(() => {
    if (!ready || !character) return;
    void heroKey;
    rig?.react?.(reaction, true);
    stillDrawn = false;
  });
  let still = false;
  function react(on: boolean) {
    if (reaction || !rig?.react) return;
    // A still frame has no time to play it out in, so it holds the face while the pointer stays.
    if (on) rig.react(undefined, still);
    else if (still) rig.react(null);
    stillDrawn = false;
  }
  $effect(() => {
    const on = reacting;
    if (ready) untrack(() => react(on));
  });

  // The controller rides on the hero rig, so it follows a rebuilt rig too.
  $effect(() => {
    if (!ready || !character) return;
    void heroKey;
    if (!playing || !rig) {
      controller?.dispose(); controller = null;
      stillDrawn = false;
      return;
    }
    if (!controller) controller = buildController();
    if (controller.group.parent !== rig.group) holdController(rig.group, controller);
    stillDrawn = false;
  });

  onMount(() => {
    scene = new THREE.Scene();
    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.2;
    const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 20);
    camera.position.set(0, 1.3, 4.4);
    camera.lookAt(0, 1, 0);
    scene.add(new THREE.HemisphereLight('#ffffff', '#64748b', 2.4));
    const light = new THREE.DirectionalLight('#fff4db', 2.8);
    light.position.set(-3, 5, 4);
    scene.add(light);
    const base = new THREE.Mesh(new THREE.CylinderGeometry(0.8, 0.9, 0.12, 48), new THREE.MeshStandardMaterial({color:'#182539',roughness:0.7}));
    base.position.y = -0.07;
    scene.add(base);
    const ring = new THREE.Mesh(new THREE.TorusGeometry(0.82, 0.025, 6, 64), new THREE.MeshBasicMaterial({color:'#fcd34d'}));
    ring.rotation.x = Math.PI / 2;
    ring.position.y = 0.005;
    scene.add(ring);
    platformMeshes = [base, ring];
    platformMeshes.forEach(mesh => mesh.visible = platform);
    const fit = () => {
      const {width,height} = canvas.getBoundingClientRect();
      renderer.setSize(width, height, false);
      camera.aspect = width / Math.max(height, 1);
      const tangent = Math.tan(THREE.MathUtils.degToRad(35 / 2));
      if (compact && rig) {
        // Fit the head, body, and carried bag together, including narrow editor columns.
        if (companion) companion.group.position.set(rig.group.position.x + 0.62, 0, 0.42);
        const petArea = focus === 'pet' && companion ? new THREE.Box3().setFromObject(companion.group, true) : null;
        const area = petArea && !petArea.isEmpty() ? petArea.expandByScalar(0.2) : focus && focus !== 'pet' ? focusBounds(rig.group, focus) : null;
        const bounds = area ?? new THREE.Box3().setFromObject(rig.group);
        if (companion && !area) bounds.union(new THREE.Box3().setFromObject(companion.group, true));
        if (stage && !area) bounds.union(new THREE.Box3().setFromObject(base));
        const size = bounds.getSize(new THREE.Vector3());
        const center = bounds.getCenter(new THREE.Vector3());
        const distance = Math.max(size.y, size.x / camera.aspect) / (2 * tangent) * (area ? 1.3 : 1.12) + size.z / 2;
        camera.position.set(center.x, center.y + (area ? 0.04 : 0.12), Math.max(area ? 1.4 : stage ? 4.3 : 3.5, distance));
        camera.lookAt(center.x, center.y, 0);
      } else {
        camera.position.z = Math.max(4.4, (2 + peers.length * 0.65) / (2 * tangent * camera.aspect));
      }
      camera.updateProjectionMatrix();
      stillDrawn = false;
    };
    refit=fit;
    const observer = new ResizeObserver(fit);
    observer.observe(canvas);
    // Nothing to draw while scrolled away or in a closed panel.
    const seen = new IntersectionObserver(([entry]) => { visible = entry?.isIntersecting ?? true; stillDrawn = false; });
    seen.observe(canvas);
    fit();
    ready = true;
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let frame = 0;
    const draw = (now: number) => {
      frame = requestAnimationFrame(draw);
      if ((paused || !visible) && stillDrawn) return;
      still = reduced || paused;
      const shown: Pose = playing && pose === 'idle' ? 'play' : pose;
      if (rig) {
        // Turned to their game while playing, rather than slowly looking around.
        rig.group.rotation.y = shown === 'play' ? 0.15 : still ? 0.15 : 0.15 + Math.sin(now * 0.0006) * 0.22;
        rig.animate(shown, still ? 0 : now / 1000, 0);
        controller?.update(still ? 0 : now / 1000);
        peers.forEach((peer,index)=>{peer.group.rotation.y=0.15;peer.animate('idle',still?0:now/1000+index*0.2,0)});
      }
      if (companion && rig) {
        companion.group.position.set(rig.group.position.x + 0.62, 0, 0.42);
        companion.animate(still ? 0 : now / 1000, shown === 'cheer');
      }
      renderer.render(scene, camera);
      stillDrawn = true;
    };
    frame = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      seen.disconnect();
      controller?.dispose(); controller = null;
      refit=null;
      companion?.dispose(); companion = null; companionKey = '';
      rig?.dispose(); rig = null; heroKey = ''; unhang?.(); unhang = null;
      peerRigs.forEach(peer => peer.rig.dispose()); peerRigs.clear(); peers = [];
      base.geometry.dispose();
      base.material.dispose();
      ring.geometry.dispose();
      ring.material.dispose();
      renderer.dispose();
      // Cards open and close all day; hand the WebGL context back now, not whenever it is collected.
      renderer.forceContextLoss();
    };
  });
</script>

<figure class:compact style="--character-color:{character?.body ?? '#fcd34d'}" aria-label="{compact ? 'Tawkathon character' : 'Your character'}: {character?.name ?? 'Choose a character'}" onpointerenter={() => hoverReaction && react(true)} onpointerleave={() => hoverReaction && react(false)}>
  <canvas bind:this={canvas} aria-hidden="true"></canvas>
  {#if !compact}<figcaption><span>{others.length ? `YOU + ${others.length} IN YOUR PARTY` : 'YOUR LOOK'}</span><strong>{character?.name ?? 'Pick your character'}</strong></figcaption>{/if}
</figure>

<style>
  figure { position: relative; min-width: 0; margin: 0; background: radial-gradient(ellipse at 50% 60%, color-mix(in srgb, var(--character-color) 18%, transparent), transparent 65%); }
  canvas { display: block; width: 100%; height: 15rem; }
  figcaption { position: absolute; right: 0; bottom: 0.2rem; left: 0; text-align: center; text-shadow: 0 2px 6px rgb(0 0 0 / 0.5); }
  figcaption span { display: block; color: rgb(255 255 255 / 0.45); font-size: 0.5rem; font-weight: 800; letter-spacing: 0.2em; }
  figcaption strong { color: var(--character-color); font-size: 1.5rem; font-weight: 950; font-style: italic; text-transform: uppercase; }
  figure.compact { height: 100%; }
  figure.compact canvas { height: 100%; }
  @media (max-width: 640px) { figure:not(.compact) canvas { height: 10rem; } figcaption { bottom: 0; } figcaption strong { font-size: 1.1rem; } }
</style>
