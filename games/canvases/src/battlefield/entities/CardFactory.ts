import * as THREE from 'three';
import type { CardView } from '../types';
import { createTextureUtils } from '../textures';
import { CDN_BASE } from '../cdn';

export class CardFactory {
  public cardW = 0.75;
  public cardH = 1.1;

  public geometry: THREE.PlaneGeometry;
  public backGeometry: THREE.PlaneGeometry;
  public edgesGeometry: THREE.EdgesGeometry;

  // Shared materials for VFX
  public prismEdgeMaterial: THREE.LineBasicMaterial;
  public prismShellMaterial: THREE.MeshBasicMaterial;
  public sparkMaterial: THREE.SpriteMaterial;

  private textureUtils: ReturnType<typeof createTextureUtils>;
  private materialCache = new Map<
    string,
    THREE.ShaderMaterial | THREE.MeshBasicMaterial
  >();
  private wearTexture: THREE.Texture;
  private wearTexture2: THREE.Texture;
  private rarityTextures: Record<string, THREE.Texture> = {};

  private cyberSkinShader = {
    uniforms: {
      tDiffuse: { value: null },
      tWear: { value: null },
      tWear2: { value: null },
      tRarity: { value: null },
      time: { value: 0 },
      seed: { value: 0 },
      wear: { value: 0 },
      rarityColor: { value: new THREE.Color(0xf59e0b) },
      isHolo: { value: 0.0 },
      isUsable: { value: 1.0 },
      isSick: { value: 0.0 },
      showBadge: { value: 1.0 },
      grayscale: { value: 0.0 },
    },
    vertexShader: `
      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vViewPosition;

      void main() {
        vUv = uv;
        vNormal = normalize(normalMatrix * normal);
        vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
        vViewPosition = -mvPosition.xyz;
        gl_Position = projectionMatrix * mvPosition;
      }
    `,
    fragmentShader: `
      uniform sampler2D tDiffuse;
      uniform sampler2D tWear;
      uniform sampler2D tWear2;
      uniform sampler2D tRarity;
      uniform float time;
      uniform float seed;
      uniform float wear;
      uniform vec3 rarityColor;
      uniform float isHolo;
      uniform float isUsable;
      uniform float isSick;
      uniform float showBadge;
      uniform float grayscale;

      varying vec2 vUv;
      varying vec3 vNormal;
      varying vec3 vViewPosition;

      void main() {
        // 0. Digital static — scan lines + pixel flickers for ALL cards
        float artMaskFx = smoothstep(0.30, 0.34, vUv.y);
        float phase = fract(seed * 0.00037);

        vec4 baseTex = texture2D(tDiffuse, vUv);
        vec3 finalColor = baseTex.rgb;

        // Rarity badge — composited early so effects apply on top
        if (showBadge > 0.5) {
          vec2 badgeSize = vec2(0.05, 0.05 * (0.75 / 1.1));
          vec2 badgeCenter = vec2(0.05 + badgeSize.x * 0.5, 1.0 - 0.035 - badgeSize.y * 0.5);
          vec2 badgeUv = (vUv - badgeCenter + badgeSize * 0.5) / badgeSize;

          float bgTick = floor(time * 1.5 + phase * 80.0);
          float bgRand = fract(sin(bgTick * 57.13 + phase * 431.0) * 7919.0);
          float bgGlitch = step(0.85, bgRand);
          vec2 glitchedBadgeUv = badgeUv;
          glitchedBadgeUv.x += bgGlitch * (bgRand - 0.85) * 3.0;
          float bgScanGlitch = step(0.9, fract(sin(floor(badgeUv.y * 8.0 + bgTick) * 43.17) * 9137.0));
          glitchedBadgeUv.x += bgScanGlitch * bgGlitch * 0.15;

          vec4 badge = vec4(0.0);
          if (glitchedBadgeUv.x >= 0.0 && glitchedBadgeUv.x <= 1.0 && glitchedBadgeUv.y >= 0.0 && glitchedBadgeUv.y <= 1.0) {
            badge = texture2D(tRarity, glitchedBadgeUv);
          }
          float glowPulse = 0.1 + 0.05 * sin(time * 1.5 + phase * 6.28);
          finalColor += badge.rgb * badge.a * glowPulse;
          finalColor = mix(finalColor, badge.rgb, badge.a * 0.5);
        }

        // Subtle scrolling scan lines — seed offsets phase
        float scanLine = step(0.5, fract(vUv.y * 20.0 - time * 0.15 + phase));
        finalColor *= mix(1.0, 0.88 + 0.12 * scanLine, artMaskFx);

        // Seed-unique vignette burn — some corners darker/lighter than others
        vec2 vc = vUv - 0.5;
        float vignette = dot(vc, vc) * 2.0;
        float burnAngle = phase * 6.28;
        float burnBias = dot(vc, vec2(cos(burnAngle), sin(burnAngle)));
        float burnAmount = vignette * (0.15 + 0.15 * burnBias);
        finalColor *= mix(1.0, 1.0 - burnAmount, artMaskFx);

        // Occasional micro-glitch — a single thin band stutters briefly
        float tick = floor(time * 2.0 + phase * 100.0);
        float glitchChance = fract(sin(tick * 43.758 + phase * 991.0) * 9137.0);
        float glitchOn = step(0.85, glitchChance) * artMaskFx;
        float glitchY = fract(sin(tick * 17.31 + phase * 337.0) * 5791.0);
        float glitchBand = (1.0 - smoothstep(0.0, 0.06, abs(vUv.y - glitchY))) * glitchOn;
        float glitchShift = (glitchChance - 0.85) * 0.5;
        vec3 glitchCol = texture2D(tDiffuse, vec2(vUv.x + glitchShift * glitchBand, vUv.y)).rgb;
        finalColor = mix(finalColor, glitchCol, glitchBand);

        // 1. Digital Glitch Effect — holo cards only, seed-unique
        if (isHolo > 0.5) {
          float artMask = smoothstep(0.30, 0.34, vUv.y);

          // Normalize seed for GLSL float precision
          float holoPhase = fract(seed * 0.00037);

          // RGB chromatic split — intermittent, not continuous
          float moveOffset = time * 0.8 + (vViewPosition.x + vViewPosition.y + vViewPosition.z) * 0.5;
          float splitTick = floor(moveOffset * 0.5 + holoPhase * 100.0);
          float splitChance = fract(sin(splitTick * 73.17 + holoPhase * 491.0) * 5791.0);
          float splitOn = step(0.75, splitChance);
          float splitAmount = (0.006 + 0.004 * sin(moveOffset * 3.0 + holoPhase * 100.0)) * artMask * splitOn;
          float rShift = texture2D(tDiffuse, vUv + vec2(splitAmount, 0.0)).r;
          float bShift = texture2D(tDiffuse, vUv - vec2(splitAmount, 0.0)).b;
          finalColor.r = mix(finalColor.r, rShift, 0.5 * artMask * splitOn);
          finalColor.b = mix(finalColor.b, bShift, 0.5 * artMask * splitOn);

          // Movement-seeded randomness
          float movePhase = moveOffset * 3.0 + holoPhase * 700.0;
          float glitchTrigger = fract(sin(floor(movePhase) * 43.758 + holoPhase * 991.0) * 9137.0);

          // Glitch bands — horizontal stripe offsets
          float bandY = fract(sin(floor(vUv.y * 20.0 + movePhase) * 43.17 + holoPhase * 777.0) * 9137.0);
          float bandActive = step(0.88, bandY);
          float bandShift = (bandY - 0.95) * 0.3 * bandActive;
          vec4 glitchTex = texture2D(tDiffuse, vec2(vUv.x + bandShift, vUv.y));
          finalColor = mix(finalColor, glitchTex.rgb, bandActive * 0.5 * artMask);

          // Big glitch burst
          float burstActive = step(0.8, glitchTrigger);
          float burstShift = (glitchTrigger - 0.8) * 0.1 * burstActive;
          vec2 burstUv = vUv + vec2(burstShift * sin(holoPhase * 500.0), burstShift * cos(holoPhase * 300.0));
          vec4 burstTex = texture2D(tDiffuse, burstUv);
          finalColor = mix(finalColor, burstTex.rgb, burstActive * 0.6 * artMask);

          // Color channel swap
          float swapTrigger = fract(sin(floor(movePhase * 1.7) * 91.37 + holoPhase * 300.0) * 7919.0);
          float swapActive = step(0.8, swapTrigger) * artMask;
          finalColor.rgb = mix(finalColor.rgb, finalColor.bgr, swapActive * 0.7);

          // Pixel noise bursts — white static
          float noiseIntensity = mix(0.99, 0.96, burstActive);
          float noiseVal = fract(sin(dot(vUv * 500.0 + holoPhase + movePhase, vec2(12.9898, 78.233))) * 43758.5453);
          float noiseMask = step(noiseIntensity, noiseVal) * artMask;
          finalColor += vec3(1.0) * noiseMask * 0.4;
        }

        // Border energy pulse — rounded rect
        float cRadius = 0.055;
        vec2 rp = abs(vUv - 0.5);
        vec2 q = rp - vec2(0.5 - cRadius);
        float rrDist = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - cRadius;
        float borderMask = (1.0 - smoothstep(-0.008, -0.002, rrDist)) * smoothstep(-0.02, -0.012, rrDist);

        // Perimeter position via atan2 from center — smooth around corners
        float angle = atan(vUv.y - 0.5, vUv.x - 0.5); // -PI to PI
        float perim = fract(angle / 6.2832 + 0.5); // 0-1 loop

        // Steady slow rotation with occasional brief speed-ups
        float basePos = time * 0.015 + phase;
        // Add occasional burst offset on top
        float burstTick = floor(time * 0.15 + phase * 50.0);
        float burstRand = fract(sin(burstTick * 91.37 + phase * 713.0) * 4817.0);
        float burstOn = step(0.75, burstRand);
        float burstFrac = fract(time * 0.15 + phase * 50.0);
        float burstWindow = burstOn * smoothstep(0.0, 0.05, burstFrac) * smoothstep(0.3, 0.15, burstFrac);
        float burstOffset = burstWindow * 0.15 * burstRand;

        float pulsePos = fract(basePos + burstOffset);
        float pulseDist = abs(fract(perim) - pulsePos);
        pulseDist = min(pulseDist, 1.0 - pulseDist);
        float pulse = (1.0 - smoothstep(0.0, 0.10, pulseDist)) * borderMask;

        // Seed-based pulse color
        vec3 pulseColor = vec3(
          0.4 + 0.6 * fract(phase * 3.7),
          0.4 + 0.6 * fract(phase * 5.1),
          0.4 + 0.6 * fract(phase * 7.3)
        );
        finalColor += pulseColor * pulse * 0.5;

        // 2. Rust/wear overlay — drifts slowly, more on heavily worn cards
        if (wear > 0.05) {
          vec2 drift = vec2(sin(time * 1.2 + wear) * 0.06, cos(time * 0.9 + wear) * 0.05) * wear;
          vec4 wearA = texture2D(tWear, vUv + drift);
          vec2 uvB = (vUv - 0.5) * 0.8 + 0.5;
          uvB = vec2(1.0 - uvB.y, uvB.x);
          vec4 wearB = texture2D(tWear2, uvB - drift * 1.5);
          vec3 wearRgb = wearA.rgb * 0.6 + wearB.rgb * 0.4;
          float rust = 1.0 - dot(wearRgb, vec3(0.299, 0.587, 0.114));
          rust = pow(rust, 1.5);
          float wearLuma = dot(finalColor, vec3(0.299, 0.587, 0.114));
          finalColor = mix(finalColor, vec3(wearLuma), wear * rust * 0.5);
          vec3 stain = vec3(0.22, 0.1, 0.04) * rust;
          finalColor = mix(finalColor, finalColor * 0.3 + stain, wear * rust * 0.4);
          finalColor *= 1.0 - wear * 0.2;
        }

        // 3. Shooting border lights (rounded rect, unaffected by wear)
        vec2 slP = vUv - 0.5;
        vec2 slB = vec2(0.5, 0.5);
        float slR = 0.05;
        vec2 slQ = abs(slP) - slB + slR;
        float slSdf = length(max(slQ, 0.0)) - slR;
        float slEdgeDist = abs(slSdf);
        float slEdgeMask = 1.0 - smoothstep(0.0, 0.02, slEdgeDist);

        float slPerim = fract(atan(slP.y, slP.x) / 6.28318 + 0.5);

        float slLight = 0.0;
        for (int i = 0; i < 3; i++) {
          float fi = float(i);
          float slPhase = phase * 13.1 + fi * 1.7;
          float slBaseSpeed = 0.02 + 0.04 * fract(phase * 7.3 + fi * 0.37);
          float slWobble = slBaseSpeed * 0.75 * sin(time * 0.2 + slPhase);
          float slPos = fract(time * slBaseSpeed * 1.75 + slWobble + fi / 3.0 + slPhase);
          float slD = abs(slPerim - slPos);
          slD = min(slD, 1.0 - slD);
          slLight += exp(-400.0 * slD * slD);
        }

        finalColor += slEdgeMask * slLight * pulseColor * 1.5;

        // 4. Status tinting — summoning sick cards go fully grayscale and
        // darkened so they read as "asleep" from across the board.
        if (isSick > 0.5) {
          float sickLuma = dot(finalColor, vec3(0.299, 0.587, 0.114));
          finalColor = vec3(sickLuma) * 0.45;
        }
        if (isUsable < 0.5) {
          finalColor *= 0.7;
        }


        // Grayscale mix (for draft unavailable cards)
        if (grayscale > 0.0) {
          float luma = dot(finalColor, vec3(0.299, 0.587, 0.114));
          finalColor = mix(finalColor, vec3(luma) * 0.5, grayscale);
        }

        gl_FragColor = vec4(finalColor, baseTex.a);
      }
    `,
  };

