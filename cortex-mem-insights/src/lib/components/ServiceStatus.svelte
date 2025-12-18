<script lang="ts">
	import { onMount } from 'svelte';

	// 服务状态类型定义
	export type ServiceStatus = {
		status: 'connected' | 'connecting' | 'detecting' | 'disconnected' | 'error';
		latency: number;
		version?: string;
		lastCheck: string;
		collectionCount?: number;
		provider?: string;
		model?: string;
		completionModel?: {
			available: boolean;
			latency: number;
			error: string | null;
		};
		embeddingModel?: {
			available: boolean;
			latency: number;
			error: string | null;
		};
	};

	export type SystemStatus = {
		cortexMemService: ServiceStatus;
		qdrant: ServiceStatus;
		llmService: ServiceStatus;
	};

	// Props
	export let systemStatus: SystemStatus | null = null;
	export let title: string = '服务状态';
	export let showRefreshButton: boolean = true;
	export let autoDetect: boolean = true;

	// 事件派发
	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

	// 状态变量
	let isDetectingServices = false;
	let localSystemStatus: SystemStatus = {
		cortexMemService: { status: 'connecting', latency: 0, version: '1.0.0', lastCheck: '' },
		qdrant: {
			status: 'connecting',
			latency: 0,
			version: '1.7.0',
			collectionCount: 0,
			lastCheck: ''
		},
		llmService: {
			status: 'connecting',
			latency: 0,
			provider: 'Unknown',
			model: 'Unknown',
			lastCheck: '',
			completionModel: {
				available: false,
				latency: 0,
				error: null as string | null
			},
			embeddingModel: {
				available: false,
				latency: 0,
				error: null as string | null
			}
		}
	};
	let isRefreshing = false;

	// 同步props到本地状态，确保深拷贝
	$: if (systemStatus) {
		localSystemStatus = JSON.parse(JSON.stringify(systemStatus));
	} else {
		// 如果没有传入systemStatus，保持默认值
	}

	// 独立检测各个服务状态（与监控页面相同的逻辑）
	async function detectIndividualServices(timestamp: string) {
		const mainService: ServiceStatus = { status: 'detecting', latency: 0, lastCheck: timestamp };
		const vectorStore: ServiceStatus = { status: 'detecting', latency: 0, lastCheck: timestamp };
		const llmService: ServiceStatus = { status: 'detecting', latency: 0, lastCheck: timestamp };

		try {
			// 1. 测试cortex-mem-service基础可用性（API端点优先）
			const serviceStartTime = Date.now();
			const serviceResponse = await fetch('/api/memories?limit=1');
			const serviceLatency = Date.now() - serviceStartTime;

			if (serviceResponse.ok) {
				// API端点正常，说明服务可用
				mainService.status = 'connected';
				mainService.latency = serviceLatency;
			} else {
				// 如果API失败，再尝试健康检查端点，但健康检查失败不应该影响主要判断
				try {
					const healthStartTime = Date.now();
					const healthResponse = await fetch('/health');
					const healthLatency = Date.now() - healthStartTime;

					if (healthResponse.ok) {
						const healthData = await healthResponse.json();
						// 即使健康检查显示不健康，如果API可以访问，服务还是可用的
						mainService.status = 'connected';
						mainService.latency = Math.min(serviceLatency, healthLatency);
					}
				} catch (healthErr) {
					console.warn('健康检查失败，但API可能仍可用:', healthErr);
					// 健康检查失败不代表服务不可用，保持连接状态或设置connecting
					if (serviceLatency > 0) {
						mainService.status = 'connecting';
						mainService.latency = serviceLatency;
					}
				}
			}
		} catch (serviceErr) {
			console.warn('cortex-mem-service检测失败:', serviceErr);
			mainService.status = 'connecting';
		}

		try {
			// 2. 通过insights server API获取向量存储状态
			const vectorStoreStartTime = Date.now();
			const vectorStoreResponse = await fetch('/api/system/vector-store/status');
			const vectorStoreLatency = Date.now() - vectorStoreStartTime;

			if (vectorStoreResponse.ok) {
				const vectorStoreData = await vectorStoreResponse.json();
				if (vectorStoreData.success && vectorStoreData.data) {
					vectorStore.status = vectorStoreData.data.status;
					vectorStore.latency = vectorStoreLatency;
				} else {
					vectorStore.status = 'connecting';
				}
			} else {
				vectorStore.status = 'connecting';
			}
		} catch (vectorStoreErr) {
			console.warn('获取向量存储状态失败:', vectorStoreErr);
			vectorStore.status = 'connecting';
		}

		try {
			// 3. 通过insights server API获取LLM服务状态
			const llmStartTime = Date.now();
			const llmResponse = await fetch('/api/system/llm/status');
			const llmLatency = Date.now() - llmStartTime;

			if (llmResponse.ok) {
				const llmData = await llmResponse.json();
				if (llmData.success && llmData.data) {
					const { overall_status, completion_model, embedding_model } = llmData.data;

					// 更新LLM服务状态
					llmService.status = overall_status === 'healthy' ? 'connected' : 'connecting';
					llmService.latency = llmLatency;
					llmService.provider = completion_model.provider;
					llmService.model = `${completion_model.model_name} / ${embedding_model.model_name}`;
					llmService.lastCheck = new Date().toISOString();

					// 更新模型详细信息
					llmService.completionModel = {
						available: completion_model.available,
						latency: completion_model.latency_ms,
						error: completion_model.error_message
					};

					llmService.embeddingModel = {
						available: embedding_model.available,
						latency: embedding_model.latency_ms,
						error: embedding_model.error_message
					};
				} else {
					llmService.status = 'connecting';
				}
			} else {
				llmService.status = 'connecting';
			}
		} catch (llmErr) {
			console.warn('获取LLM服务状态失败:', llmErr);
			llmService.status = 'connecting';
		}

		return { mainService, vectorStore, llmService };
	}

	// 异步检测服务状态
	async function detectServicesAsync() {
		isDetectingServices = true;
		try {
			const timestamp = new Date().toLocaleTimeString('zh-CN', { hour12: false });
			
			// 初始化为检测中状态
			localSystemStatus = {
				cortexMemService: { status: 'detecting', latency: 0, lastCheck: timestamp },
				qdrant: { status: 'detecting', latency: 0, lastCheck: timestamp },
				llmService: { status: 'detecting', latency: 0, lastCheck: timestamp }
			};

			const serviceStatuses = await detectIndividualServices(timestamp);

			// 更新本地系统状态
			localSystemStatus = {
				cortexMemService: {
					status: serviceStatuses.mainService.status,
					latency: serviceStatuses.mainService.latency,
					version: '',
					lastCheck: serviceStatuses.mainService.lastCheck
				},
				qdrant: {
					status: serviceStatuses.vectorStore.status,
					latency: serviceStatuses.vectorStore.latency,
					version: '',
					collectionCount: 0,
					lastCheck: serviceStatuses.vectorStore.lastCheck
				},
				llmService: {
					status: serviceStatuses.llmService.status,
					latency: serviceStatuses.llmService.latency,
					provider: serviceStatuses.llmService.provider || '',
					model: serviceStatuses.llmService.model || '',
					lastCheck: serviceStatuses.llmService.lastCheck,
					completionModel: serviceStatuses.llmService.completionModel,
					embeddingModel: serviceStatuses.llmService.embeddingModel
				}
			};

			// 派发状态更新事件
			dispatch('statusUpdate', { systemStatus: localSystemStatus });
		} catch (err) {
			console.error('异步检测服务状态失败:', err);
		} finally {
			isDetectingServices = false;
		}
	}

	// 手动刷新
	async function handleRefresh() {
		isRefreshing = true;
		isDetectingServices = true;
		try {
			await detectServicesAsync();
		} finally {
			isDetectingServices = false;
			isRefreshing = false;
		}
	}

	// 状态显示函数
	function getStatusColor(status: string) {
		switch (status) {
			case 'connected':
				return 'text-green-500 dark:bg-green-900/20';
			case 'connecting':
				return 'text-yellow-500 dark:bg-yellow-900/20';
			case 'detecting':
				return 'text-blue-500 dark:bg-blue-900/20';
			case 'disconnected':
				return 'text-red-500 dark:bg-red-900/20';
			default:
				return 'text-gray-500 dark:bg-gray-800';
		}
	}

	function getStatusLightColor(status: string) {
		switch (status) {
			case 'connected':
				return 'bg-green-400 dark:bg-green-900/20';
			case 'connecting':
				return 'bg-yellow-500 dark:bg-yellow-900/20';
			case 'detecting':
				return 'bg-blue-400 dark:bg-blue-900/20 animate-pulse';
			case 'disconnected':
				return 'bg-red-500 dark:bg-red-900/20';
			default:
				return 'bg-gray-500 dark:bg-gray-800';
		}
	}

	function getStatusText(status: string) {
		switch (status) {
			case 'connected':
				return '已连接';
			case 'connecting':
				return '连接中';
			case 'detecting':
				return '检测中';
			case 'disconnected':
				return '已断开';
			default:
				return '未知';
		}
	}

	onMount(() => {
		if (autoDetect) {
			// 延迟一点执行，确保组件完全挂载
			setTimeout(() => {
				detectServicesAsync();
			}, 100);
		}
	});
