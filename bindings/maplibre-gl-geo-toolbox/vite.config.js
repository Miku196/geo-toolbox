import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  publicDir: false,
  server: {
    port: 5173,
    open: '/examples/index.html',
    fs: {
      // Allow referencing the WASM pkg dir one level up
      allow: ['..', '../..'],
    },
  },
  build: {
    target: 'esnext',
  },
  optimizeDeps: {
    exclude: ['geo-wasm'],
  },
});
