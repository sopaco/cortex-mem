import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173, // 固定端口，避免冲突
		proxy: {
			'/api': {
				target: 'http://localhost:3001', // 代理到bun service (如需直连cortex-mem-service改为3000)
				changeOrigin: true,
				secure: false,
				// 注意：bun service的路由是/api/optimization，所以不移除/api前缀
				// 如果直连cortex-mem-service，需要添加 rewrite: (path) => path.replace(/^\/api/, '')
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