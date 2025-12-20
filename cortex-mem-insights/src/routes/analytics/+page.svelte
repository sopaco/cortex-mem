<script lang="ts">
	import { onMount } from 'svelte';
	import { Line } from 'svelte-chartjs';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		Title,
		Tooltip,
		Legend,
		Filler
	} from 'chart.js';
	import api from '$lib/api/client';

	// 注册Chart.js组件
	ChartJS.register(
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		Title,
		Tooltip,
		Legend,
		Filler
	);

	let isLoading = true;
	let error: string | null = null;

	// 真实数据
	let typeDistribution: Array<{ type: string; count: number; percentage: number }> = [];
	let qualityDistribution: Array<{ range: string; count: number; color: string }> = [];
	let timeTrends: Array<{ date: string; count: number }> = [];
	let userStats: Array<{ userId: string; memoryCount: number; avgImportance: number }> = [];
	let summaryStats = {
		totalMemories: 0,
		averageQuality: 0,
		activeUsers: 0,
		optimizationCount: 0
	};

	// 图表配置
	let chartData: any = null;
	let chartOptions = {
		responsive: true,
		maintainAspectRatio: false,
		interaction: {
			intersect: false,
			mode: 'index' as const
		},
		plugins: {
			legend: {
				display: false
			},
			tooltip: {
				backgroundColor: 'rgba(0, 0, 0, 0.8)',
				titleColor: 'white',
				bodyColor: 'white',
				borderColor: 'rgba(59, 130, 246, 0.5)',
				borderWidth: 1,
				cornerRadius: 8,
				displayColors: false,
				callbacks: {
					title: function(context: any) {
						return `${context[0].label}`;
					},
					label: function(context: any) {
						return `新增记忆: ${context.parsed.y} 条`;
					}
				}
			}
		},
		scales: {
			x: {
				grid: {
					display: false
				},
				ticks: {
					color: 'rgb(107, 114, 128)',
					font: {
						size: 12
					}
				}
			},
			y: {
				beginAtZero: true,
				grid: {
					color: 'rgba(107, 114, 128, 0.1)',
					borderDash: [2, 2]
				},
				ticks: {
					color: 'rgb(107, 114, 128)',
					font: {
						size: 12
					},
					callback: function(value: any) {
						return value + ' 条';
					}
				}
			}
		},
		elements: {
			point: {
				radius: 6,
				hoverRadius: 8,
				backgroundColor: 'rgb(59, 130, 246)',
				borderColor: 'white',
				borderWidth: 2,
				hoverBackgroundColor: 'rgb(37, 99, 235)',
				hoverBorderColor: 'white',
				hoverBorderWidth: 3
			},
			line: {
				borderWidth: 3,
				tension: 0.4,
				fill: true,
				backgroundColor: function(context: any) {
					const chart = context.chart;
					const { ctx, chartArea } = chart;

					if (!chartArea) {
						return null; // 防止服务器端渲染错误
					}

					const gradient = ctx.createLinearGradient(0, chartArea.top, 0, chartArea.bottom);
					gradient.addColorStop(0, 'rgba(59, 130, 246, 0.3)');
					gradient.addColorStop(1, 'rgba(59, 130, 246, 0.0)');
					return gradient;
				}
			}
		}
	};

	onMount(async () => {
		try {
			await loadAnalyticsData();
		} catch (err) {
			console.error('加载统计数据失败:', err);
			error = err instanceof Error ? err.message : '加载数据失败';
			loadDefaultData();
		} finally {
			isLoading = false;
		}
	});

	async function loadAnalyticsData() {
		try {
			// 获取所有记忆数据用于分析
			const memoriesResponse = await api.memory.list({ limit: 1000 });
			const memories = memoriesResponse.memories;

			if (memories.length === 0) {
				loadDefaultData();
				return;
			}

			// 计算统计数据
			summaryStats = {
				totalMemories: memories.length,
				averageQuality: calculateAverageQuality(memories),
				activeUsers: calculateActiveUsers(memories),
				optimizationCount: 0 // TODO: 从优化API获取
			};

			// 计算类型分布
			typeDistribution = calculateTypeDistribution(memories);

			// 计算质量分布
			qualityDistribution = calculateQualityDistribution(memories);

			// 计算时间趋势
			timeTrends = calculateTimeTrends(memories);

			// 计算用户统计
			userStats = calculateUserStats(memories);

			// 生成图表数据
			generateChartData();
		} catch (err) {
			console.error('分析数据错误:', err);
			throw err;
		}
	}

	function loadDefaultData() {
		summaryStats = {
			totalMemories: 0,
			averageQuality: 0,
			activeUsers: 0,
			optimizationCount: 0
		};
		typeDistribution = [];
		qualityDistribution = [
			{ range: '90-100%', count: 0, color: 'bg-green-500' },
			{ range: '70-89%', count: 0, color: 'bg-blue-500' },
			{ range: '50-69%', count: 0, color: 'bg-yellow-500' },
			{ range: '0-49%', count: 0, color: 'bg-red-500' }
		];
		timeTrends = [];
		userStats = [];
		generateChartData();
	}

	function generateChartData() {
		const labels = timeTrends.map(trend => trend.date);
		const data = timeTrends.map(trend => trend.count);

		chartData = {
			labels,
			datasets: [
				{
					label: '新增记忆',
					data,
					borderColor: 'rgb(59, 130, 246)',
					backgroundColor: 'rgba(59, 130, 246, 0.1)',
					borderWidth: 3,
					tension: 0.4,
					fill: true,
					pointRadius: 6,
					pointHoverRadius: 8,
					pointBackgroundColor: 'rgb(59, 130, 246)',
					pointBorderColor: 'white',
					pointBorderWidth: 2,
					pointHoverBackgroundColor: 'rgb(37, 99, 235)',
					pointHoverBorderColor: 'white',
					pointHoverBorderWidth: 3
				}
			]
		};
	}

	function calculateAverageQuality(memories: any[]): number {
		if (memories.length === 0) return 0;

		const totalScore = memories.reduce((sum, memory) => {
			return sum + (memory.metadata.importance_score || 0.5);
		}, 0);

		return totalScore / memories.length;
	}

	// 重要性评分已经在cortex-mem-core中计算好了，直接使用memory.metadata.importance_score字段

	function calculateActiveUsers(memories: any[]): number {
		const users = new Set();
		memories.forEach((memory) => {
			if (memory.metadata?.user_id) {
				users.add(memory.metadata.user_id);
			}
		});
		return users.size;
	}

	function calculateTypeDistribution(
		memories: any[]
	): Array<{ type: string; count: number; percentage: number }> {
		const typeCounts: Record<string, number> = {};

		memories.forEach((memory) => {
			const type = memory.metadata?.memory_type || 'Unknown';
			typeCounts[type] = (typeCounts[type] || 0) + 1;
		});

		const total = memories.length;
		return Object.entries(typeCounts)
			.map(([type, count]) => ({
				type,
				count,
				percentage: Math.round((count / total) * 100)
			}))
			.sort((a, b) => b.count - a.count);
	}

	function calculateQualityDistribution(
		memories: any[]
	): Array<{ range: string; count: number; color: string }> {
		let high = 0; // 90-100%
		let good = 0; // 70-89%
		let medium = 0; // 50-69%
		let low = 0; // 0-49%

		memories.forEach((memory) => {
			const score = memory.metadata.importance_score || 0.5;
			if (score >= 0.9) {
				high++;
			} else if (score >= 0.7) {
				good++;
			} else if (score >= 0.5) {
				medium++;
			} else {
				low++;
			}
		});

		const total = memories.length;
		return [
			{ range: '0.90-1.00', count: high, color: 'bg-green-500' },
			{ range: '0.70-0.89', count: good, color: 'bg-blue-500' },
			{ range: '0.50-0.69', count: medium, color: 'bg-yellow-500' },
			{ range: '0.00-0.49', count: low, color: 'bg-red-500' }
		];
	}

	function calculateTimeTrends(memories: any[]): Array<{ date: string; count: number }> {
		const dateCounts: Record<string, number> = {};

		// 获取最近7天
		const today = new Date();
		for (let i = 6; i >= 0; i--) {
			const date = new Date(today);
			date.setDate(date.getDate() - i);
			const dateStr = date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' });
			dateCounts[dateStr] = 0;
		}

		memories.forEach((memory) => {
			const date = new Date(memory.created_at);
			const dateStr = date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' });
			if (dateCounts.hasOwnProperty(dateStr)) {
				dateCounts[dateStr]++;
			}
		});

		return Object.entries(dateCounts).map(([date, count]) => ({ date, count }));
	}

	function calculateUserStats(
		memories: any[]
	): Array<{ userId: string; memoryCount: number; avgImportance: number }> {
		const userData: Record<string, { count: number; totalScore: number }> = {};

		memories.forEach((memory) => {
			const userId = memory.metadata?.user_id || 'unknown';
			if (!userData[userId]) {
				userData[userId] = { count: 0, totalScore: 0 };
			}
			userData[userId].count++;
			userData[userId].totalScore += memory.metadata.importance_score || 0.5;
		});

		return Object.entries(userData)
			.map(([userId, data]) => ({
				userId,
				memoryCount: data.count,
				avgImportance: data.totalScore / data.count
			}))
			.sort((a, b) => b.memoryCount - a.memoryCount)
			.slice(0, 5); // 只显示前5个用户
	}

	function getPercentageColor(percentage: number) {
		if (percentage >= 30) return 'text-blue-600 dark:text-blue-400';
		if (percentage >= 20) return 'text-green-600 dark:text-green-400';
		if (percentage >= 10) return 'text-yellow-600 dark:text-yellow-400';
		return 'text-gray-600 dark:text-gray-400';
	}
