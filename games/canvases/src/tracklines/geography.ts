import * as THREE from 'three';
import geography from './north-america.json';

export const MAP_WIDTH = 23;
export const MAP_HEIGHT = 16;

// Albers equal-area projection, the standard parallels of the contiguous US.
const radians = Math.PI / 180;
const parallel1 = 29.5 * radians;
const parallel2 = 45.5 * radians;
const n = (Math.sin(parallel1) + Math.sin(parallel2)) / 2;
const c = Math.cos(parallel1) ** 2 + 2 * n * Math.sin(parallel1);
const rho0 = Math.sqrt(c - 2 * n * Math.sin(38 * radians)) / n;

export function project(longitude: number, latitude: number): THREE.Vector3 {
  const rho = Math.sqrt(c - 2 * n * Math.sin(latitude * radians)) / n;
  const theta = n * (longitude + 96) * radians;
  return new THREE.Vector3(rho * Math.sin(theta) * 28, 0, (rho * Math.cos(theta) - rho0) * 28);
}

export function createMapTexture(): THREE.CanvasTexture {
  const canvas = document.createElement('canvas');
  canvas.width = 2300;
  canvas.height = 1600;
  const ctx = canvas.getContext('2d')!;
  const scale = canvas.width / MAP_WIDTH;
  const point = (lon: number, lat: number) => {
    const p = project(lon, lat);
    return [(p.x + MAP_WIDTH / 2) * scale, (p.z + MAP_HEIGHT / 2) * scale];
  };
  const path = (coordinates: number[][], close = false) => {
    coordinates.forEach(([lon, lat], i) => {
      const [x, y] = point(lon, lat);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    if (close) ctx.closePath();
  };
  const polygon = (rings: number[][][]) => {
    ctx.beginPath();
    rings.forEach((ring) => path(ring, true));
  };

  const ocean = ctx.createLinearGradient(0, 0, 0, canvas.height);
  ocean.addColorStop(0, '#275569');
  ocean.addColorStop(1, '#397e89');
  ctx.fillStyle = ocean;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // Graticule remains visible in the ocean and under the land's subtle tint.
  ctx.strokeStyle = '#66878b45';
  ctx.lineWidth = 1;
  for (let lon = -135; lon <= -60; lon += 5) {
    ctx.beginPath();
    path(Array.from({ length: 81 }, (_, i) => [lon, 15 + i / 2]));
    ctx.stroke();
  }
  for (let lat = 20; lat <= 55; lat += 5) {
    ctx.beginPath();
    path(Array.from({ length: 151 }, (_, i) => [-135 + i / 2, lat]));
    ctx.stroke();
  }

  for (const country of geography.land) {
    for (const rings of country.polygons) {
      polygon(rings);
      ctx.shadowColor = '#344b4d40';
      ctx.shadowBlur = 12;
      ctx.shadowOffsetY = 4;
      ctx.fillStyle = country.name === 'United States of America' ? '#a2b397' : '#829885';
      ctx.fill('evenodd');
      ctx.shadowBlur = 0;
      ctx.shadowOffsetY = 0;
      ctx.strokeStyle = '#6c8071';
      ctx.lineWidth = 2;
      ctx.stroke();

      ctx.save();
      ctx.clip('evenodd');
      // Soft relief tint gives the western ranges definition without hiding routes.
      for (const [lon, lat, radius, color] of [
        [-112, 42, 440, '#66764d35'], [-118, 37, 220, '#82765130'],
        [-109, 34, 270, '#bd945c30'], [-83, 37, 230, '#71896125'],
      ] as const) {
        const [x, y] = point(lon, lat);
        const tint = ctx.createRadialGradient(x, y, 0, x, y, radius);
        tint.addColorStop(0, color);
        tint.addColorStop(1, '#ffffff00');
        ctx.fillStyle = tint;
        ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
      }
      ctx.restore();
    }
  }
  ctx.strokeStyle = '#8a927967';
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  geography.borders.forEach((line) => path(line));
  ctx.stroke();
  ctx.strokeStyle = '#789fa9';
  ctx.lineWidth = 2;
  ctx.beginPath();
  geography.rivers.forEach((line) => path(line));
  ctx.stroke();
  for (const rings of geography.lakes) {
    polygon(rings);
    ctx.fillStyle = '#397084';
    ctx.fill('evenodd');
    ctx.strokeStyle = '#75969b';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  const label = (text: string, lon: number, lat: number, size: number, color: string, italic = false) => {
    const [x, y] = point(lon, lat);
    ctx.font = `${italic ? 'italic' : '500'} ${size}px Georgia, serif`;
    ctx.fillStyle = color;
    ctx.textAlign = 'center';
    ctx.fillText(text, x, y);
  };
  label('C A N A D A', -101, 50.5, 34, '#405d4a');
  label('M E X I C O', -107, 27, 32, '#4b6148');
  label('PACIFIC OCEAN', -126.2, 36, 25, '#8db5b7', true);
  label('ATLANTIC OCEAN', -70.3, 32.5, 25, '#8db5b7', true);
  label('Gulf of Mexico', -90, 25.5, 26, '#8db5b7', true);

  ctx.strokeStyle = '#eff3df65';
  ctx.lineWidth = 3;
  ctx.strokeRect(18, 18, canvas.width - 36, canvas.height - 36);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = 4;
  return texture;
}
