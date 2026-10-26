import * as THREE from 'three';

export const WATER_Y = -3;

export type PartyStage = {
  sun: THREE.DirectionalLight;
  update: (time: number) => void;
  dispose: () => void;
};

/**
 * The shared outdoor set for party minigames: a bright sky, sunlight with
 * soft shadows, a gently rolling sea and a few islands and clouds on the
 * horizon. Minigames add their own play area around the origin.
 */
export function createPartyStage(scene: THREE.Scene, options: { mobile: boolean }): PartyStage {
  const disposables: { dispose: () => void }[] = [];
  const track = <T extends { dispose: () => void }>(item: T) => {
    disposables.push(item);
    return item;
  };

  const sky = document.createElement('canvas');
  sky.width = 4;
  sky.height = 256;
  const ctx = sky.getContext('2d')!;
  const gradient = ctx.createLinearGradient(0, 0, 0, 256);
  gradient.addColorStop(0, '#3fb4f2');
  gradient.addColorStop(0.62, '#9fe0fb');
  gradient.addColorStop(1, '#e6f8ff');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 4, 256);
  const skyTexture = track(new THREE.CanvasTexture(sky));
  skyTexture.colorSpace = THREE.SRGBColorSpace;
  scene.background = skyTexture;
  scene.fog = new THREE.Fog('#bfeaff', 45, 120);

  scene.add(new THREE.HemisphereLight('#dff4ff', '#3d8f7a', 1.5));
  const sun = new THREE.DirectionalLight('#fff1d6', 3.1);
  sun.position.set(-10, 22, 12);
  sun.castShadow = true;
  const size = options.mobile ? 1024 : 2048;
  sun.shadow.mapSize.set(size, size);
  Object.assign(sun.shadow.camera, { left: -15, right: 15, top: 15, bottom: -15, near: 1, far: 70 });
  sun.shadow.bias = -0.0004;
  sun.shadow.normalBias = 0.03;
  scene.add(sun);

  const ripples = document.createElement('canvas');
  ripples.width = 256;
  ripples.height = 256;
  const rctx = ripples.getContext('2d')!;
  rctx.fillStyle = '#ffffff';
  rctx.fillRect(0, 0, 256, 256);
  rctx.strokeStyle = 'rgba(190, 236, 255, 0.9)';
  rctx.lineCap = 'round';
  for (let i = 0; i < 26; i++) {
    const x = (i * 97) % 256;
    const y = (i * 61) % 256;
    rctx.lineWidth = 2 + (i % 3);
    rctx.beginPath();
    rctx.arc(x, y, 10 + (i % 4) * 5, Math.PI * 1.15, Math.PI * 1.85);
    rctx.stroke();
  }
  const rippleTexture = track(new THREE.CanvasTexture(ripples));
  rippleTexture.wrapS = rippleTexture.wrapT = THREE.RepeatWrapping;
  rippleTexture.repeat.set(24, 24);
  rippleTexture.colorSpace = THREE.SRGBColorSpace;

  const uniforms = { uTime: { value: 0 } };
  const seaMaterial = track(new THREE.MeshStandardMaterial({ color: '#1485c2', map: rippleTexture, roughness: 0.18, metalness: 0.1 }));
  seaMaterial.onBeforeCompile = (shader) => {
    shader.uniforms.uTime = uniforms.uTime;
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', '#include <common>\nuniform float uTime;')
      .replace(
        '#include <begin_vertex>',
        `#include <begin_vertex>
        transformed.z += sin(position.x * 0.18 + uTime * 0.9) * 0.22 + cos(position.y * 0.23 + uTime * 1.2) * 0.18;`,
      );
  };
  const sea = new THREE.Mesh(track(new THREE.PlaneGeometry(260, 260, options.mobile ? 60 : 120, options.mobile ? 60 : 120)), seaMaterial);
  sea.rotation.x = -Math.PI / 2;
  sea.position.y = WATER_Y;
  sea.receiveShadow = true;
  scene.add(sea);

  const foam = new THREE.Mesh(
    track(new THREE.RingGeometry(3.3, 4.4, 48)),
    track(new THREE.MeshBasicMaterial({ color: '#e9fbff', transparent: true, opacity: 0.55, depthWrite: false })),
  );
  foam.rotation.x = -Math.PI / 2;
  foam.position.y = WATER_Y + 0.28;
  scene.add(foam);

  const sand = track(new THREE.MeshStandardMaterial({ color: '#f3d99a', roughness: 1 }));
  const grass = track(new THREE.MeshStandardMaterial({ color: '#5cc26b', roughness: 1 }));
  const trunk = track(new THREE.MeshStandardMaterial({ color: '#8a5a33', roughness: 1 }));
  const islandGeometry = track(new THREE.CylinderGeometry(1, 1.25, 1, 18));
  const hillGeometry = track(new THREE.SphereGeometry(1, 18, 12, 0, Math.PI * 2, 0, Math.PI / 2));
  const trunkGeometry = track(new THREE.CylinderGeometry(0.12, 0.18, 2.4, 8));
  const frondGeometry = track(new THREE.ConeGeometry(1.1, 0.5, 7));
  for (const [x, z, r] of [[-38, -46, 7], [44, -52, 9], [-60, -18, 5], [58, -10, 6], [8, -70, 11]]) {
    const island = new THREE.Group();
    const base = new THREE.Mesh(islandGeometry, sand);
    base.scale.set(r, 1, r);
    const hill = new THREE.Mesh(hillGeometry, grass);
    hill.scale.set(r * 0.72, r * 0.45, r * 0.72);
    hill.position.y = 0.5;
    island.add(base, hill);
    const palm = new THREE.Group();
    const stem = new THREE.Mesh(trunkGeometry, trunk);
    stem.position.y = 1.2;
    stem.rotation.z = 0.12;
    const fronds = new THREE.Mesh(frondGeometry, grass);
    fronds.position.set(0.15, 2.45, 0);
    palm.add(stem, fronds);
    palm.position.set(r * 0.25, r * 0.4, 0);
    island.add(palm);
    island.position.set(x, WATER_Y - 0.2, z);
    scene.add(island);
  }

  const cloudMaterial = track(new THREE.MeshStandardMaterial({ color: '#ffffff', roughness: 1, transparent: true, opacity: 0.95 }));
  const puffGeometry = track(new THREE.SphereGeometry(1, 14, 10));
  const clouds: THREE.Group[] = [];
  for (let i = 0; i < 7; i++) {
    const cloud = new THREE.Group();
    for (let j = 0; j < 4; j++) {
      const puff = new THREE.Mesh(puffGeometry, cloudMaterial);
      const s = 2.2 + ((i * 7 + j * 3) % 5) * 0.45;
      puff.scale.set(s * 1.4, s * 0.8, s);
      puff.position.set(j * 2.6 - 4, ((i + j) % 3) * 0.5, (j % 2) * 1.2);
      cloud.add(puff);
    }
    cloud.position.set(-90 + i * 30, 22 + (i % 3) * 5, -60 - (i % 2) * 20);
    clouds.push(cloud);
    scene.add(cloud);
  }

  return {
    sun,
    update: (time) => {
      uniforms.uTime.value = time;
      rippleTexture.offset.set(time * 0.012, time * 0.02);
      foam.scale.setScalar(1 + Math.sin(time * 1.6) * 0.05);
      clouds.forEach((cloud, i) => {
        cloud.position.x = ((-90 + i * 30 + time * (0.6 + i * 0.08) + 110) % 220) - 110;
      });
    },
    dispose: () => disposables.forEach((item) => item.dispose()),
  };
}
