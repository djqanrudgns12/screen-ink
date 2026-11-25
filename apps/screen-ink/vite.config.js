import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

// Tauri 개발 서버(tauri.conf.json devUrl과 같은 포트). 저장소 루트의 docs/shortcuts JSON을 읽으므로 fs.allow를 넓힌다.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, fs: { allow: ['../..'] } },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true },
});
