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
    lib: {
      entry: 'src/index.js',
      name: 'MaplibreGlGeoToolbox',
      formats: ['es'],
      fileName: 'index',
    },
    rollupOptions: {
      external: ['maplibre-gl'],
    },
  },
  optimizeDeps: {
    exclude: ['geo-wasm'],
  },
});