  constructor() {
    // 1. Geometries
    const cornerRadius = Math.min(this.cardW, this.cardH) * 0.055;
    const shape = new THREE.Shape();
    const x = -this.cardW / 2;
    const y = -this.cardH / 2;
    const w = this.cardW;
    const h = this.cardH;
    const r = Math.min(cornerRadius, w / 2, h / 2);
    shape.moveTo(x + r, y);
    shape.lineTo(x + w - r, y);
    shape.quadraticCurveTo(x + w, y, x + w, y + r);
    shape.lineTo(x + w, y + h - r);
    shape.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    shape.lineTo(x + r, y + h);
    shape.quadraticCurveTo(x, y + h, x, y + h - r);
    shape.lineTo(x, y + r);
    shape.quadraticCurveTo(x, y, x + r, y);
    const roundedGeometry = new THREE.ShapeGeometry(shape);

    // Fix UVs on rounded geometry to 0-1 range
    const frontUvAttr = roundedGeometry.getAttribute(
      'uv',
    ) as THREE.BufferAttribute;
    for (let i = 0; i < frontUvAttr.count; i++) {
      frontUvAttr.setX(i, (frontUvAttr.getX(i) - x) / w);
      frontUvAttr.setY(i, (frontUvAttr.getY(i) - y) / h);
    }
    frontUvAttr.needsUpdate = true;

    this.geometry = roundedGeometry as any;
    this.edgesGeometry = new THREE.EdgesGeometry(roundedGeometry);

    // Mirrored back geometry — clone front (UVs already corrected)
    this.backGeometry = this.geometry.clone() as any;
    this.backGeometry.scale(-1, 1, 1);

    // 2. Texture Utils
    this.textureUtils = createTextureUtils({
      cardTextureScale: 2,
      cardW: this.cardW,
      cardH: this.cardH,
    });

    // Wear/rust overlay texture
    this.wearTexture = new THREE.TextureLoader().load(`${CDN_BASE}/metal.jpg`, (tex) => {
      tex.wrapS = tex.wrapT = THREE.ClampToEdgeWrapping;
      tex.colorSpace = THREE.SRGBColorSpace;
    });

    this.wearTexture2 = new THREE.TextureLoader().load(`${CDN_BASE}/table.jpg`, (tex) => {
      tex.wrapS = tex.wrapT = THREE.ClampToEdgeWrapping;
      tex.colorSpace = THREE.SRGBColorSpace;
    });

    // Rarity badge textures
    const loader = new THREE.TextureLoader();
    for (const rarity of [
      'public',
      'private',
      'restricted',
      'classified',
      'singularity',
    ]) {
      this.rarityTextures[rarity] = loader.load(
        `${CDN_BASE}/rarities/${rarity}.png`,
        (tex) => {
          tex.colorSpace = THREE.SRGBColorSpace;
        },
      );
    }

    // 3. Shared VFX Materials
    this.prismEdgeMaterial = new THREE.LineBasicMaterial({
      color: 0xffffff,
      transparent: true,
      opacity: 0.3,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });

    this.prismShellMaterial = new THREE.MeshBasicMaterial({
      color: 0xffffff,
      transparent: true,
      opacity: 0.05,
      blending: THREE.NormalBlending,
      side: THREE.DoubleSide,
      depthWrite: false,
    });

    // Sickness Particle Texture (Simple gradient circle)
    const particleCanvas = document.createElement('canvas');
    particleCanvas.width = 64;
    particleCanvas.height = 64;
    const ctx = particleCanvas.getContext('2d');
    if (ctx) {
      ctx.clearRect(0, 0, 64, 64);
      const grad = ctx.createRadialGradient(32, 28, 2, 32, 28, 24);
      grad.addColorStop(0, 'rgba(255, 255, 255, 0.95)');
      grad.addColorStop(0.5, 'rgba(255, 255, 255, 0.45)');
      grad.addColorStop(1, 'rgba(255, 255, 255, 0)');
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.ellipse(32, 28, 10, 14, 0, 0, Math.PI * 2);
      ctx.fill();
    }
    const particleTexture = new THREE.CanvasTexture(particleCanvas);

    this.sparkMaterial = new THREE.SpriteMaterial({
      map: particleTexture,
      color: 0xffffff,
      transparent: true,
      opacity: 0.0,
      blending: THREE.AdditiveBlending,
      depthTest: true,
      depthWrite: false,
    });
  }

