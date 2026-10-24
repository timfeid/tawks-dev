import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// Activities run with an opaque origin, so their module scripts are fetched
// with `Origin: null` and need CORS. Relative paths let one build be served
// from any versioned folder on apps.tawks.app.
export default defineConfig({
  base: './',
  plugins: [svelte(), tailwindcss()],
  server: { cors: true },
  preview: { cors: true },
});
