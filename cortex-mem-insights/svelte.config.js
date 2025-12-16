import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// 使用 Static adapter 生成纯静态文件,由Elysia托管
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html', // SPA模式,所有路由返回index.html
			precompress: false,
			strict: false // 关闭strict模式,允许动态路由在客户端渲染
		})
	}
};

export default config;