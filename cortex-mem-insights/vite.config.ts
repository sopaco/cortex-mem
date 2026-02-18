import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
	plugins: [svelte()],
	server: {
		port: 8082,
		proxy: {
			'/api/v2': {
				target: 'http://localhost:8085',
				changeOrigin: true
			},
			'/health': {
				target: 'http://localhost:8085',
				changeOrigin: true
			}
		}
	}
});