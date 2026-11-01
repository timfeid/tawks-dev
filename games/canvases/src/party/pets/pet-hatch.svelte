<script lang="ts">
  import { onMount } from 'svelte';
  import * as THREE from 'three';
  import { toon } from '../toon';
  import { PET_CATALOG, type Pet } from './catalog';
  import { buildPet } from './pet-model';

  /** A 3D egg that wobbles, cracks and pops open on `pet`. `still` shows the pet straight away; `hatchNow` turning true skips the wait. `onhatch` fires as the shell breaks. */
  let { pet, still = false, hatchNow = false, onhatch }: { pet: Pet; still?: boolean; hatchNow?: boolean; onhatch?: () => void } = $props();

  const WOBBLE = 1.9;
  const EGG = 0.62;
  let canvas: HTMLCanvasElement;
  let cracked = -1;
  let begin: () => void = () => {};

  $effect(() => {
    if (hatchNow) begin();
  });

  /** The egg's outline from its base (t = 0) to its tip (t = 1), wider below like a real egg. */
  function shell(from: number, to: number, ragged: boolean) {
    const points: THREE.Vector2[] = [];
    const steps = 24;
    for (let i = 0; i <= steps; i++) {
      const t = from + ((to - from) * i) / steps;
      const radius = 0.42 * Math.sqrt(Math.max(Math.sin(Math.PI * t), 0)) * (1.08 - 0.22 * t);
      points.push(new THREE.Vector2(Math.max(radius, 0.0001), t * EGG * 1.6));
    }
    const geometry = new THREE.LatheGeometry(points, 28);
    if (ragged) {
      // Zigzag the rim of each half so the break reads as a crack, not a cut.
      const position = geometry.attributes.position;
      const rim = (to === 1 ? from : to) * EGG * 1.6;
      for (let i = 0; i < position.count; i++) {
        if (Math.abs(position.getY(i) - rim) > 1e-4) continue;
        const angle = Math.atan2(position.getZ(i), position.getX(i));
        position.setY(i, rim + Math.abs(((angle * 7) / Math.PI) % 2 - 1) * 0.09 - 0.045);
      }
      geometry.computeVertexNormals();
    }
    return geometry;
  }

  onMount(() => {
    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.2;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 20);
    camera.position.set(0, 0.8, 3.1);
    camera.lookAt(0, 0.5, 0);
    scene.add(new THREE.HemisphereLight('#ffffff', '#64748b', 2.4));
    const light = new THREE.DirectionalLight('#fff4db', 2.8);
    light.position.set(-3, 5, 4);
    scene.add(light);

    const disposables: { dispose: () => void }[] = [];
    const keep = <T extends { dispose: () => void }>(thing: T) => (disposables.push(thing), thing);
    const shellMaterial = keep(toon('#fff8ec'));
    shellMaterial.side = THREE.DoubleSide;
    const egg = new THREE.Group();
    const split = 0.56;
    const lower = new THREE.Mesh(keep(shell(0, split, true)), shellMaterial);
    const upper = new THREE.Mesh(keep(shell(split, 1, true)), shellMaterial);
    egg.add(lower, upper);
    const spotGeometry = keep(new THREE.SphereGeometry(1, 12, 8));
    const spots = [[0.3, 0.3, 0.1], [1.4, 0.68, 0.08], [2.4, 0.4, 0.11], [3.4, 0.75, 0.07], [4.3, 0.28, 0.1], [5.4, 0.62, 0.09], [0.9, 0.86, 0.06], [3.0, 0.15, 0.08]];
    spots.forEach(([angle, t, size], index) => {
      const radius = 0.42 * Math.sqrt(Math.sin(Math.PI * t)) * (1.08 - 0.22 * t);
      const spot = new THREE.Mesh(spotGeometry, keep(toon(PET_CATALOG[index % PET_CATALOG.length].body)));
      spot.position.set(Math.cos(angle) * radius, t * EGG * 1.6, Math.sin(angle) * radius);
      spot.scale.set(size, size, size * 0.35);
      spot.lookAt(spot.position.clone().multiplyScalar(2).setY(spot.position.y));
      (t > split ? upper : lower).add(spot);
    });
    // The crack line runs along the ragged rim and spreads round the shell before it breaks.
    const crackPoints = Array.from({ length: 57 }, (_, i) => {
      const angle = (i / 56) * Math.PI * 2;
      const radius = 0.42 * Math.sqrt(Math.sin(Math.PI * split)) * (1.08 - 0.22 * split) + 0.004;
      const y = split * EGG * 1.6 + Math.abs(((angle * 7) / Math.PI) % 2 - 1) * 0.09 - 0.045;
      return new THREE.Vector3(Math.cos(angle) * radius, y, Math.sin(angle) * radius);
    });
    const crackGeometry = keep(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(crackPoints, true, 'catmullrom', 0), 224, 0.014, 4, true));
    const crackLine = new THREE.Mesh(crackGeometry, keep(new THREE.MeshBasicMaterial({ color: '#2a2233' })));
    const crackLength = crackGeometry.index?.count ?? 0;
    crackGeometry.setDrawRange(0, 0);
    egg.add(crackLine);
    scene.add(egg);

    const shadow = new THREE.Mesh(keep(new THREE.CircleGeometry(0.42, 32)), keep(new THREE.MeshBasicMaterial({ color: '#000000', transparent: true, opacity: 0.25 })));
    shadow.rotation.x = -Math.PI / 2;
    shadow.position.y = 0.002;
    scene.add(shadow);

    const confettiGeometry = keep(new THREE.BoxGeometry(0.045, 0.045, 0.012));
    const confetti = Array.from({ length: 18 }, (_, index) => {
      const piece = new THREE.Mesh(confettiGeometry, keep(new THREE.MeshBasicMaterial({ color: index % 2 ? pet.body : pet.accent, transparent: true })));
      const angle = (index / 18) * Math.PI * 2;
      piece.userData.velocity = new THREE.Vector3(Math.cos(angle) * 0.9, 1.6 + (index % 3) * 0.4, Math.sin(angle) * 0.5);
      piece.visible = false;
      scene.add(piece);
      return piece;
    });

    const creature = buildPet(pet.id, 0.72);
    scene.add(creature.group);
    creature.group.visible = false;
    void creature.ready.then(() => (stillDrawn = false));

    const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    const quiet = still || reduced;
    const start = performance.now();
    let stillDrawn = false;
    const crack = (now: number) => {
      if (cracked >= 0) return;
      cracked = now;
      onhatch?.();
    };
    begin = () => crack(performance.now());
    if (quiet) crack(start - 10_000);

    const fit = () => {
      const { width, height } = canvas.getBoundingClientRect();
      renderer.setSize(width, height, false);
      camera.aspect = width / Math.max(height, 1);
      camera.updateProjectionMatrix();
      stillDrawn = false;
    };
    const observer = new ResizeObserver(fit);
    observer.observe(canvas);
    fit();

    let frame = 0;
    const draw = (now: number) => {
      frame = requestAnimationFrame(draw);
      if (quiet && stillDrawn) return;
      const elapsed = (now - start) / 1000;
      if (cracked < 0 && elapsed > WOBBLE) crack(now);
      if (cracked < 0) {
        // Rocks harder as it gets closer to hatching.
        const urge = Math.min(elapsed / WOBBLE, 1);
        egg.rotation.z = Math.sin(elapsed * (9 + urge * 9)) * (0.05 + urge * 0.16);
        egg.position.y = Math.max(0, Math.sin(elapsed * 18)) * urge * 0.03;
        const spread = Math.min(Math.max((urge - 0.35) / 0.6, 0), 1);
        crackGeometry.setDrawRange(0, Math.floor((crackLength * spread) / 3) * 3);
      } else {
        const since = quiet ? 10 : (now - cracked) / 1000;
        egg.rotation.z = 0;
        crackLine.visible = false;
        upper.position.set(since * 0.5, since * 2.4 - since * since * 4, 0);
        upper.rotation.z = -since * 5;
        lower.scale.setScalar(Math.max(0, 1 - since * 1.8));
        upper.visible = since < 0.9;
        lower.visible = since < 0.55;
        const pop = Math.min(since / 0.45, 1);
        // Overshoots then settles, like a jump out of the shell.
        const grow = 1 + 2.4 * (pop - 1) ** 3 + 1.4 * (pop - 1) ** 2;
        creature.group.visible = true;
        creature.group.scale.setScalar(Math.max(grow, 0.01));
        creature.group.position.y = Math.max(0, Math.sin(Math.min(since / 0.6, 1) * Math.PI)) * 0.25;
        creature.group.rotation.y = quiet ? -0.3 : Math.sin(since * 0.8) * 0.35;
        creature.animate(quiet ? 0 : since, since > 0.3 && since < 2.2);
        confetti.forEach((piece) => {
          const life = since;
          piece.visible = !quiet && life < 1.4;
          if (!piece.visible) return;
          const velocity = piece.userData.velocity as THREE.Vector3;
          piece.position.set(velocity.x * life, 0.5 + velocity.y * life - 2.6 * life * life, velocity.z * life);
          piece.rotation.set(life * 9, life * 7, 0);
          (piece.material as THREE.MeshBasicMaterial).opacity = Math.max(0, 1 - life / 1.4);
        });
      }
      renderer.render(scene, camera);
      stillDrawn = true;
    };
    frame = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      creature.dispose();
      disposables.forEach((thing) => thing.dispose());
      renderer.dispose();
      renderer.forceContextLoss();
    };
  });
</script>

<canvas bind:this={canvas} class="block size-full" aria-hidden="true"></canvas>
