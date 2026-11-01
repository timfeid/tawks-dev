import { defineConfig } from 'vite';

/** The board preview harness; see dev/preview.ts. */
export default defineConfig({
  root: __dirname,
  server: { port: 5195, strictPort: true },
  json: { stringify: false },
});
