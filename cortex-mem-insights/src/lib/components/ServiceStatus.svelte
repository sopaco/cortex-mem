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

		// 首先通过/api/system/status检测cortex-mem-service的可用性
		let cortexMemServiceAvailable = false;

		try {
			// 1. 通过insights server的/api/system/status检测cortex-mem-service
			const serviceStartTime = Date.now();

			// 创建带超时的请求
			const controller = new AbortController();
			const timeoutId = setTimeout(() => controller.abort(), 5000); // 5秒超时

			try {
				const serviceResponse = await fetch('/api/system/status', {
					signal: controller.signal
				});
				clearTimeout(timeoutId);
				const serviceLatency = Date.now() - serviceStartTime;

				if (serviceResponse.ok) {
					const responseData = await serviceResponse.json();
					// 检查cortex_mem_service字段
					if (responseData.success && responseData.data) {
						const cortexMemStatus = responseData.data.cortex_mem_service;
						if (cortexMemStatus === true) {
							mainService.status = 'connected';
							mainService.latency = serviceLatency;
							cortexMemServiceAvailable = true;
						} else {
							mainService.status = 'disconnected';
							mainService.latency = serviceLatency;
						}
					} else {
						mainService.status = 'disconnected';
						mainService.latency = serviceLatency;
					}
				} else {
					// HTTP错误状态码
					console.warn(`cortex-mem-service状态检查失败: HTTP ${serviceResponse.status}`);
					mainService.status = 'disconnected';
					mainService.latency = serviceLatency;
				}
			} catch (fetchErr) {
				clearTimeout(timeoutId);
				if (fetchErr.name === 'AbortError') {
					console.warn('cortex-mem-service状态检查超时');
					mainService.status = 'disconnected';
				} else {
					console.warn('cortex-mem-service检测失败:', fetchErr);
					mainService.status = 'disconnected';
				}
				mainService.latency = Date.now() - serviceStartTime;
			}
		} catch (serviceErr) {
			console.warn('cortex-mem-service检测异常:', serviceErr);
			mainService.status = 'disconnected';
		}

		// 如果cortex-mem-service不可用，则Qdrant和LLM服务也标记为不可用
		if (!cortexMemServiceAvailable) {
			console.warn('cortex-mem-service不可用，Qdrant和LLM服务也将标记为不可用');
			vectorStore.status = 'disconnected';
			vectorStore.latency = 0;
			llmService.status = 'disconnected';
			llmService.latency = 0;
			llmService.provider = 'unknown';
			llmService.model = 'unknown / unknown';
			return { mainService, vectorStore, llmService };
		}

		// 并发检测向量存储和LLM服务状态
		const [vectorStoreResult, llmResult] = await Promise.allSettled([
			// 检测向量存储状态
			(async () => {
				const vectorStoreStartTime = Date.now();
				const vectorStoreController = new AbortController();
				const vectorStoreTimeoutId = setTimeout(() => vectorStoreController.abort(), 3000);

				try {
					const vectorStoreResponse = await fetch('/api/system/vector-store/status', {
						signal: vectorStoreController.signal
					});
					clearTimeout(vectorStoreTimeoutId);
					const vectorStoreLatency = Date.now() - vectorStoreStartTime;

					if (vectorStoreResponse.ok) {
						const vectorStoreData = await vectorStoreResponse.json();
						if (vectorStoreData.success && vectorStoreData.data) {
							vectorStore.status = vectorStoreData.data.status;
							vectorStore.latency = vectorStoreLatency;
						} else {
							console.warn('向量存储API返回失败:', vectorStoreData);
							vectorStore.status = 'disconnected';
						}
					} else {
						console.warn(`向量存储API请求失败: HTTP ${vectorStoreResponse.status}`);
						vectorStore.status = 'disconnected';
					}
				} catch (vectorStoreFetchErr) {
					clearTimeout(vectorStoreTimeoutId);
					if (vectorStoreFetchErr.name === 'AbortError') {
						console.warn('向量存储API请求超时');
					} else {
						console.warn('获取向量存储状态失败:', vectorStoreFetchErr);
					}
					vectorStore.status = 'disconnected';
					vectorStore.latency = Date.now() - vectorStoreStartTime;
				}
			})(),

			// 检测LLM服务状态
			(async () => {
				const llmStartTime = Date.now();
				const llmController = new AbortController();
				const llmTimeoutId = setTimeout(() => llmController.abort(), 3000);

				try {
					const llmResponse = await fetch('/api/system/llm/status', {
						signal: llmController.signal
					});
					clearTimeout(llmTimeoutId);
					const llmLatency = Date.now() - llmStartTime;

					if (llmResponse.ok) {
						const llmData = await llmResponse.json();
						if (llmData.success && llmData.data) {
							const { overall_status, completion_model, embedding_model } = llmData.data;

							llmService.status = overall_status === 'healthy' ? 'connected' : 'disconnected';
							llmService.latency = llmLatency;
							llmService.provider = completion_model.provider;
							llmService.model = `${completion_model.model_name} / ${embedding_model.model_name}`;
							llmService.lastCheck = new Date().toISOString();

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
							console.warn('LLM API返回失败:', llmData);
							llmService.status = 'disconnected';
						}
					} else {
						console.warn(`LLM API请求失败: HTTP ${llmResponse.status}`);
						llmService.status = 'disconnected';
					}
				} catch (llmFetchErr) {
					clearTimeout(llmTimeoutId);
					if (llmFetchErr.name === 'AbortError') {
						console.warn('LLM API请求超时');
					} else {
						console.warn('获取LLM服务状态失败:', llmFetchErr);
					}
					llmService.status = 'disconnected';
					llmService.latency = Date.now() - llmStartTime;
				}
			})()
		]);

		// 处理并发请求的结果
		if (vectorStoreResult.status === 'rejected') {
			console.warn('向量存储检测异常:', vectorStoreResult.reason);
			vectorStore.status = 'disconnected';
		}

		if (llmResult.status === 'rejected') {
			console.warn('LLM服务检测异常:', llmResult.reason);
			llmService.status = 'disconnected';
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
					<svg
						class="animate-spin h-4 w-4 text-white"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
					>
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
						></circle>
						<path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						></path>
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
		0%,
		100% {
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
