import * as THREE from 'three';
import { Reflector } from 'three/examples/jsm/objects/Reflector.js';
import { createRoomEnvironment, applyRingUv } from '../environment';

const TABLE_REFLECTOR_SHADER = {
  name: 'TableReflectorShader',
  uniforms: {
    color: { value: null },
    tDiffuse: { value: null },
    textureMatrix: { value: null },
    reflectivity: { value: 0.14 },
  },
  vertexShader: /* glsl */ `
    uniform mat4 textureMatrix;
    varying vec4 vUv;

    #include <common>
    #include <logdepthbuf_pars_vertex>

    void main() {
      vUv = textureMatrix * vec4(position, 1.0);
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);

      #include <logdepthbuf_vertex>
    }
  `,
  fragmentShader: /* glsl */ `
    uniform vec3 color;
    uniform sampler2D tDiffuse;
    uniform float reflectivity;
    varying vec4 vUv;

    #include <logdepthbuf_pars_fragment>

    void main() {
      #include <logdepthbuf_fragment>

      vec4 reflected = texture2DProj(tDiffuse, vUv);
      vec3 tinted = reflected.rgb * color;
      gl_FragColor = vec4(tinted * reflectivity, reflectivity);

      #include <tonemapping_fragment>
      #include <colorspace_fragment>
    }
  `,
};

export class TableManager {
  private scene: THREE.Scene;

  private tableRadius = 10;
  private tableGroup = new THREE.Group();

  private pingGroup = new THREE.Group();

  private tableMirror!: Reflector;

  private edgeGlow!: THREE.Mesh;

  private halo!: THREE.Mesh;

  private tableSurface: THREE.Mesh | null = null;

  private baseMesh: THREE.Mesh | null = null;

  private underTableLight: THREE.PointLight | null = null;

  private glowColor = new THREE.Color(0xf59e0b);

  private reflectionStrength = 0.14;

  private activePings: {
    group: THREE.Group;
    ring: THREE.LineSegments;
    particles: THREE.Points;
    age: number;
    maxAge: number;
  }[] = [];

  private activeProjectiles: {
    mesh: THREE.Mesh;
    trail: THREE.Line;
    start: THREE.Vector3;
    end: THREE.Vector3;
    age: number;
    maxAge: number;
    color: number;
  }[] = [];

  private activeFloatingTexts: {
    sprite: THREE.Sprite;
    texture: THREE.CanvasTexture;
    startY: number;
    riseDistance: number;
    age: number;
    maxAge: number;
  }[] = [];

  private sparkTexture: THREE.Texture;

  private time = 0;

  private currentPlayerCount = 2;
  private appliedScale = 1.0;
  private scaleAnimating = false;

  constructor(scene: THREE.Scene) {
    this.scene = scene;

    this.sparkTexture = this.createSparkTexture();

    this.initEnvironment();

    this.initTable();

    this.scene.add(this.tableGroup);
    this.scene.add(this.pingGroup);

    // Apply the initial scale synchronously so the first view has the
    // mesh and the zones agreeing on table size.
    this.appliedScale = this.computeScale();
    this.tableGroup.scale.set(this.appliedScale, 1.0, this.appliedScale);
  }

  public setPlayerCount(count: number) {
    // Ignore transient drops to < 2 (e.g. during workflow state
    // transitions or a player briefly missing from the list). The table
    // should only shrink/grow when the real party size changes.
    if (count < 2) return;
    if (this.currentPlayerCount === count) return;
    this.currentPlayerCount = count;

    const targetScale = this.computeScale();
    // Update the shared "applied scale" immediately so zones that read
    // getTableScale() on the same frame (inner ring, cards, mana hexes,
    // player seats) match the final mesh size the animation is lerping
    // toward. Without this, zones can render at a different radius than
    // the mesh for a few frames, which was the "table but cards float off
    // the table" bug.
    this.appliedScale = targetScale;

    if (this.scaleAnimating) return;
    this.scaleAnimating = true;
    let currentScale = this.tableGroup.scale.x;
    const animate = () => {
      currentScale += (this.appliedScale - currentScale) * 0.1;
      this.tableGroup.scale.set(currentScale, 1.0, currentScale);
      if (Math.abs(currentScale - this.appliedScale) > 0.005) {
        requestAnimationFrame(animate);
      } else {
        this.tableGroup.scale.set(
          this.appliedScale,
          1.0,
          this.appliedScale,
        );
        this.scaleAnimating = false;
      }
    };
    animate();
  }

