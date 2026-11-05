import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// The frame has an opaque origin, so module scripts load with `Origin: null`
// and need CORS. Relative paths let one build live in any versioned folder.
export default defineConfig({
  base: './',
  plugins: [svelte()],
  server: { cors: true },
  preview: { cors: true },
});
