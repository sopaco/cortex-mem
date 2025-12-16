import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173, // 固定端口，避免冲突
		proxy: {
			'/api': {
				target: 'http://localhost:3000',
				changeOrigin: true,
				secure: false,
				rewrite: (path) => path.replace(/^\/api/, ''), // 移除/api前缀，匹配cortex-mem-service的端点
				configure: (proxy, _options) => {
					proxy.on('error', (err, _req, _res) => {
						console.log('代理错误:', err);
					});
					proxy.on('proxyReq', (proxyReq, req, _res) => {
						console.log('代理请求:', req.method, req.url, '→', proxyReq.path);
					});
				}
			},
			'/health': {
				target: 'http://localhost:3000',
				changeOrigin: true,
				secure: false
			}
		}
	}
});