</script>

<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
	<div class="flex items-center justify-between mb-6">
		<h2 class="text-lg font-semibold text-gray-900 dark:text-white">{title}</h2>
		{#if showRefreshButton}
			<button
				on:click={handleRefresh}
				disabled={isDetectingServices}
				class="px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors duration-200 flex items-center space-x-2"
			>
				{#if isDetectingServices}
					<svg class="animate-spin h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
					</svg>
					<span>检测中...</span>
				{:else}
					<span>重新检查所有服务</span>
				{/if}
			</button>
		{/if}
	</div>

	<div class="space-y-4">
		{#if localSystemStatus}
			{#each Object.entries(localSystemStatus) as [service, data]}
			{#if data && typeof data === 'object' && data.status}
				<div class="p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
					<div class="flex items-center justify-between mb-2">
						<div class="flex items-center space-x-2">
							<div class={`w-2 h-2 rounded-full ${getStatusLightColor(data.status)}`}></div>
							<span class="font-medium text-gray-900 dark:text-white">
								{service === 'cortexMemService'
									? 'Cortex Memory Service'
									: service === 'qdrant'
										? 'Qdrant 数据库'
										: 'LLM 服务'}
							</span>
						</div>
						<span class={`text-sm font-medium ${getStatusColor(data.status)}`}>
							{getStatusText(data.status)}
						</span>
					</div>

					<div class="grid grid-cols-2 gap-2 text-sm text-gray-600 dark:text-gray-400">
						<div>
							延迟: <span class="font-medium">
								{#if data.status === 'detecting'}
									<span class="animate-pulse">检测中...</span>
								{:else}
									{data.latency}ms
								{/if}
							</span>
						</div>
						{#if data.provider}
							<div>提供商: <span class="font-medium">{data.provider}</span></div>
						{/if}
						{#if data.model}
							<div>模型: <span class="font-medium text-xs">{data.model}</span></div>
						{/if}
					</div>

					{#if data.lastCheck}
						<div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
							最后检查: {data.lastCheck}
						</div>
					{/if}
				</div>
			{/if}
			{/each}
		{/if}
	</div>
</div>

<style>
	@keyframes pulse {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}
	.animate-pulse {
		animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
	}
</style>