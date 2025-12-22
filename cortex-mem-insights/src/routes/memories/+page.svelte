<script lang="ts">
	import { onMount } from 'svelte';
	import api from '$lib/api/client';
	import { t } from '$lib/i18n';

	interface Memory {
		id: string;
		content: string;
		type: string;
		importance: number;
		userId?: string;
		agentId?: string;
		createdAt: string;
		updatedAt: string;
	}

	let memories: Memory[] = [];
	let isLoading = true;
	let searchQuery = '';
	let selectedType = 'all';
	let sortBy = 'createdAt';
	let sortOrder: 'asc' | 'desc' = 'desc';
	let error: string | null = null;
	let filteredMemories: Memory[] = [];
	let selectedMemories: Set<string> = new Set();
	let selectedMemoryIds: Set<string> = new Set();
	let showBatchOperations = false;

	// 分页相关状态
	let currentPage = 1;
	let pageSize = 20;
	let paginatedMemories: Memory[] = [];
	let totalPages = 1;

	// 弹窗相关状态
	let showContentModal = false;
	let selectedContent = '';
	let selectedMemoryId = '';

	// 计算全选状态
	$: isAllSelected =
		paginatedMemories.length > 0 &&
		paginatedMemories.every((memory) => selectedMemories.has(memory.id));
	$: isPartialSelected =
		paginatedMemories.some((memory) => selectedMemories.has(memory.id)) && !isAllSelected;

	// 排序状态响应式计算
	$: console.log('排序状态变化:', { sortBy, sortOrder });

	// 为每个列计算排序图标
	$: createdAtSortIcon = sortBy === 'createdAt' ? (sortOrder === 'asc' ? '↑' : '↓') : '↓';
	$: importanceSortIcon = sortBy === 'importance' ? (sortOrder === 'asc' ? '↑' : '↓') : '↓';

	const memoryTypes = [
		{ value: 'all', label: $t('memories.allTypes') },
		{ value: 'conversational', label: $t('memories.conversational') },
		{ value: 'factual', label: $t('memories.factual') },
		{ value: 'personal', label: $t('memories.personal') },
		{ value: 'procedural', label: $t('memories.procedural') }
	];

	onMount(async () => {
		await loadMemories();
	});

	async function loadMemories() {
		try {
			isLoading = true;
			error = null;

			// 调用API获取记忆列表
			const response = await api.memory.list();

			// 转换API响应到前端数据结构
			memories = response.memories.map((memory: any) => {
				// 处理编码问题：尝试修复乱码
				let content = memory.content;
				try {
					// 如果内容看起来像乱码，尝试UTF-8解码
					if (content.includes('ç') || content.includes('æ') || content.includes('å')) {
						// 创建TextDecoder进行UTF-8解码
						const decoder = new TextDecoder('utf-8');
						// 将字符串转换为Uint8Array
						const encoder = new TextEncoder();
						const bytes = encoder.encode(content);
						// 尝试解码
						content = decoder.decode(bytes);
					}
				} catch (decodeError) {
					console.warn('解码内容失败，使用原始内容:', decodeError);
				}

				// 使用cortex-mem-core计算好的importance_score字段
				// 如果没有该字段，则使用默认值0.5（中性重要性）
				let importance = memory.metadata.importance_score || 0.5;
				
				// 确保重要性在0-1范围内
				importance = Math.max(0, Math.min(1, importance));

				return {
					id: memory.id,
					content: content,
					type: memory.metadata.memory_type.toLowerCase(),
					importance: importance,
					userId: memory.metadata.user_id,
					agentId: memory.metadata.agent_id,
					createdAt: memory.created_at,
					updatedAt: memory.updated_at
				};
			});

			// 重置到第一页
			currentPage = 1;
		} catch (err) {
			console.error('加载记忆失败:', err);
			error = err instanceof Error ? err.message : '加载记忆失败';
		} finally {
			isLoading = false;
		}
	}

	async function handleSearch() {
		if (!searchQuery.trim()) {
			await loadMemories();
			return;
		}

		try {
			isLoading = true;
			error = null;

			// 调用搜索API
			const response = await api.memory.search(searchQuery);

			// 转换搜索结果
			memories = response.results.map((result: any) => {
				// 处理编码问题
				let content = result.memory.content;
				try {
					if (content.includes('ç') || content.includes('æ') || content.includes('å')) {
						const decoder = new TextDecoder('utf-8');
						const encoder = new TextEncoder();
						const bytes = encoder.encode(content);
						content = decoder.decode(bytes);
					}
				} catch (decodeError) {
					console.warn('解码搜索内容失败:', decodeError);
				}

				return {
					id: result.memory.id,
					content: content,
					type: result.memory.metadata.memory_type.toLowerCase(),
					importance: result.score, // 使用相似度分数作为重要性
					userId: result.memory.metadata.user_id,
					agentId: result.memory.metadata.agent_id,
					createdAt: result.memory.created_at,
					updatedAt: result.memory.updated_at
				};
			});

			// 重置到第一页
			currentPage = 1;
		} catch (err) {
			console.error('搜索记忆失败:', err);
			error = err instanceof Error ? err.message : '搜索失败';
		} finally {
			isLoading = false;
		}
	}

	function getTypeColor(type: string) {
		switch (type) {
			case 'conversational':
				return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
			case 'factual':
				return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
			case 'personal':
				return 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300';
			case 'procedural':
				return 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300';
			default:
				return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
		}
	}

	function getTypeLabel(type: string) {
		switch (type) {
			case 'conversational':
				return $t('memories.conversational');
			case 'factual':
				return $t('memories.factual');
			case 'personal':
				return $t('memories.personal');
			case 'procedural':
				return $t('memories.procedural');
			default:
				return $t('memories.unknown');
		}
	}

	function formatImportance(importance: number) {
		return importance.toFixed(2);
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
					minute: '2-digit',
					second: '2-digit'
				})
				.replace(/\//g, '-')
				.replace(',', '');
		} catch {
			return isoString;
		}
	}
	function getImportanceColor(importance: number) {
		if (importance >= 0.9) return 'text-red-600 dark:text-red-400';
		if (importance >= 0.7) return 'text-orange-600 dark:text-orange-400';
		if (importance >= 0.5) return 'text-yellow-600 dark:text-yellow-400';
		return 'text-green-600 dark:text-green-400';
	}

	// 过滤和排序记忆 - 使用响应式变量
	$: filteredMemories = (() => {
		let result = [...memories];

		// 搜索过滤
		if (searchQuery) {
			const query = searchQuery.toLowerCase();
			result = result.filter(
				(memory) =>
					memory.content.toLowerCase().includes(query) ||
					memory.id.toLowerCase().includes(query) ||
					(memory.userId && memory.userId.toLowerCase().includes(query)) ||
					(memory.agentId && memory.agentId.toLowerCase().includes(query))
			);
		}

		// 类型过滤
		if (selectedType !== 'all') {
			result = result.filter((memory) => memory.type === selectedType);
		}

		// 排序
		result.sort((a, b) => {
			let aValue: any, bValue: any;

			switch (sortBy) {
				case 'importance':
					aValue = a.importance;
					bValue = b.importance;
					break;
				case 'createdAt':
					aValue = new Date(a.createdAt).getTime();
					bValue = new Date(b.createdAt).getTime();
					break;
				case 'updatedAt':
					aValue = new Date(a.updatedAt).getTime();
					bValue = new Date(b.updatedAt).getTime();
					break;
				default:
					aValue = a.id;
					bValue = b.id;
			}

			if (sortOrder === 'asc') {
				return aValue > bValue ? 1 : -1;
			} else {
				return aValue < bValue ? 1 : -1;
			}
		});

		return result;
	})();

	// 计算分页数据
	$: {
		totalPages = Math.ceil(filteredMemories.length / pageSize);
		const startIndex = (currentPage - 1) * pageSize;
		const endIndex = Math.min(startIndex + pageSize, filteredMemories.length);
		paginatedMemories = filteredMemories.slice(startIndex, endIndex);
		console.log('分页数据更新:', {
			currentPage,
			totalPages,
			totalItems: filteredMemories.length,
			pageItems: paginatedMemories.length
		});
	}

	function toggleSort(column: string) {
		if (sortBy === column) {
			sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
		} else {
			sortBy = column;
			sortOrder = 'desc';
		}
		// 显式触发响应式更新 - 使用临时变量技巧
		const newSortBy = sortBy;
		const newSortOrder = sortOrder;
		sortBy = '';
		sortOrder = sortOrder === 'asc' ? 'asc' : 'desc';
		sortBy = newSortBy;
		sortOrder = newSortOrder;

		// 排序变化时重置到第一页
		currentPage = 1;
	}

	function getSortIcon(column: string) {
		if (sortBy !== column) return '↓';
		return sortOrder === 'asc' ? '↑' : '↓';
	}

	// 分页函数
	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			currentPage = page;
		}
	}

	function nextPage() {
		if (currentPage < totalPages) {
			currentPage++;
		}
	}

	function prevPage() {
		if (currentPage > 1) {
			currentPage--;
		}
	}

	// 弹窗功能
	function showFullContent(content: string, memoryId: string) {
		selectedContent = content;
		selectedMemoryId = memoryId;
		showContentModal = true;
	}

	function hideContentModal() {
		showContentModal = false;
		selectedContent = '';
		selectedMemoryId = '';
	}

	// 选择功能
	function toggleSelectMemory(memoryId: string) {
		// 创建新的Set以确保响应式更新
		const newSelection = new Set(selectedMemories);
		if (newSelection.has(memoryId)) {
			newSelection.delete(memoryId);
		} else {
			newSelection.add(memoryId);
		}
		selectedMemories = newSelection;
		showBatchOperations = selectedMemories.size > 0;
	}

	function selectAll() {
		// 直接创建新的Set而不是修改现有Set
		const newSelection = new Set(selectedMemories); // 保留之前的选择
		paginatedMemories.forEach((memory) => newSelection.add(memory.id));
		selectedMemories = newSelection;
		showBatchOperations = selectedMemories.size > 0;
	}

	function deselectAll() {
		// 创建新的空Set
		selectedMemories = new Set();
		showBatchOperations = false;
	}

	// 创建响应式的选中状态映射
	$: selectedMemoryMap = new Map();
	$: {
		console.log('选择状态变化:', {
			selectedCount: selectedMemories.size,
			totalCount: filteredMemories.length,
			pageCount: paginatedMemories.length,
			isAllSelected,
			isPartialSelected,
			selectedIds: Array.from(selectedMemories).slice(0, 3) // 只显示前3个用于调试
		});

		// 为每个当前页的memory创建选中状态映射
		const map = new Map();
		paginatedMemories.forEach((memory) => {
			map.set(memory.id, selectedMemories.has(memory.id));
		});
		selectedMemoryMap = map;
		console.log('selectedMemoryMap已更新:', selectedMemoryMap.size);
	}
	// 批量操作功能
	async function batchExport() {
		const selected = filteredMemories.filter((memory) => selectedMemories.has(memory.id));
		const exportData = selected.map((memory) => ({
			id: memory.id,
			content: memory.content,
			type: memory.type,
			importance: memory.importance,
			userId: memory.userId,
			agentId: memory.agentId,
			createdAt: memory.createdAt,
			updatedAt: memory.updatedAt
		}));

		const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `memories-export-${new Date().toISOString().split('T')[0]}.json`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);

		console.log(`已导出 ${selected.length} 条记忆`);
		deselectAll();
	}

	async function batchOptimize() {
		const selected = filteredMemories.filter((memory) => selectedMemories.has(memory.id));

		if (!confirm(`确定要优化选中的 ${selected.length} 条记忆吗？`)) {
			return;
		}

		try {
			// 优化功能：在内容后添加优化标记
			const updates = selected.map((memory) => ({
				id: memory.id,
				content: `${memory.content}\n[已优化 ${new Date().toLocaleDateString()}]`
			}));

			// 使用批量更新API
			const result = await api.memory.batchUpdate(updates);
			console.log(`批量优化结果:`, result);
			console.log(`已优化 ${selected.length} 条记忆`);
			await loadMemories(); // 重新加载数据
			deselectAll();
		} catch (err) {
			console.error('批量优化失败:', err);
			alert(`批量优化失败: ${err instanceof Error ? err.message : '未知错误'}`);
		}
	}

	async function batchDelete() {
		const selected = filteredMemories.filter((memory) => selectedMemories.has(memory.id));
		const memoryIds = selected.map((memory) => memory.id);

		if (!confirm(`确定要删除选中的 ${selected.length} 条记忆吗？此操作不可撤销。`)) {
			return;
		}

		try {
			// 使用批量删除API
			await api.memory.batchDelete(memoryIds);
			console.log(`已删除 ${selected.length} 条记忆`);
			await loadMemories(); // 重新加载数据
			deselectAll();
		} catch (err) {
			console.error('批量删除失败:', err);
			alert(`批量删除失败: ${err instanceof Error ? err.message : '未知错误'}`);
		}
	}