  private hashId(id: string): number {
    let h = 0;
    for (let i = 0; i < id.length; i++) {
      h = (Math.imul(31, h) + id.charCodeAt(i)) | 0;
    }
    return (h >>> 0) / 0xffffffff;
  }

  public getMaterial(
    card: CardView,
    isSick: boolean,
  isUsable: boolean,
  ): THREE.ShaderMaterial | THREE.MeshBasicMaterial {
    const texture =
      this.textureUtils.createCardTexture(card) ??
      this.textureUtils.createCardBackTexture();
    const defaultWear = card.visual_spec
      ? card.visual_spec.wear
      : 0.4 + this.hashId(card.id) * 0.6;
    const key = `${texture.uuid}:${isSick}:${isUsable}:${card.rarity}:${card.visual_spec?.seed ?? 0}:${defaultWear}:${card.visual_spec?.holo ?? false}`;

    if (this.materialCache.has(key)) {
      return this.materialCache.get(key)! as any;
    }

    const rarityColors: Record<string, number> = {
      public: 0x94a3b8,
      private: 0x10b981,
      restricted: 0xa855f7,
      classified: 0xf43f5e,
      singularity: 0xfacc15,
    };

    const mat = new THREE.ShaderMaterial({
      uniforms: THREE.UniformsUtils.clone(this.cyberSkinShader.uniforms),
      vertexShader: this.cyberSkinShader.vertexShader,
      fragmentShader: this.cyberSkinShader.fragmentShader,
      transparent: true,
      side: THREE.FrontSide,
    });

    mat.uniforms.tDiffuse.value = texture;
    mat.uniforms.tWear.value = this.wearTexture;
    mat.uniforms.tWear2.value = this.wearTexture2;
    mat.uniforms.tRarity.value =
      this.rarityTextures[card.rarity] || this.rarityTextures['public'];
    mat.uniforms.seed.value =
      card.visual_spec?.seed ?? this.hashId(card.id) * 1000;
    mat.uniforms.wear.value = defaultWear;
    mat.uniforms.rarityColor.value = new THREE.Color(
      rarityColors[card.rarity] || 0xf59e0b,
    );
    mat.uniforms.isHolo.value = card.visual_spec?.holo ? 1.0 : 0.0;
    mat.uniforms.isSick.value = isSick ? 1.0 : 0.0;
    mat.uniforms.isUsable.value = isUsable ? 1.0 : 0.0;
    mat.uniforms.showBadge.value = card.rarity === 'public' ? 0.0 : 1.0;

    this.materialCache.set(key, mat);
    return mat;
  }

