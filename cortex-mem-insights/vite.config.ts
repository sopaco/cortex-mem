import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: 'http://localhost:3000',
				changeOrigin: true,
				secure: false,
				rewrite: (path) => path.replace(/^\/api/, '') // 移除/api前缀，匹配cortex-mem-service的端点
			},
			'/health': {
				target: 'http://localhost:3000',
				changeOrigin: true,
				secure: false
			}
		}
	}
});