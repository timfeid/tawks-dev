import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

const here = (path: string) => fileURLToPath(new URL(path, import.meta.url));

// The screenshot page (index.html): the kit renderer and HUD drawing recorded
// scene frames. `vite --config render/vite.config.ts` also serves dev.html, a
// local round of Thin Ice for working on the renderer.
export default defineConfig({
  root: here('.'),
  base: './',
  plugins: [svelte(), tailwindcss()],
  resolve: { alias: { '@tawks/made-kit': here('../../src/index.js') } },
  build: { outDir: 'dist', emptyOutDir: true, chunkSizeWarningLimit: 2000 },
});
