import fs from 'node:fs';
import path from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

const devApiTarget = process.env.YANSHI_DEV_API ?? 'http://127.0.0.1:8080';

export default defineConfig({
  base: '/ui/',
  server: {
    fs: {
      allow: ['..'],
    },
    proxy: {
      '/v1': {
        target: devApiTarget,
        changeOrigin: true,
      },
      '/health': {
        target: devApiTarget,
        changeOrigin: true,
      },
    },
  },
  build: {
    cssCodeSplit: false,
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        assetFileNames: 'assets/styles.css',
        entryFileNames: 'assets/app.js',
      },
    },
  },
  plugins: [
    react(),
    {
      // Mirror `dist/assets/*` into `dist/ui/assets/*` after Vite's CSS post-plugin
      // writes `styles.css`, so `dist/` works identically when embedded by
      // `yanshi-server` (`include_str!("../dist/assets/*")` served at `/ui/*`)
      // and when bundled directly by Tauri v2 (`frontendDist: "../dist"` at `/`).
      name: 'mirror-tauri-ui-prefix',
      enforce: 'post',
      writeBundle(options) {
        const outDir = options.dir ?? path.resolve(__dirname, 'dist');
        const srcAssets = path.join(outDir, 'assets');
        const dstAssets = path.join(outDir, 'ui', 'assets');
        if (!fs.existsSync(srcAssets)) {
          return;
        }
        fs.mkdirSync(dstAssets, { recursive: true });
        for (const entry of fs.readdirSync(srcAssets)) {
          const from = path.join(srcAssets, entry);
          const to = path.join(dstAssets, entry);
          if (fs.statSync(from).isFile()) {
            fs.copyFileSync(from, to);
          }
        }
      },
    },
  ],
});
