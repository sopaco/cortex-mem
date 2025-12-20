<script lang="ts">
	import { t } from '$lib/i18n';
	import LanguageSwitcher from './LanguageSwitcher.svelte';

	export let currentPath = '/';

	const navItems = [
		{ path: '/', label: 'navigation.dashboard', icon: '🏠' },
		{ path: '/memories', label: 'navigation.memories', icon: '💭' },
		{ path: '/analytics', label: 'navigation.analytics', icon: '📊' },
		{ path: '/monitor', label: 'navigation.monitor', icon: '🔍' },
		{ path: '/optimization', label: 'navigation.optimization', icon: '⚡' }
	];
</script>

<nav class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
	<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
		<div class="flex justify-between h-16">
			<div class="flex">
				<!-- Logo and App Name -->
				<div class="flex-shrink-0 flex items-center">
					<div class="flex items-center space-x-3">
						<img src="/favicon.png" alt="Cortex Memory" class="w-8 h-8 rounded-lg" />
						<span class="text-xl font-bold text-gray-900 dark:text-white">
							{$t('common.appName')}
						</span>
					</div>
				</div>

				<!-- Navigation Links -->
				<div class="hidden sm:ml-6 sm:flex sm:space-x-8">
					{#each navItems as item}
						<a
							href={item.path}
							class={`inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium ${
								currentPath === item.path
									? 'border-blue-500 text-gray-900 dark:text-white'
									: 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 dark:hover:border-gray-600 hover:text-gray-700 dark:hover:text-gray-300'
							}`}
						>
							<span class="mr-2">{item.icon}</span>
							{$t(item.label)}
						</a>
					{/each}
				</div>
			</div>

			<!-- Right side: Language switcher and user menu -->
			<div class="flex items-center space-x-4">
				<LanguageSwitcher />

				<!-- User menu placeholder -->
				<div class="relative">
					<button
						type="button"
						class="flex items-center space-x-2 p-1 rounded-full text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
						aria-label="User menu"
					>
						<div
							class="w-8 h-8 bg-gray-300 dark:bg-gray-600 rounded-full flex items-center justify-center"
						>
							<span class="text-sm font-medium text-gray-700 dark:text-gray-300">U</span>
						</div>
					</button>
				</div>
			</div>
		</div>

		<!-- Mobile navigation -->
		<div class="sm:hidden">
			<div class="pt-2 pb-3 space-y-1">
				{#each navItems as item}
					<a
						href={item.path}
						class={`block pl-3 pr-4 py-2 border-l-4 text-base font-medium ${
							currentPath === item.path
								? 'bg-blue-50 dark:bg-blue-900/30 border-blue-500 text-blue-700 dark:text-blue-400'
								: 'border-transparent text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700 hover:border-gray-300 dark:hover:border-gray-600 hover:text-gray-700 dark:hover:text-gray-300'
						}`}
					>
						<span class="mr-2">{item.icon}</span>
						{$t(item.label)}
					</a>
				{/each}
			</div>
		</div>
	</div>
</nav>

<style>
	nav {
		font-family:
			-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
	}
</style>
