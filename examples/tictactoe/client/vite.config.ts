import { defineConfig } from 'vite';

// Activities run with an opaque origin, so their module scripts are fetched
// with `Origin: null` and need CORS.
export default defineConfig({
  base: './',
  server: { cors: true },
  preview: { cors: true },
});
