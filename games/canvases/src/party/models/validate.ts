import { MODEL_LIMITS, type ModelClipMap, type ModelReport } from './types';

/** A quick bounded check before GLTFLoader is allowed to parse untrusted
 * files. The API repeats these checks and validates all binary references. */
export function inspectGlb(data: ArrayBuffer, requested?: ModelClipMap): ModelReport {
  const fail = (message: string): never => { throw new Error(message); };
  if (data.byteLength > MODEL_LIMITS.bytes) fail('Models can be at most 10 MB.');
  if (data.byteLength < 28) fail('Choose a complete binary glTF (.glb) file.');
  const view = new DataView(data);
  if (view.getUint32(0, true) !== 0x46546c67 || view.getUint32(4, true) !== 2 || view.getUint32(8, true) !== data.byteLength) fail('Choose a complete glTF 2 binary (.glb) file.');
  const jsonLength = view.getUint32(12, true);
  if (jsonLength > 512 * 1024 || jsonLength % 4 || 28 + jsonLength > data.byteLength || view.getUint32(16, true) !== 0x4e4f534a) fail('The GLB JSON chunk is invalid or too large.');
  const end = 20 + jsonLength;
  if (view.getUint32(end + 4, true) !== 0x004e4942 || end + 8 + view.getUint32(end, true) !== data.byteLength) fail('Embed all model data in one GLB binary chunk.');
  let doc: any;
  try { doc = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(new Uint8Array(data, 20, jsonLength))); } catch { fail('The GLB JSON could not be read.'); }
  if (doc?.asset?.version !== '2.0') fail('Only glTF 2 models are supported.');
  const allUris = (value: unknown, depth = 0): boolean => {
    if (depth > 64) fail('The model metadata is nested too deeply.');
    return !!value && typeof value === 'object' && Object.entries(value).some(([key, v]) => ['uri', '__proto__', 'constructor', 'prototype'].includes(key) || allUris(v, depth + 1));
  };
  if (allUris(doc)) fail('Embed all textures and buffers. External URLs and files are not supported.');
  const extensions = new Set(['KHR_materials_unlit', 'KHR_materials_ior', 'KHR_materials_specular', 'KHR_materials_clearcoat', 'KHR_materials_emissive_strength', 'KHR_texture_transform']);
  if ([...(doc.extensionsUsed ?? []), ...(doc.extensionsRequired ?? [])].some((extension) => !extensions.has(extension))) fail('Export standard glTF materials without compression or custom extensions.');
  if (!Array.isArray(doc.buffers) || doc.buffers.length !== 1 || !Number.isSafeInteger(doc.buffers[0].byteLength) || doc.buffers[0].byteLength > data.byteLength - end - 8) fail('The embedded buffer is invalid.');
  const binary = end + 8;
  const views: any[] = doc.bufferViews ?? [];
  const accessors: any[] = doc.accessors ?? [];
  if (views.length > 512 || accessors.length > 512) fail('The model contains too many buffer views or accessors.');
  const integer = (n: unknown): n is number => Number.isSafeInteger(n) && (n as number) >= 0;
  for (const buffer of views) if ((buffer.buffer ?? 0) !== 0 || !integer(buffer.byteOffset ?? 0) || !integer(buffer.byteLength) || buffer.byteLength === 0 || (buffer.byteOffset ?? 0) + buffer.byteLength > doc.buffers[0].byteLength) fail('A buffer view points outside the model.');
  const widths: Record<number, number> = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4};
  const elements: Record<string, number> = {SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16};
  let accessorBytes = 0;
  for (const accessor of accessors) {
    const buffer = views[accessor.bufferView];
    const width = widths[accessor.componentType];
    const size = elements[accessor.type] * width;
    const stride = buffer?.byteStride ?? size;
    if (accessor.sparse || !buffer || !size || !integer(accessor.count) || accessor.count < 1 || accessor.count > 300_000 || !integer(accessor.byteOffset ?? 0) || stride < size || stride > 252 || (accessor.byteOffset ?? 0) % width || buffer.byteStride && stride % 4 || (accessor.byteOffset ?? 0) + (accessor.count - 1) * stride + size > buffer.byteLength) fail('A model accessor has invalid bounds or layout.');
    accessorBytes += accessor.count * size;
    if (accessorBytes > 32 * 1024 * 1024) fail('The model contains too much decoded accessor data.');
    if (accessor.componentType === 5126) {
      const start = binary + (buffer.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
      for (let i = 0; i < accessor.count; i++) for (let j = 0; j < elements[accessor.type]; j++) {
        const value = view.getFloat32(start + i * stride + j * 4, true);
        if (!Number.isFinite(value) || Math.abs(value) > 1_000_000) fail('The model contains invalid numeric values.');
      }
    }
  }
  const meshes: any[] = doc.meshes ?? [];
  const nodes: any[] = doc.nodes ?? [];
  if (!meshes.length || meshes.length > 64 || !nodes.length || nodes.length > MODEL_LIMITS.nodes) fail('Use at most 64 meshes and 128 nodes, including bones.');
  let parts = 0;
  const counts = meshes.map((mesh) => {
    let vertices = 0, triangles = 0;
    for (const primitive of mesh.primitives ?? []) {
      parts++;
      const position = accessors[primitive.attributes?.POSITION];
      if ((primitive.mode ?? 4) !== 4 || primitive.targets?.length || !position || position.type !== 'VEC3' || position.componentType !== 5126) fail('Export triangle meshes with standard floating point positions and no morph targets.');
      const indices = primitive.indices === undefined ? position.count : accessors[primitive.indices]?.count;
      if (!integer(indices) || indices % 3) fail('A triangle or index accessor is invalid.');
      vertices += position.count; triangles += indices / 3;
    }
    return { vertices, triangles };
  });
  if (parts > 96) fail('Use at most 96 mesh parts.');
  let vertices = 0, triangles = 0, rigged = false;
  const parents = new Set<number>();
  for (const node of nodes) {
    if (node.mesh !== undefined) { const count = counts[node.mesh]; if (!count) fail('A node references a missing mesh.'); vertices += count.vertices; triangles += count.triangles; }
    if (node.skin !== undefined) { const skin = doc.skins?.[node.skin]; if (!skin?.joints?.length || skin.joints.length > MODEL_LIMITS.bones || skin.joints.some((joint: number) => !nodes[joint])) fail('A skeleton has missing or too many bones (maximum 96).'); rigged = node.mesh !== undefined || rigged; }
    for (const key of ['matrix', 'translation', 'rotation', 'scale']) if (node[key] !== undefined && (!Array.isArray(node[key]) || node[key].length !== (key === 'matrix' ? 16 : key === 'rotation' ? 4 : 3) || node[key].some((n: unknown) => typeof n !== 'number' || !Number.isFinite(n) || Math.abs(n) > 100_000))) fail('The model contains invalid transforms.');
    for (const child of node.children ?? []) { if (!integer(child) || !nodes[child] || parents.has(child)) fail('The model contains an invalid hierarchy.'); parents.add(child); }
  }
  const visit = (index: number, path: Set<number>) => { if (path.has(index) || path.size > 32) fail('The model hierarchy is cyclic or too deep.'); const next = new Set(path).add(index); for (const child of nodes[index].children ?? []) visit(child, next); };
  nodes.forEach((_, index) => visit(index, new Set()));
  if (doc.scenes?.length !== 1 || !doc.scenes[0]?.nodes?.length || (doc.scene ?? 0) !== 0 || doc.scenes[0].nodes.some((n: number) => !nodes[n])) fail('Export one scene with valid root nodes per GLB model.');
  if (!vertices || vertices > MODEL_LIMITS.vertices || triangles > MODEL_LIMITS.triangles) fail('Use at most 60,000 vertices and 30,000 triangles across all model instances.');
  const images: any[] = doc.images ?? [];
  if (images.length > MODEL_LIMITS.textures || (doc.textures?.length ?? 0) > MODEL_LIMITS.textures || (doc.materials?.length ?? 0) > 32) fail('Use at most eight textures and 32 materials.');
  let pixels = 0;
  for (const image of images) {
    const buffer = views[image.bufferView];
    if (!buffer || !['image/png', 'image/jpeg'].includes(image.mimeType)) fail('Use embedded PNG or JPEG textures.');
    if (image.mimeType === 'image/png') {
      const offset = binary + (buffer.byteOffset ?? 0);
      if (buffer.byteLength < 24 || view.getUint32(offset) !== 0x89504e47 || view.getUint32(offset + 4) !== 0x0d0a1a0a || !view.getUint32(offset + 16) || !view.getUint32(offset + 20) || view.getUint32(offset + 16) > MODEL_LIMITS.textureSize || view.getUint32(offset + 20) > MODEL_LIMITS.textureSize) fail('Textures must be valid PNG or JPEG images of at most 2048 × 2048.');
      pixels += view.getUint32(offset + 16) * view.getUint32(offset + 20);
    } else {
      const start = binary + (buffer.byteOffset ?? 0), end = start + buffer.byteLength;
      if (buffer.byteLength < 4 || view.getUint16(start) !== 0xffd8) fail('The embedded JPEG is invalid.');
      let offset = start + 2, found = false;
      while (offset + 4 <= end) {
        if (view.getUint8(offset) !== 0xff) fail('The embedded JPEG header is invalid.');
        while (offset < end && view.getUint8(offset) === 0xff) offset++;
        if (offset + 3 > end) break;
        const marker = view.getUint8(offset++);
        if (marker === 0xda || marker === 0xd9) break;
        const length = view.getUint16(offset);
        if (length < 2 || offset + length > end) fail('The embedded JPEG header is invalid.');
        if ([0xc0,0xc1,0xc2,0xc3,0xc5,0xc6,0xc7,0xc9,0xca,0xcb,0xcd,0xce,0xcf].includes(marker)) {
          if (length < 8) fail('The embedded JPEG header is invalid.');
          const height = view.getUint16(offset + 3), width = view.getUint16(offset + 5);
          if (!width || !height || width > MODEL_LIMITS.textureSize || height > MODEL_LIMITS.textureSize) fail('Textures can be at most 2048 × 2048 pixels.');
          pixels += width * height;
          found = true; break;
        }
        offset += length;
      }
      if (!found) fail('The embedded JPEG has no readable dimensions.');
    }
    if (pixels > 8 * 1024 * 1024) fail('Combined model textures can be at most eight megapixels.');
  }
  let keyframes = 0;
  const animations = (doc.animations ?? []).map((animation: any, index: number) => {
    const name = animation.name || `animation_${index}`;
    let duration = 0;
    for (const sampler of animation.samplers ?? []) {
      const accessor = accessors[sampler.input];
      const buffer = views[accessor?.bufferView];
      if (!buffer || accessor.componentType !== 5126 || accessor.type !== 'SCALAR' || accessor.count > 100_000) fail('Animation keyframes are invalid or too large.');
      const offset = binary + (buffer.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
      const last = view.getFloat32(offset + (accessor.count - 1) * (buffer.byteStride ?? 4), true);
      if (!Number.isFinite(last) || last < 0 || last > 120) fail('Keep animation clips under 120 seconds.');
      duration = Math.max(duration, last);
    }
    for (const channel of animation.channels ?? []) {
      const sampler = animation.samplers?.[channel.sampler];
      const input = accessors[sampler?.input], output = accessors[sampler?.output];
      const path = channel.target?.path;
      if (!sampler || !nodes[channel.target?.node] || !['translation', 'rotation', 'scale'].includes(path) || !input || !output) fail('The animation contains an invalid sampler or target.');
      const multiple = sampler.interpolation === 'CUBICSPLINE' ? 3 : 1;
      if (!['LINEAR', 'STEP', 'CUBICSPLINE'].includes(sampler.interpolation ?? 'LINEAR') || output.type !== (path === 'rotation' ? 'VEC4' : 'VEC3') || output.componentType !== 5126 || output.count !== input.count * multiple) fail('Animation values do not match their keyframes.');
      keyframes += input.count;
      if (keyframes > 100_000) fail('Use at most 100,000 total animation keyframes.');
      const buffer = views[input.bufferView];
      const start = binary + (buffer.byteOffset ?? 0) + (input.byteOffset ?? 0), stride = buffer.byteStride ?? 4;
      let previous = -1;
      for (let i = 0; i < input.count; i++) {
        const time = view.getFloat32(start + i * stride, true);
        if (time < 0 || time <= previous) fail('Animation keyframes must use increasing nonnegative times.');
        previous = time;
      }
    }
    return {name, duration};
  }).filter((animation: {duration: number}) => animation.duration > 0);
  if (animations.length > 24 || new Set(animations.map((a: {name: string}) => a.name)).size !== animations.length) fail('Use at most 24 uniquely named animation clips.');
  const find = (terms: string[]) => animations.find((animation: {name: string}) => animation.name.toLowerCase().split(/[^a-z0-9]+/).some((word) => terms.includes(word)))?.name ?? null;
  const clip_map = requested ?? {idle: find(['idle', 'standing']), ride: find(['walk', 'walking', 'run', 'running', 'locomotion']), cheer: find(['cheer', 'victory']), sad: find(['sad', 'defeat']), fall: find(['fall', 'death'])};
  for (const name of Object.values(clip_map)) if (name && !animations.some((animation: {name: string}) => animation.name === name)) fail('The selected animation clip is missing.');
  return {bytes: data.byteLength, vertices, triangles, meshes: meshes.length, textures: images.length, rigged, playable: rigged && !!clip_map.idle && !!clip_map.ride && clip_map.idle !== clip_map.ride, animations, clip_map, kind: 'custom_model'};
}
