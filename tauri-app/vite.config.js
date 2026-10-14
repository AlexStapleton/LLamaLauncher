import { defineConfig } from 'vite';

export default defineConfig({
  root: './src/frontend',
  server: {
    port: 14200,
    strictPort: true,
  },
  build: {
    target: 'esnext',
    outDir: '../../dist',
    emptyOutDir: true,
    rollupOptions: {
      external: ['tauri-plugin-*'],
    },
  },
  resolve: {
    alias: {
      '@': '/src',
    },
  },
});