  private computeScale(): number {
    // 2p ≈ 0.7 (smaller, more intimate table). 3p ≈ 1.05, 4p ≈ 1.4, 6p ≈ 1.6.
    return Math.min(1.6, Math.max(0.7, 0.35 * this.currentPlayerCount));
  }

  public getTableScale(): number {
    // Report the *applied* scale (not just the computed one) so zones
    // and the camera always agree with the mesh about table size — this
    // is what keeps cards / mana / players / inner ring lined up with
    // the outer glow ring.
    return this.appliedScale;
  }

  /**
   * THE single source of truth for table radius everywhere in the
   * renderer. The table mesh is built with ring geometry at a fixed
   * radius of `tableRadius` (10) and `tableGroup.scale` multiplies
   * that visually; this method returns the world-space radius of the
   * outer glow ring so zones / camera / layout can line up exactly
   * with what's drawn.
   */
  public getEffectiveRadius(): number {
    return this.tableRadius * this.appliedScale;
  }

  public getBaseRadius(): number {
    return this.tableRadius;
  }

  public setReflectionOpacity(opacity: number) {
    if (this.tableMirror) {
      this.tableMirror.visible = opacity > 0;
      const material = this.tableMirror.material as THREE.ShaderMaterial;
      const uniform = material.uniforms?.reflectivity;

      if (uniform) {
        uniform.value = this.reflectionStrength * THREE.MathUtils.clamp(opacity, 0, 1);
      }
    }
  }

  private createGlowTexture(peaked = false) {
    const canvas = document.createElement('canvas');

    canvas.width = 1;

    canvas.height = 128;

    const ctx = canvas.getContext('2d')!;

    const grad = ctx.createLinearGradient(0, 0, 0, 128);

    if (peaked) {
      grad.addColorStop(0, 'rgba(255, 255, 255, 0)');

      grad.addColorStop(0.5, 'rgba(255, 255, 255, 1)');

      grad.addColorStop(1, 'rgba(255, 255, 255, 0)');
    } else {
      grad.addColorStop(0, 'rgba(255, 255, 255, 1)');

      grad.addColorStop(1, 'rgba(255, 255, 255, 0)');
    }

    ctx.fillStyle = grad;

    ctx.fillRect(0, 0, 1, 128);

    const tex = new THREE.CanvasTexture(canvas);

    return tex;
  }

  private createSparkTexture() {
    const canvas = document.createElement('canvas');

    canvas.width = 64;

    canvas.height = 64;

    const ctx = canvas.getContext('2d');

    if (ctx) {
      const grad = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);

      grad.addColorStop(0, 'rgba(255, 255, 255, 1)');

      grad.addColorStop(0.2, 'rgba(245, 158, 11, 0.8)');

      grad.addColorStop(0.5, 'rgba(245, 158, 11, 0.2)');

      grad.addColorStop(1, 'rgba(245, 158, 11, 0)');

      ctx.fillStyle = grad;

      ctx.fillRect(0, 0, 64, 64);
    }

    const tex = new THREE.CanvasTexture(canvas);

