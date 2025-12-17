<script lang="ts">
	import { onMount } from 'svelte';
	import api from '$lib/api/client';

	// 真实数据
	let stats = {
		totalMemories: 0,
		optimizationCount: 0,
		averageQuality: 0,
		qualityDistribution: { high: 0, medium: 0, low: 0 }
	};

	// 使用与监控页面相同的数据结构
	let systemStatus = {
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

	let recentMemories: Array<{
		id: string;
		content: string;
		type: string;
		importance: number;
		createdAt: string;
	}> = [];

	let isLoading = true;
	let isDetectingServices = false;
	let error: string | null = null;

	onMount(async () => {
		try {
			// 先加载基本数据，不等待服务检测
			await loadBasicData();
			// 异步检测服务状态，不阻塞页面
			detectServicesAsync();
		} catch (err) {
			console.error('加载仪表板数据失败:', err);
			error = err instanceof Error ? err.message : '加载数据失败';
			fallbackToMockData();
		} finally {
			isLoading = false;
		}
	});

	// 加载基本数据，不等待服务检测
	async function loadBasicData() {
		try {
			let memories: any[] = [];

			// 获取记忆统计（这也可以验证服务的实际可用性）
			try {
				const memoriesResponse = await api.memory.list({ limit: 1000 });
				memories = memoriesResponse.memories || [];
				console.log(`获取到 ${memories.length} 条记忆记录`);
			} catch (memoryErr) {
				console.warn('获取记忆列表失败:', memoryErr);
				memories = [];
			}

			// 计算统计数据
			const totalCount = memories.length;

			// 计算质量分布（基于记忆类型和元数据）
			const qualityStats = calculateQualityDistribution(memories);

			// 获取最近记忆
			recentMemories = memories
				.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
				.slice(0, 5)
				.map((memory) => ({
					id: memory.id,
					content: memory.content,
					type: memory.metadata.memory_type || 'Unknown',
					importance: calculateImportanceScore(memory),
					createdAt: formatDate(memory.created_at)
				}));

			stats = {
				totalMemories: totalCount,
				optimizationCount: 0, // TODO: 从优化API获取实际计数
				averageQuality: qualityStats.average,
				qualityDistribution: qualityStats.distribution
			};

			// 初始化系统状态为检测中
			const timestamp = new Date().toLocaleTimeString('zh-CN', { hour12: false });
			systemStatus = {
				cortexMemService: {
					status: 'detecting',
					latency: 0,
					version: '',
					lastCheck: timestamp
				},
				qdrant: {
					status: 'detecting',
					latency: 0,
					version: '',
					collectionCount: 0,
					lastCheck: timestamp
				},
				llmService: {
					status: 'detecting',
					latency: 0,
					provider: '',
					model: '',
					lastCheck: timestamp
				}
			};
		} catch (err) {
			console.error('加载基本数据错误:', err);
			throw err;
		}
	}

	// 异步检测服务状态
	async function detectServicesAsync() {
		isDetectingServices = true;
		try {
			const timestamp = new Date().toLocaleTimeString('zh-CN', { hour12: false });
			const serviceStatuses = await detectIndividualServices(timestamp);

			// 更新系统状态
			systemStatus = {
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
					provider: '',
					model: '',
					lastCheck: serviceStatuses.llmService.lastCheck
				}
			};
		} catch (err) {
			console.error('异步检测服务状态失败:', err);
		} finally {
			isDetectingServices = false;
		}
	}

	// 独立检测各个服务状态（与监控页面相同的逻辑）
	async function detectIndividualServices(timestamp: string) {
		const mainService = { status: 'detecting', latency: 0, lastCheck: timestamp };
		const vectorStore = { status: 'detecting', latency: 0, lastCheck: timestamp };
		const llmService = { status: 'detecting', latency: 0, lastCheck: timestamp };

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
			mainService.status = 'detecting';
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
					vectorStore.status = 'error';
				}
			} else {
				vectorStore.status = 'detecting';
			}
		} catch (vectorStoreErr) {
			console.warn('获取向量存储状态失败:', vectorStoreErr);
			vectorStore.status = 'detecting';
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
					llmService.status = overall_status === 'healthy' ? 'connected' : 'error';
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
					llmService.status = 'detecting';
				}
			} else {
				llmService.status = 'detecting';
			}
		} catch (llmErr) {
			console.warn('获取LLM服务状态失败:', llmErr);
			llmService.status = 'detecting';
		}

		return { mainService, vectorStore, llmService };
	}

	// 获取Qdrant集合数量 - 已移除API调用

	// 计算质量分布
	function calculateQualityDistribution(memories: any[]) {
		if (memories.length === 0) {
			return { average: 0, distribution: { high: 0, medium: 0, low: 0 } };
		}

		let high = 0;
		let medium = 0;
		let low = 0;
		let totalScore = 0;

		memories.forEach((memory) => {
			const score = calculateImportanceScore(memory);
			totalScore += score;

			if (score >= 0.8) {
				high++;
			} else if (score >= 0.6) {
				medium++;
			} else {
				low++;
			}
		});

		const average = totalScore / memories.length;

		return {
			average,
			distribution: { high, medium, low }
		};
	}

	// 计算重要性评分
	function calculateImportanceScore(memory: any) {
		// 基于记忆类型、角色和自定义字段计算重要性
		let score = 0.5; // 基础分数

		const memoryType = memory.metadata?.memory_type?.toLowerCase() || '';
		const role = memory.metadata?.role?.toLowerCase() || '';

		// 根据记忆类型调整分数
		if (memoryType.includes('procedural') || memoryType.includes('workflow')) {
			score += 0.3;
		} else if (memoryType.includes('personal')) {
			score += 0.2;
		} else if (memoryType.includes('conversational')) {
			score += 0.1;
		}

		// 根据角色调整分数
		if (role.includes('admin') || role.includes('system')) {
			score += 0.2;
		} else if (role.includes('user')) {
			score += 0.1;
		}

		// 检查自定义字段中的重要性标识
		if (memory.metadata?.custom?.importance) {
			score += memory.metadata.custom.importance * 0.3;
		}

		return Math.min(1.0, Math.max(0.0, score));
	}

	function fallbackToMockData() {
		console.log('回退到默认数据');
		const timestamp = new Date().toLocaleTimeString('zh-CN', { hour12: false });

		stats = {
			totalMemories: 0,
			optimizationCount: 0,
			averageQuality: 0.5,
			qualityDistribution: { high: 0, medium: 0, low: 0 }
		};

		systemStatus = {
			cortexMemService: {
				status: 'detecting',
				latency: 0,
				version: '1.0.0',
				lastCheck: timestamp
			},
			qdrant: {
				status: 'detecting',
				latency: 0,
				version: '1.7.0',
				collectionCount: 0,
				lastCheck: timestamp
			},
			llmService: {
				status: 'detecting',
				latency: 0,
				provider: 'Unknown',
				model: 'Unknown',
				lastCheck: timestamp
			}
		};

		recentMemories = [];

		isLoading = false;
	}

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
		<p class="mt-2 text-gray-600 dark:text-gray-400">监控和分析 Cortex Memory 系统的运行状态</p>
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
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
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
					高质量记忆: <span class="font-medium text-green-600 dark:text-green-400"
						>{stats.qualityDistribution.high}</span
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

			<!-- 质量分布 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 border-l-4 border-green-500">
				<div class="flex items-center justify-between">
					<div>
						<p class="text-sm font-medium text-gray-600 dark:text-gray-400">质量分布</p>
						<p class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">
							{stats.qualityDistribution.high}/{stats.qualityDistribution.medium}/{stats
								.qualityDistribution.low}
						</p>
					</div>
					<div
						class="w-12 h-12 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center"
					>
						<span class="text-2xl">📊</span>
					</div>
				</div>
				<p class="mt-2 text-sm text-gray-500 dark:text-gray-400">高/中/低质量记忆数量</p>
				<div class="mt-2 flex space-x-1">
					<div class="flex-1 bg-green-200 dark:bg-green-800 rounded h-1"></div>
					<div class="flex-1 bg-yellow-200 dark:bg-yellow-800 rounded h-1"></div>
					<div class="flex-1 bg-red-200 dark:bg-red-800 rounded h-1"></div>
				</div>
			</div>
		</div>

		<!-- 系统状态和最近记忆 -->
		<div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
			<!-- 系统状态 -->
			<div class="lg:col-span-1">
				<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
					<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">服务状态</h2>

					<div class="space-y-4">
						{#each Object.entries(systemStatus) as [service, data]}
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
					</div>

					<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
						<button
							on:click={() => detectServicesAsync()}
							disabled={isDetectingServices}
							class="w-full px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors duration-200"
						>
							{isDetectingServices ? '检测中...' : '重新检查所有服务'}
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
