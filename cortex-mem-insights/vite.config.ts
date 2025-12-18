import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173, // 固定端口，避免冲突
		proxy: {
			'/api': {
				target: 'http://localhost:15173', // dev模式代理到API服务器
				changeOrigin: true,
				secure: false,
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
				target: 'http://localhost:15173',
				changeOrigin: true,
				secure: false
			}
		}
	}
});