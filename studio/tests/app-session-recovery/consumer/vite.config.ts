import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
export default defineConfig({ base: '/rom-studio/', plugins: [svelte(), tailwindcss()], resolve: { dedupe: ['svelte'] }, build: { rollupOptions: { input: ['index.html','startup.html'] } } });
