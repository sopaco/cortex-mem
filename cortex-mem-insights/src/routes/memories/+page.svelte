<script lang="ts">
  import { onMount } from 'svelte';
  
  let memories: Array<{
    id: string;
    content: string;
    type: string;
    importance: number;
    userId?: string;
    agentId?: string;
    createdAt: string;
    updatedAt: string;
  }> = [];
  
  let isLoading = true;
  let searchQuery = '';
  let selectedType = 'all';
  let sortBy = 'createdAt';
  let sortOrder: 'asc' | 'desc' = 'desc';
  
  const memoryTypes = [
    { value: 'all', label: '全部类型' },
    { value: 'conversational', label: '对话' },
    { value: 'factual', label: '事实' },
    { value: 'personal', label: '个人' },
    { value: 'procedural', label: '流程' }
  ];
  
  onMount(async () => {
    // 模拟加载数据
    setTimeout(() => {
      memories = Array.from({ length: 20 }, (_, i) => ({
        id: `mem_${String(i + 1).padStart(3, '0')}`,
        content: `记忆内容示例 ${i + 1}: 这是第 ${i + 1} 条记忆记录，包含一些重要的用户偏好或系统配置信息。`,
        type: ['conversational', 'factual', 'personal', 'procedural'][i % 4],
        importance: 0.5 + Math.random() * 0.5,
        userId: i % 3 === 0 ? 'user_001' : i % 3 === 1 ? 'user_002' : undefined,
        agentId: i % 2 === 0 ? 'agent_001' : 'agent_002',
        createdAt: `2025-12-${String(13 - Math.floor(i / 3)).padStart(2, '0')} ${String(10 + i % 8).padStart(2, '0')}:${String(30 + i % 30).padStart(2, '0')}`,
        updatedAt: `2025-12-${String(13 - Math.floor(i / 5)).padStart(2, '0')} ${String(14 + i % 6).padStart(2, '0')}:${String(15 + i % 45).padStart(2, '0')}`
      }));
      
      isLoading = false;
    }, 1500);
  });
  
  function getTypeColor(type: string) {
    switch (type) {
      case 'conversational': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
      case 'factual': return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
      case 'personal': return 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300';
      case 'procedural': return 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
    }
  }
  
  function getTypeLabel(type: string) {
    switch (type) {
      case 'conversational': return '对话';
      case 'factual': return '事实';
      case 'personal': return '个人';
      case 'procedural': return '流程';
      default: return '未知';
    }
  }
  
  function formatImportance(importance: number) {
    return (importance * 100).toFixed(1) + '%';
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
      result = result.filter(memory => 
        memory.content.toLowerCase().includes(query) ||
        memory.id.toLowerCase().includes(query) ||
        (memory.userId && memory.userId.toLowerCase().includes(query)) ||
        (memory.agentId && memory.agentId.toLowerCase().includes(query))
      );
    }
    
    // 类型过滤
    if (selectedType !== 'all') {
      result = result.filter(memory => memory.type === selectedType);
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
  });
  
  function toggleSort(column: string) {
    if (sortBy === column) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortOrder = 'desc';
    }
  }
  
  function getSortIcon(column: string) {
    if (sortBy !== column) return '↕️';
    return sortOrder === 'asc' ? '↑' : '↓';
  }
</script>

