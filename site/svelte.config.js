import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({ fallback: '404.html' }),
    alias: {
      '@tawks/workflow-ui': '../web/workflow-ui/src/lib',
      '@tawks/ui': '../web/ui/src/lib',
      '@tawks/editor': '../web/editor/src/lib',
    },
    prerender: { entries: ['*'] },
  },
};
