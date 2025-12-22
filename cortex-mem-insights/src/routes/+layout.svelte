<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import Navigation from '$lib/components/Navigation.svelte';
	import { t } from '$lib/i18n';

	// 获取当前路径，处理page.url可能为undefined的情况
	$: currentPath = $page.url?.pathname || '/';
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900">
	<!-- 导航栏 -->
	<Navigation {currentPath} />

	<!-- 主内容区域 -->
	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		<slot />
	</main>

	<!-- 页脚 -->
	<footer class="bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 mt-8">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
			<div class="flex justify-between items-center">
				<div class="text-sm text-gray-500 dark:text-gray-400">© 2025 {$t('common.appName')}</div>
				<div class="text-sm text-gray-500 dark:text-gray-400">
					{$t('common.language')}: {$t('common.language_this')}
				</div>
			</div>
		</div>
	</footer>
</div>

<style>
	/* 自定义样式 */
</style>