<div class="space-y-6">
  <!-- 页面标题 -->
  <div>
    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">记忆浏览器</h1>
    <p class="mt-2 text-gray-600 dark:text-gray-400">
      浏览、搜索和管理所有记忆记录
    </p>
  </div>

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
            placeholder="搜索记忆内容、ID、用户或Agent..."
            class="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
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
          on:click={() => console.log('高级搜索')}
        >
          高级搜索
        </button>
        <button
          class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors duration-200"
          on:click={() => {
            searchQuery = '';
            selectedType = 'all';
            sortBy = 'createdAt';
            sortOrder = 'desc';
          }}
        >
          重置
        </button>
      </div>
    </div>
    
    <!-- 统计信息 -->
    <div class="mt-4 flex items-center justify-between text-sm text-gray-500 dark:text-gray-400">
      <span>
        共 <span class="font-medium text-gray-700 dark:text-gray-300">{memories.length}</span> 条记忆，
        显示 <span class="font-medium text-gray-700 dark:text-gray-300">{filteredMemories.length}</span> 条
      </span>
      <div class="flex items-center space-x-4">
        <span>排序:</span>
        <div class="flex space-x-2">
          <button
            class={`px-3 py-1 rounded ${sortBy === 'createdAt' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400'}`}
            on:click={() => toggleSort('createdAt')}
          >
            创建时间 {getSortIcon('createdAt')}
          </button>
          <button
            class={`px-3 py-1 rounded ${sortBy === 'importance' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400'}`}
            on:click={() => toggleSort('importance')}
          >
            重要性 {getSortIcon('importance')}
          </button>
        </div>
      </div>
    </div>
  </div>

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
        <div class="w-16 h-16 mx-auto mb-4 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center">
          <span class="text-2xl">📭</span>
        </div>
        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">未找到记忆记录</h3>
        <p class="text-gray-500 dark:text-gray-400 mb-6">
          {searchQuery || selectedType !== 'all' ? '尝试调整搜索条件' : '系统暂无记忆记录'}
        </p>
        <button
          class="px-6 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
          on:click={() => {
            searchQuery = '';
            selectedType = 'all';
          }}
        >
          {searchQuery || selectedType !== 'all' ? '清除筛选条件' : '添加测试记忆'}
        </button>
      </div>
    {:else}
      <!-- 记忆表格 -->
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-gray-50 dark:bg-gray-900/50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                ID
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                内容
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                类型
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                重要性
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                用户/Agent
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                创建时间
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                操作
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each filteredMemories as memory}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-900/30 transition-colors duration-150">
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm font-medium text-gray-900 dark:text-white">
                    {memory.id}
                  </div>
                </td>
                <td class="px-6 py-4">
                  <div class="max-w-md">
                    <div class="text-sm text-gray-900 dark:text-white truncate-2-lines">
                      {memory.content}
                    </div>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class={`px-2 py-1 text-xs font-medium rounded-full ${getTypeColor(memory.type)}`}>
                    {getTypeLabel(memory.type)}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center">
                    <div class="w-16 bg-gray-200 dark:bg-gray-700 rounded-full h-2 mr-2">
                      <div
                        class={`h-2 rounded-full ${getImportanceColor(memory.importance)}`}
                        style={`width: ${memory.importance * 100}%`}
                      ></div>
                    </div>
                    <span class={`text-sm font-medium ${getImportanceColor(memory.importance)}`}>
                      {formatImportance(memory.importance)}
                    </span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm text-gray-500 dark:text-gray-400">
                    {#if memory.userId}
                      <div>用户: {memory.userId}</div>
                    {/if}
                    {#if memory.agentId}
                      <div>Agent: {memory.agentId}</div>
                    {/if}
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                  {memory.createdAt}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium">
                  <div class="flex space-x-2">
                    <button
                      class="text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300"
                      on:click={() => console.log('查看详情', memory.id)}
                    >
                      查看
                    </button>
                    <button
                      class="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300"
                      on:click={() => console.log('编辑', memory.id)}
                    >
                      编辑
                    </button>
                    <button
                      class="text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
                      on:click={() => console.log('删除', memory.id)}
                    >
                      删除
                    </button>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      
      <!-- 分页 -->
      <div class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <div class="text-sm text-gray-500 dark:text-gray-400">
            显示第 <span class="font-medium">1</span> 到 <span class="font-medium">{Math.min(filteredMemories.length, 20)}</span> 条，
            共 <span class="font-medium">{filteredMemories.length}</span> 条
          </div>
          <div class="flex space-x-2">
            <button
              class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
              disabled
            >
              上一页
            </button>
            <button
              class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            >
              1
            </button>
            <button
              class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            >
              2
            </button>
            <button
              class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            >
              3
            </button>
            <button
              class="px-3 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
            >
              下一页
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- 批量操作 -->
  <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
    <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">批量操作</h3>
    <div class="flex flex-wrap gap-3">
      <button
        class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
        on:click={() => console.log('批量导出')}
      >
        批量导出
      </button>
      <button
        class="px-4 py-2 bg-green-500 hover:bg-green-600 text-white rounded-lg font-medium transition-colors duration-200"
        on:click={() => console.log('批量标记')}
      >
        批量标记
      </button>
      <button
        class="px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-white rounded-lg font-medium transition-colors duration-200"
        on:click={() => console.log('批量优化')}
      >
        批量优化
      </button>
      <button
        class="px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg font-medium transition-colors duration-200"
        on:click={() => console.log('批量删除')}
      >
        批量删除
      </button>
    </div>
  </div>
</div>

<style>
  .truncate-2-lines {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>