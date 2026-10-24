import * as THREE from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { SMAAPass } from 'three/examples/jsm/postprocessing/SMAAPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';

export type SceneManagerOptions = {
  canvas: HTMLCanvasElement;
  useBloom?: boolean;
  useSmaa?: boolean;
};

export class SceneManager {
  public scene: THREE.Scene;
  public camera: THREE.PerspectiveCamera;
  public renderer: THREE.WebGLRenderer;
  public composer: EffectComposer;
  
  private clock: THREE.Clock;
  private resizeObserver: ResizeObserver;

  constructor(options: SceneManagerOptions) {
    const { canvas, useBloom = true, useSmaa = false } = options;

    // 1. Scene Setup
    this.scene = new THREE.Scene();
    const fogColor = 0x020617;
    this.scene.background = new THREE.Color(fogColor);
    this.scene.fog = new THREE.FogExp2(fogColor, 0.015);

    // 2. Camera Setup
    this.camera = new THREE.PerspectiveCamera(45, 1, 0.1, 200);
    // Initial position, can be overridden by TableManager or InteractionController
    this.camera.position.set(18.0, 6.6, 0); 
    this.camera.lookAt(0, 1.3, 0);
    this.camera.layers.enable(1); // Enable special layers (e.g. for reflections)
    this.scene.add(this.camera);

    // 3. Renderer Setup
    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: false, // Using SMAA or relying on high DPI
      powerPreference: 'high-performance',
      depth: true,
    });
    this.renderer.localClippingEnabled = true;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2.0));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;

    // 4. Composer (Post-processing)
    this.composer = new EffectComposer(this.renderer);
    this.composer.addPass(new RenderPass(this.scene, this.camera));

    if (useBloom) {
      const bloomPass = new UnrealBloomPass(
        new THREE.Vector2(window.innerWidth, window.innerHeight),
        0.5, // strength
        0.3, // radius
        0.9, // threshold
      );
      this.composer.addPass(bloomPass);
    }

    if (useSmaa) {
      const smaaPass = new SMAAPass();
      this.composer.addPass(smaaPass);
    }

    this.clock = new THREE.Clock();

    // 5. Auto-Resize
    this.resizeObserver = new ResizeObserver(() => this.resize());
    if (canvas.parentElement) {
      this.resizeObserver.observe(canvas.parentElement);
    }
    
    // Initial resize
    this.resize();
  }

  public render() {
    if (this.renderer.getContext().isContextLost()) return;
    this.composer.render();
  }

  public getDelta() {
    return this.clock.getDelta();
  }

  public getTime() {
    return this.clock.getElapsedTime();
  }

  public resize() {
    const canvas = this.renderer.domElement;
    const parent = canvas.parentElement;
    if (!parent) return;

    let { width, height } = parent.getBoundingClientRect();
    
    // Fallback if parent has no size yet (e.g. initial flex layout)
    if (width === 0 || height === 0) {
        width = window.innerWidth;
        height = window.innerHeight;
    }
    
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    
    this.renderer.setSize(width, height, false); 
    this.composer.setSize(width, height);
    
    // Update Bloom Pass resolution if present
    const bloomPass = this.composer.passes.find(p => p instanceof UnrealBloomPass) as UnrealBloomPass;
    if (bloomPass) {
        bloomPass.resolution.set(width, height);
    }
  }

  public dispose() {
    this.resizeObserver.disconnect();
    this.renderer.dispose();
    this.composer.dispose();
  }
}
