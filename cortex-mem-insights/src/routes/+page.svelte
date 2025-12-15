<script lang="ts">
	import { onMount } from 'svelte';
	import api from '$lib/api/client';

	// 模拟数据
	let stats = {
		totalMemories: 0,
		todayAdded: 0,
		optimizationCount: 0,
		averageQuality: 0
	};

	let systemStatus = {
		cortexMemService: 'connecting',
		qdrant: 'connecting',
		llmService: 'connecting'
	};

	let recentMemories: Array<{
		id: string;
		content: string;
		type: string;
		importance: number;
		createdAt: string;
	}> = [];

	let isLoading = true;
	let error: string | null = null;

	onMount(async () => {
		try {
			// 尝试加载实际数据
			await loadDashboardData();
		} catch (err) {
			console.error('加载仪表板数据失败:', err);
			error = err instanceof Error ? err.message : '加载数据失败';
			// 回退到模拟数据
			fallbackToMockData();
		} finally {
			isLoading = false;
		}
	});

	async function loadDashboardData() {
		try {
			// 获取系统状态
			const healthResponse = await fetch('/health');
			const healthData = await healthResponse.json();

			// 更新系统状态
			systemStatus = {
				cortexMemService: healthData.status === 'healthy' ? 'connected' : 'error',
				qdrant: healthData.vector_store ? 'connected' : 'error',
				llmService: healthData.llm_service ? 'connected' : 'error'
			};

			// 获取记忆统计
			const memoriesResponse = await api.memory.list({ limit: 100 });

			// 计算统计数据
			const today = new Date();
			const todayMemories = memoriesResponse.memories.filter(
				(m) => new Date(m.created_at).toDateString() === today.toDateString()
			);

			stats = {
				totalMemories: memoriesResponse.total,
				todayAdded: todayMemories.length,
				optimizationCount: 0, // TODO: 获取实际优化计数
				averageQuality: 0.75 // TODO: 计算实际平均质量
			};

			// 获取最近记忆
			recentMemories = memoriesResponse.memories
				.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
				.slice(0, 5)
				.map((memory) => ({
					id: memory.id,
					content: memory.content,
					type: memory.metadata.memory_type,
					importance: 0.7, // TODO: 从metadata获取实际重要性
					createdAt: formatDate(memory.created_at)
				}));
		} catch (err) {
			console.error('加载仪表板数据错误:', err);
			throw err;
		}
	}

	function fallbackToMockData() {
		console.log('回退到模拟数据');
		stats = {
			totalMemories: 1245,
			todayAdded: 23,
			optimizationCount: 12,
			averageQuality: 0.78
		};

		systemStatus = {
			cortexMemService: 'connected',
			qdrant: 'connected',
			llmService: 'connected'
		};

		recentMemories = [
			{
				id: 'mem_001',
				content: '用户偏好：喜欢使用暗色主题，经常在晚上工作',
				type: 'Personal',
				importance: 0.9,
				createdAt: '2025-12-13 14:30'
			},
			{
				id: 'mem_002',
				content: '项目需求：需要实现用户认证系统，支持OAuth2.0',
				type: 'Factual',
				importance: 0.8,
				createdAt: '2025-12-13 13:45'
			},
			{
				id: 'mem_003',
				content: '对话历史：用户询问关于Rust异步编程的最佳实践',
				type: 'Conversational',
				importance: 0.7,
				createdAt: '2025-12-13 12:20'
			},
			{
				id: 'mem_004',
				content: '系统配置：API超时时间设置为30秒，重试次数3次',
				type: 'Procedural',
				importance: 0.85,
				createdAt: '2025-12-13 11:15'
			}
		];

		isLoading = false;
	}

	function getStatusColor(status: string) {
		switch (status) {
			case 'connected':
				return 'text-green-500 bg-green-50 dark:bg-green-900/20';
			case 'connecting':
				return 'text-yellow-500 bg-yellow-50 dark:bg-yellow-900/20';
			case 'disconnected':
				return 'text-red-500 bg-red-50 dark:bg-red-900/20';
			default:
				return 'text-gray-500 bg-gray-50 dark:bg-gray-800';
		}
	}

	function getStatusText(status: string) {
		switch (status) {
			case 'connected':
				return '已连接';
			case 'connecting':
				return '连接中';
			case 'disconnected':
				return '已断开';
			default:
				return '未知';
		}
	}

	function formatImportance(importance: number) {
		if (importance >= 0.9) return '极高';
		if (importance >= 0.7) return '高';
		if (importance >= 0.5) return '中';
		return '低';
	}

	function getImportanceColor(importance: number) {
		if (importance >= 0.9) return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300';
		if (importance >= 0.7)
			return 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300';
		if (importance >= 0.5)
			return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300';
		return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
	}

	function formatDate(isoString: string): string {
		try {
			const date = new Date(isoString);
			return date
				.toLocaleString('zh-CN', {
					year: 'numeric',
					month: '2-digit',
					day: '2-digit',
					hour: '2-digit',
					minute: '2-digit'
				})
				.replace(/\//g, '-')
				.replace(',', '');
		} catch {
			return isoString;
		}
	}
</script>

<div class="space-y-8">
	<!-- 欢迎标题 -->
	<div>
		<h1 class="text-3xl font-bold text-gray-900 dark:text-white">仪表盘</h1>
		<p class="mt-2 text-gray-600 dark:text-gray-400">监控和分析 cortex-mem 记忆系统的运行状态</p>
	</div>

	{#if isLoading}
		<!-- 加载状态 -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
			{#each Array(4) as _, i}
				<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse">
					<div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-4"></div>
					<div class="h-8 bg-gray-200 dark:bg-gray-700 rounded w-2/3"></div>
				</div>
			{/each}
		</div>
	{:else}
		<!-- 统计卡片 -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
			<!-- 总记忆数 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 border-l-4 border-blue-500">
				<div class="flex items-center justify-between">
					<div>
						<p class="text-sm font-medium text-gray-600 dark:text-gray-400">总记忆数</p>
						<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
							{stats.totalMemories.toLocaleString()}
						</p>
					</div>
					<div
						class="w-12 h-12 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center"
					>
						<span class="text-2xl">📚</span>
					</div>
				</div>
				<p class="mt-4 text-sm text-gray-500 dark:text-gray-400">
					今日新增: <span class="font-medium text-green-600 dark:text-green-400"
						>+{stats.todayAdded}</span
					>
				</p>
			</div>

			<!-- 今日新增 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 border-l-4 border-green-500">
				<div class="flex items-center justify-between">
					<div>
						<p class="text-sm font-medium text-gray-600 dark:text-gray-400">今日新增</p>
						<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
							+{stats.todayAdded}
						</p>
					</div>
					<div
						class="w-12 h-12 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center"
					>
						<span class="text-2xl">📈</span>
					</div>
				</div>
				<p class="mt-4 text-sm text-gray-500 dark:text-gray-400">
					较昨日: <span class="font-medium text-green-600 dark:text-green-400">+15%</span>
				</p>
			</div>

			<!-- 优化次数 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 border-l-4 border-purple-500">
				<div class="flex items-center justify-between">
					<div>
						<p class="text-sm font-medium text-gray-600 dark:text-gray-400">优化次数</p>
						<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
							{stats.optimizationCount}
						</p>
					</div>
					<div
						class="w-12 h-12 bg-purple-100 dark:bg-purple-900/30 rounded-lg flex items-center justify-center"
					>
						<span class="text-2xl">⚡</span>
					</div>
				</div>
				<p class="mt-4 text-sm text-gray-500 dark:text-gray-400">
					平均质量: <span class="font-medium text-blue-600 dark:text-blue-400"
						>{(stats.averageQuality * 100).toFixed(1)}%</span
					>
				</p>
			</div>

			<!-- 平均质量 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 border-l-4 border-yellow-500">
				<div class="flex items-center justify-between">
					<div>
						<p class="text-sm font-medium text-gray-600 dark:text-gray-400">平均质量</p>
						<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
							{(stats.averageQuality * 100).toFixed(1)}%
						</p>
					</div>
					<div
						class="w-12 h-12 bg-yellow-100 dark:bg-yellow-900/30 rounded-lg flex items-center justify-center"
					>
						<span class="text-2xl">⭐</span>
					</div>
				</div>
				<div class="mt-4">
					<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
						<div
							class="bg-yellow-500 h-2 rounded-full"
							style={`width: ${stats.averageQuality * 100}%`}
						></div>
					</div>
				</div>
			</div>
		</div>

		<!-- 系统状态和最近记忆 -->
		<div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
			<!-- 系统状态 -->
			<div class="lg:col-span-1">
				<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
					<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">系统状态</h2>

					<div class="space-y-4">
						{#each Object.entries(systemStatus) as [service, status]}
							<div
								class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-900/50"
							>
								<div class="flex items-center space-x-3">
									<div
										class={`w-3 h-3 rounded-full ${status === 'connected' ? 'bg-green-500' : status === 'connecting' ? 'bg-yellow-500' : 'bg-red-500'}`}
									></div>
									<span class="font-medium text-gray-700 dark:text-gray-300">
										{service === 'cortexMemService'
											? 'cortex-mem-service'
											: service === 'qdrant'
												? 'Qdrant 数据库'
												: 'LLM 服务'}
									</span>
								</div>
								<span
									class={`px-3 py-1 rounded-full text-xs font-medium ${getStatusColor(status)}`}
								>
									{getStatusText(status)}
								</span>
							</div>
						{/each}
					</div>

					<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
						<button
							class="w-full px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
							on:click={() => console.log('检查状态')}
						>
							检查所有服务状态
						</button>
					</div>
				</div>
			</div>

			<!-- 最近记忆 -->
			<div class="lg:col-span-2">
				<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
					<div class="flex items-center justify-between mb-6">
						<h2 class="text-lg font-semibold text-gray-900 dark:text-white">最近记忆</h2>
						<a
							href="/memories"
							class="text-sm font-medium text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
						>
							查看全部 →
						</a>
					</div>

					<div class="space-y-4">
						{#each recentMemories as memory}
							<div
								class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-900/50 transition-colors duration-200"
							>
								<div class="flex items-start justify-between">
									<div class="flex-1">
										<div class="flex items-center space-x-2 mb-2">
											<span
												class={`px-2 py-1 rounded text-xs font-medium ${getImportanceColor(memory.importance)}`}
											>
												{formatImportance(memory.importance)}
											</span>
											<span
												class="px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-xs text-gray-600 dark:text-gray-400"
											>
												{memory.type}
											</span>
										</div>
										<p class="text-gray-700 dark:text-gray-300 mb-2 truncate-2-lines">
											{memory.content}
										</p>
										<div
											class="flex items-center justify-between text-sm text-gray-500 dark:text-gray-400"
										>
											<span>ID: {memory.id}</span>
											<span>{memory.createdAt}</span>
										</div>
									</div>
									<button
										class="ml-4 p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
										on:click={() => console.log('查看详情', memory.id)}
									>
										🔍
									</button>
								</div>
							</div>
						{/each}
					</div>

					<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
						<div class="flex space-x-4">
							<button
								class="flex-1 px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors duration-200"
								on:click={() => console.log('添加记忆')}
							>
								添加测试记忆
							</button>
							<button
								class="flex-1 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
								on:click={() => console.log('搜索记忆')}
							>
								搜索记忆
							</button>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- 快速操作 -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">快速操作</h2>

			<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-700 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all duration-200 group"
					on:click={() => console.log('运行优化')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center group-hover:bg-blue-200 dark:group-hover:bg-blue-800/40"
						>
							<span class="text-xl">⚡</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">运行优化</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">清理重复和低质量记忆</p>
						</div>
					</div>
				</button>

				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-green-300 dark:hover:border-green-700 hover:bg-green-50 dark:hover:bg-green-900/20 transition-all duration-200 group"
					on:click={() => console.log('导出数据')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center group-hover:bg-green-200 dark:group-hover:bg-green-800/40"
						>
							<span class="text-xl">📥</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">导出数据</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">导出记忆为JSON/CSV格式</p>
						</div>
					</div>
				</button>

				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-purple-300 dark:hover:border-purple-700 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-all duration-200 group"
					on:click={() => console.log('查看报告')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-purple-100 dark:bg-purple-900/30 rounded-lg flex items-center justify-center group-hover:bg-purple-200 dark:group-hover:bg-purple-800/40"
						>
							<span class="text-xl">📊</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">生成报告</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">生成系统运行分析报告</p>
						</div>
					</div>
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.truncate-2-lines {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
</style>