</script>

<div class="max-w-[95vw] mx-auto space-y-6">
	<!-- 页面标题 -->
	<div>
		<h1 class="text-3xl font-bold text-gray-900 dark:text-white">{$t('memories.title')}</h1>
		<p class="mt-2 text-gray-600 dark:text-gray-400">{$t('memories.description')}</p>
	</div>

			<!-- 错误显示 -->
			{#if error}
				<div
					class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4"
				>
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<span class="text-red-500">⚠️</span>
						</div>
						<div class="ml-3">
							<h3 class="text-sm font-medium text-red-800 dark:text-red-300">{$t('memories.loadFailed')}</h3>
							<div class="mt-1 text-sm text-red-700 dark:text-red-400">
								{error}
							</div>
							<div class="mt-3">
								<button
									type="button"
									class="text-sm font-medium text-red-800 dark:text-red-300 hover:text-red-900 dark:hover:text-red-200"
									on:click={loadMemories}
								>
									{$t('memories.retry')}
								</button>
							</div>
						</div>
					</div>
				</div>
			{/if}
	<!-- 搜索和过滤栏 -->
	<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
		<div class="grid grid-cols-1 md:grid-cols-4 gap-4">
			<!-- 搜索框 -->
			<div class="md:col-span-2">
				<div class="relative">
					<div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
						<span class="text-gray-400">🔍</span>
					</div>
					<input
						type="text"
						bind:value={searchQuery}
						placeholder={$t('memories.searchPlaceholder')}
						class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
						on:keydown={(e) => {
							if (e.key === 'Enter') {
								handleSearch();
							}
						}}
					/>
				</div>
			</div>

			<!-- 类型过滤 -->
			<div>
				<select
					bind:value={selectedType}
					class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
				>
					{#each memoryTypes as type}
						<option value={type.value}>{type.label}</option>
					{/each}
				</select>
			</div>

			<!-- 操作按钮 -->
			<div class="flex space-x-2">
				<button
					class="flex-1 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
					on:click={handleSearch}
				>
					{$t('memories.search')}
				</button>
				<button
					class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors duration-200"
					on:click={() => {
						searchQuery = '';
						selectedType = 'all';
						sortBy = 'createdAt';
						sortOrder = 'desc';
						loadMemories();
					}}
				>
					{$t('memories.reset')}
				</button>
			</div>
		</div>

		<!-- 统计信息 -->
		<div class="mt-4 flex items-center justify-between text-sm text-gray-500 dark:text-gray-400">
			<span>
				{$t('memories.totalMemories')}: <span class="font-medium text-gray-700 dark:text-gray-300"
					>{filteredMemories.length}</span
				>
				, {$t('memories.showing')}
				<span class="font-medium text-gray-700 dark:text-gray-300"
					>{(currentPage - 1) * pageSize + 1}</span
				>
				{$t('memories.to')}
				<span class="font-medium text-gray-700 dark:text-gray-300"
					>{Math.min(currentPage * pageSize, filteredMemories.length)}</span
				>
				{$t('memories.of')} {filteredMemories.length}
			</span>
			<div class="flex items-center space-x-4">
				<span>{$t('memories.sort')}:</span>
				<div class="flex space-x-2">
					<button
						class={`px-3 py-1 rounded ${sortBy === 'createdAt' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400'}`}
						on:click={() => toggleSort('createdAt')}
					>
						{$t('memories.createdAt')} {createdAtSortIcon}
					</button>
					<button
						class={`px-3 py-1 rounded ${sortBy === 'importance' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400'}`}
						on:click={() => toggleSort('importance')}
					>
						{$t('memories.importance')} {importanceSortIcon}
					</button>
				</div>
			</div>
		</div>
	</div>

			<!-- 批量操作栏 -->
			{#if showBatchOperations}
				<div
					class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl p-4"
				>
					<div class="flex items-center justify-between">
						<div class="flex items-center space-x-4">
							<span class="text-sm font-medium text-blue-800 dark:text-blue-300">
								{$t('memories.batchOperations')}: <span class="font-bold">{selectedMemories.size}</span> {$t('memories.totalMemories')}
							</span>
							<button
								class="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300"
								on:click={deselectAll}
							>
								{$t('memories.selectAll')}
							</button>
						</div>
						<div class="flex flex-wrap gap-2">
							<button
								class="px-3 py-1 bg-blue-500 hover:bg-blue-600 text-white text-sm rounded font-medium transition-colors duration-200"
								on:click={batchExport}
							>
								📤 {$t('memories.exportSelected')}
							</button>
	
							<button
								class="px-3 py-1 bg-yellow-500 hover:bg-yellow-600 text-white text-sm rounded font-medium transition-colors duration-200"
								on:click={batchOptimize}
							>
								⚡ {$t('memories.optimizeSelected')}
							</button>
							<button
								class="px-3 py-1 bg-red-500 hover:bg-red-600 text-white text-sm rounded font-medium transition-colors duration-200"
								on:click={batchDelete}
							>
								🗑️ {$t('memories.deleteSelected')}
							</button>
						</div>
					</div>
				</div>
			{/if}
	<!-- 记忆列表 -->
	<div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm overflow-hidden">
		{#if isLoading}
			<!-- 加载状态 -->
			<div class="p-8">
				<div class="space-y-4">
					{#each Array(5) as _, i}
						<div class="h-20 bg-gray-100 dark:bg-gray-700 rounded animate-pulse"></div>
					{/each}
				</div>
			</div>
		{:else if filteredMemories.length === 0}
			<!-- 空状态 -->
			<div class="p-12 text-center">
				<div
					class="w-16 h-16 mx-auto mb-4 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center"
				>
					<span class="text-2xl">📭</span>
				</div>
				<h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">{$t('memories.noMemoriesFound')}</h3>
				<p class="text-gray-500 dark:text-gray-400 mb-6">
					{searchQuery || selectedType !== 'all' ? $t('memories.adjustSearch') : $t('memories.noMemoriesInSystem')}
				</p>
				{#if searchQuery || selectedType !== 'all'}
					<button
						class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
						on:click={() => {
							searchQuery = '';
							selectedType = 'all';
						}}
					>
						{$t('memories.clearFilters')}
					</button>
				{/if}
			</div>
		{:else if paginatedMemories.length === 0}
			<!-- 当前页无数据状态 -->
			<div class="p-12 text-center">
				<div
					class="w-16 h-16 mx-auto mb-4 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center"
				>
					<span class="text-2xl">📄</span>
				</div>
				<h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">{$t('memories.noDataOnCurrentPage')}</h3>
				<p class="text-gray-500 dark:text-gray-400 mb-6">
					{$t('memories.page')} {currentPage} {$t('memories.checkPageOrFilters')}
				</p>
				<button
					class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
					on:click={() => goToPage(1)}
				>
					{$t('memories.goToFirstPage')}
				</button>
			</div>
		{:else}
			<!-- 记忆表格 -->
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead class="bg-gray-50 dark:bg-gray-900/50">
						<tr>
							<th
								class="w-6 px-3 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								<input
									type="checkbox"
									class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
									checked={isAllSelected}
									on:change={(e) => {
										if (e.currentTarget.checked) {
											selectAll();
										} else {
											deselectAll();
										}
									}}
								/>
							</th>
							<th
								class="w-32 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								ID
							</th>
							<th
								class="w-1/2 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								{$t('memories.content')}
							</th>
							<th
								class="w-24 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								{$t('memories.type')}
							</th>
							<th
								class="w-28 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								{$t('memories.importance')}
							</th>
							<th
								class="w-32 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								{$t('memories.userAgent')}
							</th>
							<th
								class="w-40 px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
							>
								{$t('memories.created')}
							</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-gray-200 dark:divide-gray-700">
						{#each paginatedMemories as memory}
							<tr class="hover:bg-gray-50 dark:hover:bg-gray-900/30 transition-colors duration-150">
								<td class="w-6 px-3 py-3 whitespace-nowrap">
									<input
										type="checkbox"
										class="rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
										checked={selectedMemoryMap.get(memory.id) || false}
										on:change={() => toggleSelectMemory(memory.id)}
									/>
								</td>
								<td class="w-32 px-6 py-4 whitespace-nowrap">
									<div class="text-sm font-medium text-gray-900 dark:text-white truncate">
										{memory.id}
									</div>
								</td>
								<td class="w-1/2 px-6 py-4">
									<div class="max-w-none">
										<button
											class="text-sm text-gray-900 dark:text-white truncate-2-lines cursor-pointer hover:text-blue-600 dark:hover:text-blue-400 transition-colors duration-150 text-left w-full"
											on:click={() => showFullContent(memory.content, memory.id)}
											on:keydown={(e) => {
												if (e.key === 'Enter' || e.key === ' ') {
													e.preventDefault();
													showFullContent(memory.content, memory.id);
												}
											}}
											title={$t('memories.clickToViewFullContent')}
											type="button"
										>
											{memory.content}
										</button>
										{#if memory.content.length > 100}
											<div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
												{$t('memories.clickToViewFullContent')} ({memory.content.length} {$t('memories.characters')})
											</div>
										{/if}
									</div>
								</td>
								<td class="w-24 px-6 py-4 whitespace-nowrap">
									<span
										class={`px-2 py-1 text-xs font-medium rounded-full ${getTypeColor(memory.type)}`}
									>
										{getTypeLabel(memory.type)}
									</span>
								</td>
								<td class="w-28 px-6 py-4 whitespace-nowrap">
									<div class="flex items-center">
										<span class={`text-sm font-medium ${getImportanceColor(memory.importance)}`}>
											{formatImportance(memory.importance)}
										</span>
									</div>
								</td>
								<td class="w-32 px-6 py-4 whitespace-nowrap">
									<div class="text-sm text-gray-500 dark:text-gray-400">
										{#if memory.userId}
											<div class="truncate">{memory.userId}</div>
										{/if}
										{#if memory.agentId}
											<div class="truncate">Agent: {memory.agentId}</div>
										{/if}
									</div>
								</td>
								<td
									class="w-40 px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400"
								>
									{formatDate(memory.createdAt)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- 分页 -->
			{#if totalPages > 1}
				<div
					class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700"
				>
					<div class="flex items-center justify-between">
						<div class="text-sm text-gray-500 dark:text-gray-400">
							{$t('memories.showing')} <span class="font-medium">{(currentPage - 1) * pageSize + 1}</span> {$t('memories.to')}
							<span class="font-medium"
								>{Math.min(currentPage * pageSize, filteredMemories.length)}</span
							>
							{$t('memories.of')} <span class="font-medium">{filteredMemories.length}</span>, {$t('memories.page')}
							<span class="font-medium">{currentPage}</span>
							/ {totalPages}
						</div>
						<div class="flex items-center space-x-2">
							<button
								class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
								disabled={currentPage === 1}
								on:click={prevPage}
							>
								{$t('memories.previousPage')}
							</button>

							<!-- 页码按钮 -->
							{#each Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
								const startPage = Math.max(1, currentPage - 2);
								const endPage = Math.min(totalPages, startPage + 4);
								return startPage + i;
							}) as page}
								{#if page <= totalPages}
									<button
										class={`px-3 py-1 border rounded text-sm font-medium ${
											page === currentPage
												? 'bg-blue-500 text-white border-blue-500'
												: 'border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700'
										}`}
										on:click={() => goToPage(page)}
									>
										{page}
									</button>
								{/if}
							{/each}

							<button
								class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
								disabled={currentPage === totalPages}
								on:click={nextPage}
							>
								{$t('memories.nextPage')}
							</button>
						</div>
					</div>
				</div>
			{/if}
		{/if}
	</div>

	<!-- 内容弹窗 -->
	{#if showContentModal}
		<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
			<div
				class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-4xl w-full max-h-[80vh] overflow-hidden"
			>
				<div
					class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700"
				>
					<div>
						<h3 class="text-lg font-semibold text-gray-900 dark:text-white">{$t('memories.fullContent')}</h3>
						<p class="text-sm text-gray-500 dark:text-gray-400 mt-1">ID: {selectedMemoryId}</p>
					</div>
					<button
						class="px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors duration-150"
						on:click={hideContentModal}
					>
						<span class="text-2xl text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
							>×</span
						>
					</button>
				</div>
				<div class="p-6 overflow-y-auto max-h-[60vh]">
					<div class="prose prose-gray dark:prose-invert max-w-none">
						<pre
							class="whitespace-pre-wrap text-sm text-gray-900 dark:text-gray-100 font-sans leading-relaxed">{selectedContent}</pre>
					</div>
				</div>
				<div class="flex justify-end p-6 border-t border-gray-200 dark:border-gray-700">
					<button
						class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors duration-200"
						on:click={hideContentModal}
					>
						{$t('memories.close')}
					</button>
				</div>
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