</script>

<div class="space-y-8">
	<!-- 页面标题 -->
	<div>
		<h1 class="text-3xl font-bold text-gray-900 dark:text-white">统计分析</h1>
		<p class="mt-2 text-gray-600 dark:text-gray-400">深入分析记忆数据的分布、质量和趋势</p>
	</div>

	{#if isLoading}
		<!-- 加载状态 -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
			{#each Array(4) as _, i}
				<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse">
					<div class="h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-6"></div>
					<div class="h-48 bg-gray-200 dark:bg-gray-700 rounded"></div>
				</div>
			{/each}
		</div>
	{:else if error}
		<!-- 错误状态 -->
		<div
			class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl p-6"
		>
			<div class="flex items-center">
				<div
					class="w-8 h-8 bg-red-100 dark:bg-red-900/30 rounded-lg flex items-center justify-center mr-3"
				>
					<span class="text-red-600 dark:text-red-400">⚠️</span>
				</div>
				<div>
					<h3 class="text-lg font-medium text-red-800 dark:text-red-200">加载失败</h3>
					<p class="text-red-600 dark:text-red-400">{error}</p>
				</div>
			</div>
		</div>
	{:else}
		<!-- 统计概览 -->
		<div class="grid grid-cols-1 md:grid-cols-4 gap-6">
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<p class="text-sm font-medium text-gray-600 dark:text-gray-400">总记忆数</p>
				<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
					{summaryStats.totalMemories.toLocaleString()}
				</p>
				<p class="mt-2 text-sm text-green-600 dark:text-green-400">当前总数</p>
			</div>

			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<p class="text-sm font-medium text-gray-600 dark:text-gray-400">平均质量</p>
				<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
					{summaryStats.averageQuality.toFixed(2)}
				</p>
				<p class="mt-2 text-sm text-blue-600 dark:text-blue-400">基于重要性评分</p>
			</div>

			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<p class="text-sm font-medium text-gray-600 dark:text-gray-400">活跃用户</p>
				<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
					{summaryStats.activeUsers}
				</p>
				<p class="mt-2 text-sm text-purple-600 dark:text-purple-400">有记忆的用户</p>
			</div>

			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<p class="text-sm font-medium text-gray-600 dark:text-gray-400">优化次数</p>
				<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">
					{summaryStats.optimizationCount}
				</p>
				<p class="mt-2 text-sm text-yellow-600 dark:text-yellow-400">历史优化记录</p>
			</div>
		</div>

		<!-- 图表区域 -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
			<!-- 类型分布 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">记忆类型分布</h2>

				<div class="space-y-4">
					{#each typeDistribution as item}
						<div>
							<div class="flex justify-between mb-1">
								<span class="text-sm font-medium text-gray-700 dark:text-gray-300">
									{item.type}
								</span>
								<span class={`text-sm font-bold ${getPercentageColor(item.percentage)}`}>
									{item.percentage}%
								</span>
							</div>
							<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
								<div
									class="h-2 rounded-full bg-blue-500"
									style={`width: ${item.percentage}%`}
								></div>
							</div>
							<div class="flex justify-between mt-1">
								<span class="text-xs text-gray-500 dark:text-gray-400">
									{item.count} 条记录
								</span>
								<span class="text-xs text-gray-500 dark:text-gray-400">
									占比 {item.percentage}%
								</span>
							</div>
						</div>
					{/each}
				</div>

				<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
					<div class="flex items-center justify-between text-sm text-gray-500 dark:text-gray-400">
						<span>总计: {summaryStats.totalMemories} 条记忆</span>
					</div>
				</div>
			</div>

			<!-- 质量分布 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
				<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">质量评分分布</h2>

				<div class="space-y-4">
					{#each qualityDistribution as item}
						<div>
							<div class="flex items-center justify-between mb-1">
								<div class="flex items-center space-x-2">
									<div class={`w-3 h-3 rounded-full ${item.color}`}></div>
									<span class="text-sm font-medium text-gray-700 dark:text-gray-300">
										{item.range}
									</span>
								</div>
								<span class="text-sm font-bold text-gray-900 dark:text-white">
									{item.count} 条
								</span>
							</div>
							<div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
								<div
									class={`h-3 rounded-full ${item.color}`}
									style={`width: ${summaryStats.totalMemories > 0 ? (item.count / summaryStats.totalMemories * 100).toFixed(1) : 0}%`}
								></div>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<!-- 时间趋势 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 lg:col-span-2">
				<div class="flex items-center justify-between mb-6">
					<h2 class="text-lg font-semibold text-gray-900 dark:text-white">新增记忆趋势</h2>
					<div class="flex space-x-2">
						<button
							class="px-3 py-1 text-sm bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 rounded"
						>
							最近7天
						</button>
						<button
							class="px-3 py-1 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
						>
							最近30天
						</button>
					</div>
				</div>

				{#if chartData && timeTrends.length > 0}
					<div class="h-64">
						<Line data={chartData} options={chartOptions} />
					</div>
				{:else if timeTrends.length > 0}
					<!-- 备用显示：当图表库无法加载时 -->
					<div class="h-64 flex items-end space-x-2">
						{#each timeTrends as trend, i}
							<div class="flex-1 flex flex-col items-center">
								<div
									class="w-full bg-blue-500 rounded-t-lg transition-all duration-300 hover:bg-blue-600"
									style={`height: ${(trend.count / 80) * 100}%`}
									title={`${trend.date}: ${trend.count} 条`}
								></div>
								<div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
									{trend.date}
								</div>
								<div class="text-sm font-medium text-gray-900 dark:text-white">
									{trend.count}
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="h-64 flex items-center justify-center text-gray-500 dark:text-gray-400">
						<div class="text-center">
							<div class="text-4xl mb-2">📊</div>
							<p>暂无数据</p>
							<p class="text-sm">等待记忆数据加载...</p>
						</div>
					</div>
				{/if}

				<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
					<div class="flex items-center justify-between text-sm text-gray-500 dark:text-gray-400">
						<span>日均新增: 54.8 条</span>
						<span>峰值: 68 条 (12月13日)</span>
					</div>
				</div>
			</div>

			<!-- 用户统计 -->
			<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 lg:col-span-2">
				<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">用户维度统计</h2>

				<div class="overflow-x-auto">
					<table class="w-full">
						<thead class="bg-gray-50 dark:bg-gray-900/50">
							<tr>
								<th
									class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase"
								>
									用户ID
								</th>
								<th
									class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase"
								>
									记忆数量
								</th>
								<th
									class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase"
								>
									平均质量
								</th>
								<th
									class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase"
								>
									占比
								</th>
								<th
									class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase"
								>
									趋势
								</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
							{#each userStats as user}
								<tr class="hover:bg-gray-50 dark:hover:bg-gray-900/30">
									<td class="px-4 py-3">
										<div class="font-medium text-gray-900 dark:text-white">
											{user.userId}
										</div>
									</td>
									<td class="px-4 py-3">
										<div class="flex items-center">
											<div class="w-24 bg-gray-200 dark:bg-gray-700 rounded-full h-2 mr-2">
												<div
													class="h-2 rounded-full bg-blue-500"
													style={`width: ${summaryStats.totalMemories > 0 ? (user.memoryCount / summaryStats.totalMemories) * 100 : 0}%`}
												></div>
											</div>
											<span class="text-sm font-medium">
												{user.memoryCount}
											</span>
										</div>
									</td>
									<td class="px-4 py-3">
										<span
											class={`px-2 py-1 rounded text-xs font-medium ${
												user.avgImportance >= 0.8
													? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300'
													: user.avgImportance >= 0.7
														? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300'
														: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300'
											}`}
										>
											{user.avgImportance.toFixed(2)}
										</span>
									</td>
									<td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
										{summaryStats.totalMemories > 0
											? ((user.memoryCount / summaryStats.totalMemories) * 100).toFixed(1)
											: '0.0'}%
									</td>
									<td class="px-4 py-3">
										<span class="text-gray-600 dark:text-gray-400 text-sm font-medium">
											数据不足
										</span>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
					<div class="flex items-center justify-between">
						<span class="text-sm text-gray-500 dark:text-gray-400">
							前{userStats.length}用户占总记忆的 {summaryStats.totalMemories > 0
								? (
										(userStats.reduce((sum, user) => sum + user.memoryCount, 0) /
											summaryStats.totalMemories) *
										100
									).toFixed(1)
								: '0.0'}%
						</span>
					</div>
				</div>
			</div>
		</div>

		<!-- 分析工具 -->
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">分析工具</h2>

			<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-700 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all duration-200"
					on:click={() => console.log('生成质量报告')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center"
						>
							<span class="text-xl">📈</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">质量分析报告</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">生成详细的质量分析</p>
						</div>
					</div>
				</button>

				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-green-300 dark:hover:border-green-700 hover:bg-green-50 dark:hover:bg-green-900/20 transition-all duration-200"
					on:click={() => console.log('趋势预测')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center"
						>
							<span class="text-xl">🔮</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">趋势预测</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">预测未来增长趋势</p>
						</div>
					</div>
				</button>

				<button
					class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-purple-300 dark:hover:border-purple-700 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-all duration-200"
					on:click={() => console.log('对比分析')}
				>
					<div class="flex items-center space-x-3">
						<div
							class="w-10 h-10 bg-purple-100 dark:bg-purple-900/30 rounded-lg flex items-center justify-center"
						>
							<span class="text-xl">⚖️</span>
						</div>
						<div class="text-left">
							<p class="font-medium text-gray-900 dark:text-white">对比分析</p>
							<p class="text-sm text-gray-500 dark:text-gray-400">不同时间段对比</p>
						</div>
					</div>
				</button>
			</div>
		</div>
	{/if}
</div>
