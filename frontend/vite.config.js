import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

export default defineConfig({
  plugins: [svelte({ preprocess: vitePreprocess() })],
  resolve: {
    alias: {
      $lib: resolve('./src/lib'),
      $app: resolve('./src/$app')
    }
  },
  server: {
    port: 5173
  },
  test: {
    environment: 'jsdom',
    globals: true,
    transformMode: { web: [/\.svelte$/] },
    setupFiles: './vitest.setup.ts'
  }
});
