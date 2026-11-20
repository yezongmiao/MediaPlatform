import { defineConfig } from 'vite';
import preact from '@preact/preset-vite';

const apiTarget = process.env.MEDIADESK_API ?? 'http://127.0.0.1:4317';

export default defineConfig({
  plugins: [preact()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': { target: apiTarget, changeOrigin: false },
      '/files': { target: apiTarget, changeOrigin: false },
    },
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    assetsDir: 'assets',
    cssCodeSplit: true,
    sourcemap: false,
    reportCompressedSize: true,
  },
});
