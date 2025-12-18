<script lang="ts">
  import { onMount } from 'svelte';
  import { memoryApi } from '$lib/api/client';

  export let issue: {
    type: string;
    count: number;
    severity: string;
    description: string;
    affected_memories?: string[];
  } | null = null;
  
  export let onClose: () => void;

  let memories: any[] = [];
  let isLoading = true;
  let error: string | null = null;

  $: if (issue) {
    loadMemoryDetails();
  }

  async function loadMemoryDetails() {
    if (!issue || !issue.affected_memories || issue.affected_memories.length === 0) {
      isLoading = false;
      return;
    }

    isLoading = true;
    error = null;
    memories = [];

    try {
      // 逐个获取memory详情
      const memoryPromises = issue.affected_memories.map(id => 
        memoryApi.get(id).catch(err => {
          console.error(`获取memory ${id} 失败:`, err);
          return null;
        })
      );

      const results = await Promise.all(memoryPromises);
      memories = results.filter(m => m !== null);
    } catch (err) {
      console.error('加载记忆详情失败:', err);
      error = err instanceof Error ? err.message : '加载失败';
    } finally {
      isLoading = false;
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function getSeverityColor(severity: string) {
    switch (severity) {
      case 'high': return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300';
      case 'medium': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300';
      case 'low': return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
    }
  }

  function formatDate(dateString: string) {
    try {
      return new Date(dateString).toLocaleString('zh-CN');
    } catch {
      return dateString;
    }
  }

  function getMemoryTypeLabel(type: string) {
    const typeMap: Record<string, string> = {
      'Conversational': '对话',
      'Factual': '事实',
      'Personal': '个人',
      'Procedural': '流程',
      'Episodic': '情节',
    };
    return typeMap[type] || type;
  }
</script>

{#if issue}
  <!-- 模态框背景 -->
  <div
    class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
    on:click={handleBackdropClick}
    on:keydown={(e) => e.key === 'Escape' && onClose()}
    role="button"
    tabindex="0"
  >
    <!-- 模态框内容 -->
    <div
      class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col"
      on:click|stopPropagation
      role="dialog"
      aria-modal="true"
    >
      <!-- 头部 -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <div class="flex items-center space-x-3">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
            {issue.type} - 详细信息
          </h2>
          <span class={`px-2 py-1 rounded text-xs font-medium ${getSeverityColor(issue.severity)}`}>
            {issue.severity === 'high' ? '高' : issue.severity === 'medium' ? '中' : '低'}
          </span>
        </div>
        <button
          on:click={onClose}
          class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
          </svg>
        </button>
      </div>

      <!-- 问题描述 -->
      <div class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
        <div class="text-sm text-gray-600 dark:text-gray-400 mb-2">问题描述</div>
        <div class="text-gray-900 dark:text-white">{issue.description}</div>
        <div class="mt-2 text-sm text-gray-500 dark:text-gray-400">
          影响记忆数量: <span class="font-medium text-gray-900 dark:text-white">{issue.count}</span> 条
        </div>
      </div>

      <!-- 记忆列表 -->
      <div class="flex-1 overflow-y-auto px-6 py-4">
        {#if isLoading}
          <div class="flex items-center justify-center py-12">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            <span class="ml-3 text-gray-600 dark:text-gray-400">加载记忆详情...</span>
          </div>
        {:else if error}
          <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
            <div class="flex items-center">
              <svg class="w-5 h-5 text-red-600 dark:text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
              </svg>
              <span class="text-red-800 dark:text-red-200">{error}</span>
            </div>
          </div>
        {:else if memories.length === 0}
          <div class="text-center py-12 text-gray-500 dark:text-gray-400">
            暂无记忆详情
          </div>
        {:else}
          <div class="space-y-4">
            {#each memories as memory, index}
              <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:border-gray-300 dark:hover:border-gray-600 transition-colors">
                <!-- 记忆头部信息 -->
                <div class="flex items-start justify-between mb-3">
                  <div class="flex items-center space-x-2">
                    <span class="text-sm font-medium text-gray-500 dark:text-gray-400">#{index + 1}</span>
                    <span class="px-2 py-1 bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300 rounded text-xs font-medium">
                      {getMemoryTypeLabel(memory.metadata?.memory_type || 'Unknown')}
                    </span>
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400 font-mono">
                    ID: {memory.id}
                  </div>
                </div>

                <!-- 记忆内容 -->
                <div class="mb-3">
                  <div class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">内容</div>
                  <div class="text-gray-900 dark:text-white bg-gray-50 dark:bg-gray-900/50 rounded p-3 text-sm">
                    {memory.content}
                  </div>
                </div>

                <!-- 元数据 -->
                <div class="grid grid-cols-2 gap-3 text-xs">
                  {#if memory.metadata?.user_id}
                    <div>
                      <span class="text-gray-500 dark:text-gray-400">用户ID:</span>
                      <span class="ml-1 text-gray-900 dark:text-white font-mono">{memory.metadata.user_id}</span>
                    </div>
                  {/if}
                  {#if memory.metadata?.agent_id}
                    <div>
                      <span class="text-gray-500 dark:text-gray-400">代理ID:</span>
                      <span class="ml-1 text-gray-900 dark:text-white font-mono">{memory.metadata.agent_id}</span>
                    </div>
                  {/if}
                  {#if memory.created_at}
                    <div>
                      <span class="text-gray-500 dark:text-gray-400">创建时间:</span>
                      <span class="ml-1 text-gray-900 dark:text-white">{formatDate(memory.created_at)}</span>
                    </div>
                  {/if}
                  {#if memory.updated_at}
                    <div>
                      <span class="text-gray-500 dark:text-gray-400">更新时间:</span>
                      <span class="ml-1 text-gray-900 dark:text-white">{formatDate(memory.updated_at)}</span>
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- 底部操作栏 -->
      <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3">
        <button
          on:click={onClose}
          class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors"
        >
          关闭
        </button>
      </div>
    </div>
  </div>
{/if}
