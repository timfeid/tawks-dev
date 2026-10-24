import * as THREE from 'three';
import { CDN_BASE } from './cdn';

export const createArcLabelMesh = (
  text: string,
  radius: number,
  angle: number,
  arcSize: number = 0.45,
  flip: boolean = false,
  flipV: boolean = false,
) => {
  const canvas = document.createElement('canvas');
  canvas.width = 1024;
  canvas.height = 128;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  let fontSize = 102;
  ctx.font = `700 ${fontSize}px "Orbitron", sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';

  let textWidth = ctx.measureText(text).width;
  while (textWidth > canvas.width * 0.92 && fontSize > 70) {
    fontSize -= 4;
    ctx.font = `700 ${fontSize}px "Orbitron", sans-serif`;
    textWidth = ctx.measureText(text).width;
  }

  // Thick white outline style
  ctx.lineJoin = 'round';
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 5.0;
  ctx.strokeText(text, canvas.width / 2, canvas.height / 2);

  let arcSpan = arcSize;
  if (arcSpan <= 0) {
    const normalized = Math.min(0.98, Math.max(0.32, textWidth / canvas.width));
    arcSpan = Math.min(Math.PI * 1.5, Math.max(0.6, normalized * 2.4));
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;

  const material = new THREE.MeshBasicMaterial({
    map: texture,
    transparent: true,
    depthWrite: false,
    depthTest: true,
    opacity: 0.8, 
    side: THREE.DoubleSide,
    blending: THREE.AdditiveBlending,
  });

  const thickness = 0.4;
  // Start the ring at the correct angle
  const geometry = new THREE.RingGeometry(
    radius - thickness / 2,
    radius + thickness / 2,
    64,
    1,
    angle - arcSpan / 2,
    arcSpan,
  );

  // Re-map UVs to rectangular projection for text wrapping
  const uv = geometry.attributes.uv;
  const pos = geometry.attributes.position;
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i);
    const y = pos.getY(i);
    const r = Math.sqrt(x * x + y * y);
    const a = Math.atan2(y, x);

    let relativeAngle = a - angle;
    while (relativeAngle > Math.PI) relativeAngle -= Math.PI * 2;
    while (relativeAngle < -Math.PI) relativeAngle += Math.PI * 2;

    // Restore flow (u) and invert radial (v) to fix character flip
    const u = relativeAngle / arcSpan + 0.5;
    const radial = (r - (radius - thickness / 2)) / thickness;
    const v = flipV ? radial : 1.0 - radial;
    uv.setXY(i, flip ? 1.0 - u : u, v);
  }

  const mesh = new THREE.Mesh(geometry, material);
  mesh.position.y = 0.04;
  mesh.rotation.x = -Math.PI / 2;
  mesh.renderOrder = 20;

  return mesh;
};

export const applyRingUv = (
  geometry: THREE.RingGeometry,
  innerRadius: number,
  outerRadius: number,
) => {
  const pos = geometry.attributes.position;
  const uv = geometry.attributes.uv;
  const radiusRange = Math.max(0.0001, outerRadius - innerRadius);
  for (let i = 0; i < pos.count; i += 1) {
    const x = pos.getX(i);
    const y = pos.getY(i);
    const angle = Math.atan2(y, x);
    const u = (angle + Math.PI) / (Math.PI * 2);
    const r = Math.sqrt(x * x + y * y);
    const v = (r - innerRadius) / radiusRange;
    uv.setXY(i, u, v);
  }
  uv.needsUpdate = true;
};

export const createRoomEnvironment = (scene: THREE.Scene) => {
  const roomGroup = new THREE.Group();
  
  // Tactical Grid Floor
  const gridHelper = new THREE.GridHelper(100, 100, 0x064e3b, 0x022c22);
  gridHelper.position.y = -4.0;
  roomGroup.add(gridHelper);

  // Dark Concrete Base
  const floorGeo = new THREE.PlaneGeometry(120, 120);
  const floorMat = new THREE.MeshStandardMaterial({
    color: 0x020617,
    roughness: 0.9,
    metalness: 0.1,
  });
  const floor = new THREE.Mesh(floorGeo, floorMat);
  floor.rotation.x = -Math.PI / 2;
  floor.position.y = -4.1;
  roomGroup.add(floor);

  // Scanning Lasers / Holographic projector beams
  const projectorLight = new THREE.SpotLight(0x10b981, 2.0, 50, 0.4, 0.5, 1);
  projectorLight.position.set(0, 15, 0);
  projectorLight.target.position.set(0, 0, 0);
  roomGroup.add(projectorLight);
  roomGroup.add(projectorLight.target);

  const ambient = new THREE.AmbientLight(0x022c22, 0.5);
  roomGroup.add(ambient);

  scene.add(roomGroup);
};