    return tex;
  }

  private initEnvironment() {
    createRoomEnvironment(this.scene);

    // Tactical Ambient
    this.scene.add(new THREE.AmbientLight(0x064e3b, 0.8));
    
    // Central Holo Projector Light
    const tableLight = new THREE.PointLight(0x10b981, 1.2, 40);
    tableLight.position.set(0, -1, 0); // Projecting up from the center
    this.scene.add(tableLight);

    const overheadLight = new THREE.SpotLight(0x10b981, 1.5, 60, 0.8, 0.5, 1);
    overheadLight.position.set(0, 20, 0);
    overheadLight.target.position.set(0, 0, 0);
    this.scene.add(overheadLight);
    this.scene.add(overheadLight.target);
  }

  private initTable() {
    // 1. Holographic Glass Surface with Image
    const loader = new THREE.TextureLoader();
    const tableTexture = loader.load(new URL('../assets/table-surface.jpg', import.meta.url).href);
    tableTexture.colorSpace = THREE.SRGBColorSpace;
    tableTexture.wrapS = THREE.ClampToEdgeWrapping;
    tableTexture.wrapT = THREE.ClampToEdgeWrapping;
    tableTexture.repeat.set(0.82, 0.82);
    tableTexture.center.set(0.5, 0.5);

    // Create a radial gradient alpha map to fade out the center
    const alphaCanvas = document.createElement('canvas');
    alphaCanvas.width = 512;
    alphaCanvas.height = 512;
    const alphaCtx = alphaCanvas.getContext('2d')!;
    const alphaGrad = alphaCtx.createRadialGradient(256, 256, 0, 256, 256, 256);
    alphaGrad.addColorStop(0, 'rgba(0, 0, 0, 1)'); // Center is fully transparent
    alphaGrad.addColorStop(0.5, 'rgba(0, 0, 0, 1)'); // Large black center
    alphaGrad.addColorStop(1, 'rgba(255, 255, 255, 1)'); // Fade to opaque edge
    alphaCtx.fillStyle = alphaGrad;
    alphaCtx.fillRect(0, 0, 512, 512);
    const alphaMap = new THREE.CanvasTexture(alphaCanvas);

    const surfaceMat = new THREE.MeshBasicMaterial({
      map: tableTexture,
      alphaMap: alphaMap,
      color: new THREE.Color(0xffe0cc).multiplyScalar(0.6), // Tinted warm
      transparent: true,
      opacity: 0.8,
      depthWrite: false,
    });

    // The hex-textured surface IS the table — sized EXACTLY at
    // tableRadius so it lines up flush with the edge glow ring (drawn
    // at tableRadius - 0.01 → tableRadius - 0.03). No dead band.
    this.tableSurface = new THREE.Mesh(
      new THREE.CircleGeometry(this.tableRadius, 64),
      surfaceMat,
    );
    this.tableSurface.position.y = 0.0;
    this.tableSurface.rotation.x = -Math.PI / 2;
    this.tableSurface.renderOrder = 5;
    this.tableGroup.add(this.tableSurface);

    // 2. Dark Base underneath — matches the surface so nothing dark peeks
    // out past the hex texture.
    const baseMat = new THREE.MeshBasicMaterial({ color: 0x020617 });
    const baseMesh = new THREE.Mesh(new THREE.CircleGeometry(this.tableRadius, 64), baseMat);
    baseMesh.position.y = -0.01;
    baseMesh.rotation.x = -Math.PI / 2;
    this.tableGroup.add(baseMesh);

    // 3. Edge Glow (Tactical Green Ring)
    const edgeGlowGeometry = new THREE.RingGeometry(
      this.tableRadius - 0.03,
      this.tableRadius - 0.01,
      128,
    );
    applyRingUv(
      edgeGlowGeometry,
      this.tableRadius - 0.03,
      this.tableRadius - 0.01,
    );
    this.edgeGlow = new THREE.Mesh(
      edgeGlowGeometry,
      new THREE.MeshBasicMaterial({
        color: new THREE.Color(0x10b981).multiplyScalar(2.0),
        alphaMap: this.createGlowTexture(true),
        transparent: true,
        opacity: 0.8,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        side: THREE.DoubleSide,
      }),
    );
    this.edgeGlow.rotation.x = -Math.PI / 2;
    this.edgeGlow.position.y = 0.015;
    this.edgeGlow.renderOrder = 10;
    this.tableGroup.add(this.edgeGlow);

    // 4. Outer Halo (Scanning glow)
    const haloGeometry = new THREE.RingGeometry(
      this.tableRadius - 0.01,
      this.tableRadius + 0.2,
      128,
    );
    applyRingUv(haloGeometry, this.tableRadius - 0.01, this.tableRadius + 0.2);
    this.halo = new THREE.Mesh(
      haloGeometry,
      new THREE.MeshBasicMaterial({
        color: new THREE.Color(0x059669),
        alphaMap: this.createGlowTexture(false),
        transparent: true,
        opacity: 0.3,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        side: THREE.DoubleSide,
      }),
    );
    this.halo.rotation.x = -Math.PI / 2;
    this.halo.position.y = 0.01;
    this.tableGroup.add(this.halo);

    // 5. Under-table Glow Light
    this.glowColor.setHex(0x10b981);
    this.underTableLight = new THREE.PointLight(this.glowColor, 4.0, 20);
    this.underTableLight.position.set(0, -0.5, 0);
    this.tableGroup.add(this.underTableLight);

    // Initial state: Off
    this.setGlow(false);
  }

  public setGlow(isTurn: boolean) {
    if (this.edgeGlow && this.halo) {
      this.edgeGlow.visible = true;
      this.halo.visible = true;
      const offColor = new THREE.Color(0x059669);
      const onColor = new THREE.Color(0x34d399);
      this.glowColor.copy(isTurn ? onColor : offColor);
    }
  }

  public addBurst(
    x: number,
    z: number,
    color: number = 0xf59e0b,
    y: number = 0.05,
  ) {
    const group = new THREE.Group();
    group.position.set(x, y, z);

    const threeColor = new THREE.Color(color);

    // 1. Expanding Ring
    const ringGeo = new THREE.RingGeometry(0.2, 0.25, 32);
    const ringMat = new THREE.MeshBasicMaterial({
      color: threeColor,
      transparent: true,
      opacity: 0.9,
      blending: THREE.AdditiveBlending,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.rotation.x = -Math.PI / 2;
    group.add(ring);

    // 2. Particle Burst
    const partCount = 12;
    const posArr = new Float32Array(partCount * 3);
    const velArr: THREE.Vector3[] = [];

    for (let i = 0; i < partCount; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 1.5 + Math.random() * 3.0;
      const radius = 0.1 + Math.random() * 0.4;

      posArr[i * 3] = Math.cos(angle) * radius;
      posArr[i * 3 + 1] = 0;
      posArr[i * 3 + 2] = Math.sin(angle) * radius;

      velArr.push(
        new THREE.Vector3(
          Math.cos(angle) * speed,
          (0.5 + Math.random()) * speed * 1.5, // Stronger upward bias
          Math.sin(angle) * speed,
        ),
      );
    }

    const partGeo = new THREE.BufferGeometry();
    partGeo.setAttribute('position', new THREE.BufferAttribute(posArr, 3));

    const partMat = new THREE.PointsMaterial({
      map: this.sparkTexture,
      size: 0.6,
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      opacity: 1.0,
      color: threeColor,
    });

    const particles = new THREE.Points(partGeo, partMat);
    particles.userData.velocities = velArr;
    group.add(particles);

    this.pingGroup.add(group);
    this.activePings.push({
      group,
      ring: ring as any,
      particles,
      age: 0,
      maxAge: 0.8,
    });
  }

  public addPackBurst(x: number, y: number, z: number) {
    const group = new THREE.Group();
    group.position.set(x, y, z);

    const amberColor = new THREE.Color(0xf59e0b);

    // 1. Particle Geyser (Fountain out of the bag opening)
    const partCount = 150;
    const posArr = new Float32Array(partCount * 3);
    const velArr: THREE.Vector3[] = [];

    for (let i = 0; i < partCount; i++) {
      // Create a cone facing upwards
      const theta = Math.random() * Math.PI * 2;
      const radius = Math.random() * 0.4; // Tighter cluster at origin
      const speed = 6.0 + Math.random() * 12.0;

      // Cone spread
      const vx = Math.cos(theta) * radius * speed * 0.5;
      const vz = Math.sin(theta) * radius * speed * 0.5;
      const vy = speed; // Strong upward force

      // Spawn slightly above the pack center to simulate coming out of the opening
      posArr[i * 3] = vx * 0.01;
      posArr[i * 3 + 1] = 0.5 + Math.random() * 0.3; // Start higher up
      posArr[i * 3 + 2] = vz * 0.01;

      velArr.push(new THREE.Vector3(vx, vy, vz));
    }

    const partGeo = new THREE.BufferGeometry();
    partGeo.setAttribute('position', new THREE.BufferAttribute(posArr, 3));

    const partMat = new THREE.PointsMaterial({
      map: this.sparkTexture,
      size: 2.5,
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      depthTest: true, // Occlude behind bag walls if needed
      opacity: 1.0,
      color: amberColor,
    });

    const particles = new THREE.Points(partGeo, partMat);
    particles.renderOrder = 250;
    particles.userData.velocities = velArr;
    group.add(particles);

    this.pingGroup.add(group);
    this.activePings.push({
      group,
      ring: null as any,
      particles,
      age: 0,
      maxAge: 1.5,
    });
  }

  public fireProjectile(
    start: THREE.Vector3,
    end: THREE.Vector3,
    color: number = 0xff2200,
  ) {
    // 1. The Projectile Head - Smaller and subtler
    const meshGeo = new THREE.SphereGeometry(0.08, 8, 8);
    const meshMat = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity: 0.8,
      blending: THREE.AdditiveBlending,
    });
    const mesh = new THREE.Mesh(meshGeo, meshMat);
    mesh.position.copy(start);
    this.pingGroup.add(mesh);

    // 2. The Trail - Thinner
    const dist = start.distanceTo(end);
    const trailLen = Math.min(0.6, dist * 0.3);
    const trailPoints = [
      start.clone(),
      start.clone().lerp(end, trailLen / dist),
    ];
    const trailGeo = new THREE.BufferGeometry().setFromPoints(trailPoints);
    const trailMat = new THREE.LineBasicMaterial({
      color,
      transparent: true,
      opacity: 0.3,
      blending: THREE.AdditiveBlending,
      linewidth: 2,
    });
    const trail = new THREE.Line(trailGeo, trailMat);
    this.pingGroup.add(trail);

    // Calculate duration based on distance (speed = 15 units/sec)
    const speed = 15.0;
    const maxAge = dist / speed;

    this.activeProjectiles.push({
      mesh,
      trail,
      start: start.clone(),
      end: end.clone(),
      age: 0,
      maxAge,
      color,
    });
  }

  public addFloatingText(
    x: number,
    y: number,
    z: number,
    text: string,
    color: number = 0xff3333,
    scale: number = 1.0,
  ) {
    // Render the text to a canvas, then use that canvas as a sprite texture.
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 128;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const hex = '#' + color.toString(16).padStart(6, '0');

    ctx.font = 'bold 88px "Courier New", monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    // Thick black outline for readability over any background
    ctx.lineWidth = 8;
    ctx.strokeStyle = 'rgba(0,0,0,0.9)';
    ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
    // Fill with the tint color
    ctx.fillStyle = hex;
    ctx.fillText(text, canvas.width / 2, canvas.height / 2);

    const texture = new THREE.CanvasTexture(canvas);
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const material = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
      opacity: 1.0,
    });
    const sprite = new THREE.Sprite(material);
    const baseScale = 1.2 * scale;
    sprite.scale.set(baseScale * 2, baseScale, 1);
    sprite.position.set(x, y, z);
    sprite.renderOrder = 999;

    this.pingGroup.add(sprite);
    this.activeFloatingTexts.push({
      sprite,
      texture,
      startY: y,
      riseDistance: 1.2,
      age: 0,
      maxAge: 1.2,
    });
  }

  public tick(delta: number) {
    this.time += delta;

    // 0. Ethereal Table Motion
    if (this.tableSurface) {
      // Very slow, non-linear rotation (drifting speed)
      this.tableSurface.rotation.z +=
        delta * (0.008 + Math.sin(this.time * 0.2) * 0.004);

      // Gentle breathing/scaling effect
      const breathe = 1.0 + Math.sin(this.time * 0.35) * 0.012;
      this.tableSurface.scale.set(breathe, breathe, 1);
    }

    if (this.edgeGlow && this.edgeGlow.visible) {
      const pulse = 0.85 + Math.sin(this.time * 0.5) * 0.15;
      const mat = this.edgeGlow.material as THREE.MeshBasicMaterial;
      mat.opacity = pulse;
      // Smooth color transition
      mat.color.lerp(this.glowColor.clone().multiplyScalar(3.0), 0.05);
    }

    if (this.halo && this.halo.visible) {
      const pulse = 0.2 + Math.sin(this.time * 0.4) * 0.1;
      const mat = this.halo.material as THREE.MeshBasicMaterial;
      mat.opacity = pulse;
      // Smooth color transition
      mat.color.lerp(this.glowColor, 0.05);
    }

    if (this.underTableLight) {
      this.underTableLight.color.lerp(this.glowColor, 0.05);
      this.underTableLight.intensity = 6.0 + Math.sin(this.time * 0.6) * 3.0; // Pushes light much further into the room
    }

    // 1. Projectiles
    for (let i = this.activeProjectiles.length - 1; i >= 0; i--) {
      const proj = this.activeProjectiles[i];
      proj.age += delta;

      const progress = Math.min(1.0, proj.age / proj.maxAge);

      // Arc the projectile slightly
      const currentPos = proj.start.clone().lerp(proj.end, progress);
      // Add arc height
      const arcHeight = Math.sin(progress * Math.PI) * 1.5;
      currentPos.y += arcHeight;

      proj.mesh.position.copy(currentPos);

      // Update trail
      const trailEnd = currentPos.clone();
      const trailProgress = Math.max(0, progress - 0.15); // trail lags behind by 15%
      const trailStart = proj.start.clone().lerp(proj.end, trailProgress);
      trailStart.y += Math.sin(trailProgress * Math.PI) * 1.5;

      const positions = proj.trail.geometry.attributes
        .position as THREE.BufferAttribute;
      positions.setXYZ(0, trailStart.x, trailStart.y, trailStart.z);
      positions.setXYZ(1, trailEnd.x, trailEnd.y, trailEnd.z);
      positions.needsUpdate = true;

      if (progress >= 1.0) {
        // Impact!
        this.addBurst(proj.end.x, proj.end.z, proj.color);

        // Clean up
        this.pingGroup.remove(proj.mesh);
        this.pingGroup.remove(proj.trail);
        this.activeProjectiles.splice(i, 1);
      }
    }

    // 2. Pings / Bursts

    for (let i = this.activePings.length - 1; i >= 0; i--) {
      const ping = this.activePings[i];
      ping.age += delta;

      const progress = ping.age / ping.maxAge;
      if (progress >= 1.0) {
        this.pingGroup.remove(ping.group);
        this.activePings.splice(i, 1);
        continue;
      }

      // Animate Ring
      if (ping.ring) {
        const ringScale = 0.1 + progress * 4.0;
        ping.ring.scale.set(ringScale, ringScale, 1);
        const ringMat = ping.ring.material as THREE.MeshBasicMaterial;
        ringMat.opacity = Math.pow(1.0 - progress, 2.0);
      }

      // Animate Particles
      const posAttr = ping.particles.geometry.attributes
        .position as THREE.BufferAttribute;
      const vels = ping.particles.userData.velocities as THREE.Vector3[];

      for (let j = 0; j < vels.length; j++) {
        const v = vels[j];
        posAttr.setXYZ(
          j,
          posAttr.getX(j) + v.x * delta,
          posAttr.getY(j) + v.y * delta,
          posAttr.getZ(j) + v.z * delta,
        );
        // Add gravity/drag to particles
        v.y -= delta * 1.5;
        v.multiplyScalar(0.98);
      }
      posAttr.needsUpdate = true;

      const partMat = ping.particles.material as THREE.PointsMaterial;
      partMat.opacity = Math.max(0, 1.0 - progress * 1.2);
      partMat.size = 0.4 * (1.0 - progress * 0.5);
    }

    // 3. Floating texts (damage numbers, etc.)
    for (let i = this.activeFloatingTexts.length - 1; i >= 0; i--) {
      const ft = this.activeFloatingTexts[i];
      ft.age += delta;
      const progress = ft.age / ft.maxAge;

      if (progress >= 1.0) {
        this.pingGroup.remove(ft.sprite);
        ft.texture.dispose();
        (ft.sprite.material as THREE.SpriteMaterial).dispose();
        this.activeFloatingTexts.splice(i, 1);
        continue;
      }

      // Rise upward with eased-out curve
      const rise = 1 - Math.pow(1 - progress, 2);
      ft.sprite.position.y = ft.startY + rise * ft.riseDistance;

      // Fade: hold for first 60% of life, then fade out
      const fadeProgress = Math.max(0, (progress - 0.4) / 0.6);
      (ft.sprite.material as THREE.SpriteMaterial).opacity = 1.0 - fadeProgress;

      // Slight scale-up to draw the eye
      const scalePulse = 1.0 + Math.min(progress * 3, 0.3);
      const baseX = ft.sprite.userData.baseScaleX ?? ft.sprite.scale.x;
      const baseY = ft.sprite.userData.baseScaleY ?? ft.sprite.scale.y;
      if (ft.sprite.userData.baseScaleX == null) {
        ft.sprite.userData.baseScaleX = baseX;
        ft.sprite.userData.baseScaleY = baseY;
      }
      ft.sprite.scale.set(baseX * scalePulse, baseY * scalePulse, 1);
    }
  }
}