  public getBackMaterial(): THREE.MeshBasicMaterial {
    const key = 'card_back_material';
    if (this.materialCache.has(key)) {
      return this.materialCache.get(key)! as THREE.MeshBasicMaterial;
    }

    const texture = this.textureUtils.createCardBackTexture();
    const mat = new THREE.MeshBasicMaterial({
      map: texture,
      color: 0xffffff,
      transparent: true,
      side: THREE.FrontSide, // Plane is flipped in geometry
    });

    this.materialCache.set(key, mat);
    return mat;
  }

  public getInfrastructureMaterial(card: CardView): THREE.MeshBasicMaterial {
    const key = `infra:${card.id}:${card.tapped}`;
    if (this.materialCache.has(key)) {
      return this.materialCache.get(key)! as THREE.MeshBasicMaterial;
    }

    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    if (ctx) {
      const accentColor = card.colors?.[0] || '#f59e0b';
      ctx.clearRect(0, 0, 256, 256);

      // Hexagon Logic - Pointy topped in canvas
      const centerX = 128;
      const centerY = 128;
      const drawR = 90;

      ctx.beginPath();
      for (let j = 0; j < 6; j++) {
        const angle = (j * 2 * Math.PI) / 6 - Math.PI / 2;
        const px = centerX + Math.cos(angle) * drawR;
        const py = centerY + Math.sin(angle) * drawR;
        if (j === 0) ctx.moveTo(px, py);
        else ctx.lineTo(px, py);
      }
      ctx.closePath();

      if (!card.tapped) {
        ctx.fillStyle = accentColor;
        ctx.shadowColor = accentColor;
        ctx.shadowBlur = 24;
        ctx.fill();

        ctx.strokeStyle = 'rgba(255, 255, 255, 0.9)';
        ctx.lineWidth = 4;
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(centerX, centerY, 28, 0, Math.PI * 2);
        ctx.stroke();
      } else {
        ctx.save();
        ctx.strokeStyle = accentColor;
        ctx.lineWidth = 6;
        ctx.globalAlpha = 0.2;
        ctx.setLineDash([15, 8]);
        ctx.stroke();
        ctx.restore();
      }
    }

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;

    const mat = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      blending: THREE.NormalBlending, // Solid etching
      opacity: 0.95,
      side: THREE.DoubleSide,
      depthWrite: false,
      depthTest: true,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1,
    });

    this.materialCache.set(key, mat);
    return mat;
  }

  public getTextureUtils() {
    return this.textureUtils;
  }
}
