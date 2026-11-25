// ink-ui 미리보기(브라우저). FakeInkClient로 S01~S06 화면과 부품 판을 띄운다.
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  server: { port: 5180, strictPort: true, fs: { allow: ['../..'] } },
});
