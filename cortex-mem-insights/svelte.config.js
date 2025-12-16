import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// 使用 Node adapter 以便部署
		adapter: adapter({
			out: 'build',
			precompress: false,
			envPrefix: ''
		})
	}
};

export default config;