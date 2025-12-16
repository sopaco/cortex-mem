<script lang="ts">
  import { onMount } from 'svelte';
  import { optimizationApi } from '$lib/api/client';
  
  let isLoading = true;
  let isOptimizing = false;
  let optimizationProgress = 0;
  let optimizationStatus = 'idle'; // idle, analyzing, executing, completed, failed
  let currentJobId: string | null = null;
  let pollInterval: number | null = null;
  let errorMessage: string | null = null;
  
  // 优化策略
  const strategies = [
    { id: 'full', name: '全面优化', description: '检测并处理所有类型的问题', estimatedTime: '60分钟' },
    { id: 'deduplication', name: '去重优化', description: '仅处理重复记忆', estimatedTime: '20分钟' },
    { id: 'quality', name: '质量优化', description: '处理低质量记忆', estimatedTime: '30分钟' },
    { id: 'relevance', name: '相关性优化', description: '优化记忆相关性', estimatedTime: '25分钟' }
  ];
  
  let selectedStrategy = 'full';
  let previewMode = true;
  let aggressiveMode = false;
  let timeoutMinutes = 30;
  
  // 优化历史
  let optimizationHistory = [
    { id: 'opt_001', strategy: '全面优化', status: 'completed', startedAt: '2025-12-13 10:30', duration: '45分钟', memoriesAffected: 124, spaceSaved: '15.2MB' },
    { id: 'opt_002', strategy: '去重优化', status: 'completed', startedAt: '2025-12-12 14:15', duration: '18分钟', memoriesAffected: 56, spaceSaved: '8.7MB' },
    { id: 'opt_003', strategy: '质量优化', status: 'failed', startedAt: '2025-12-11 09:45', duration: '32分钟', memoriesAffected: 0, spaceSaved: '0MB', error: 'LLM服务超时' },
    { id: 'opt_004', strategy: '全面优化', status: 'completed', startedAt: '2025-12-10 16:20', duration: '52分钟', memoriesAffected: 198, spaceSaved: '22.1MB' }
  ];
  
  // 检测到的问题
  let detectedIssues = [
    { type: '重复记忆', count: 45, severity: 'high', description: '语义相似度超过85%的记忆' },
    { type: '低质量记忆', count: 89, severity: 'medium', description: '重要性评分低于50%的记忆' },
    { type: '过时记忆', count: 23, severity: 'low', description: '超过30天未更新的记忆' },
    { type: '分类不当', count: 12, severity: 'low', description: '类型与内容不匹配的记忆' }
  ];
  
  onMount(async () => {
    // 加载优化历史和检测问题
    await loadOptimizationData();
    isLoading = false;
  });

  async function loadOptimizationData(skipAnalyze = false) {
    try {
      // 加载优化历史
      const historyResponse = await optimizationApi.history({ limit: 10 });
      if (historyResponse.success && historyResponse.data) {
        optimizationHistory = historyResponse.data.history.map((h: any) => ({
          id: h.job_id,
          strategy: h.strategy || '未知',
          status: h.status,
          startedAt: new Date(h.start_time).toLocaleString('zh-CN'),
          duration: h.duration ? `${Math.floor(h.duration / 60000)}分钟` : '未知',
          memoriesAffected: h.memories_affected || 0,
          spaceSaved: h.space_saved ? `${h.space_saved.toFixed(1)}MB` : '0MB',
        }));
      }

      // 分析检测问题（可选，避免重复分析）
      if (!skipAnalyze) {
        const analyzeResponse = await optimizationApi.analyze({});
        if (analyzeResponse.success && analyzeResponse.data) {
          const data = analyzeResponse.data;
          if (data.issues && Array.isArray(data.issues)) {
            detectedIssues = data.issues.map((issue: any) => ({
              type: issue.kind || issue.type || '未知问题',
              count: issue.affected_memories?.length || 0,
              severity: issue.severity?.toLowerCase() || 'low',
              description: issue.description || '',
            }));
          }
        }
      }
    } catch (error) {
      console.error('加载优化数据失败:', error);
      errorMessage = '加载数据失败，请刷新页面重试';
    }
  }
  
  function getStatusColor(status: string) {
    switch (status) {
      case 'completed': return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300';
      case 'running': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
      case 'failed': return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
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
  
  async function startOptimization() {
    if (isOptimizing) return;
    
    errorMessage = null;
    isOptimizing = true;
    optimizationStatus = 'analyzing';
    optimizationProgress = 0;
    
    try {
      // 启动优化任务
      const response = await optimizationApi.optimize({
        strategy: selectedStrategy,
        dry_run: previewMode,
        aggressive: aggressiveMode,
        timeout_minutes: timeoutMinutes,
      });
      
      if (!response.success || !response.data) {
        throw new Error(response.error?.message || '启动优化失败');
      }
      
      currentJobId = response.data.job_id;
      
      // 开始轮询状态
      startPolling();
    } catch (error) {
      console.error('启动优化失败:', error);
      errorMessage = error instanceof Error ? error.message : '启动优化失败';
      isOptimizing = false;
      optimizationStatus = 'failed';
    }
  }
  
  function startPolling() {
    if (!currentJobId) return;
    
    // 每2秒轮询一次
    pollInterval = window.setInterval(async () => {
      if (!currentJobId) {
        stopPolling();
        return;
      }
      
      try {
        const response = await optimizationApi.getStatus(currentJobId);
        
        if (!response.success || !response.data) {
          throw new Error('获取状态失败');
        }
        
        const jobState = response.data;
        optimizationProgress = jobState.progress || 0;
        
        // 更新状态
        if (jobState.status === 'running') {
          optimizationStatus = jobState.current_phase?.includes('分析') ? 'analyzing' : 'executing';
        } else if (jobState.status === 'completed') {
          optimizationStatus = 'completed';
          isOptimizing = false;
          stopPolling();
          
          // 刷新历史记录（跳过分析，避免重复调用）
          await loadOptimizationData(true);
        } else if (jobState.status === 'failed' || jobState.status === 'cancelled') {
          optimizationStatus = 'failed';
          isOptimizing = false;
          stopPolling();
          errorMessage = jobState.logs?.[jobState.logs.length - 1] || '优化失败';
        }
      } catch (error) {
        console.error('轮询状态失败:', error);
        stopPolling();
        isOptimizing = false;
        optimizationStatus = 'failed';
        errorMessage = '获取优化状态失败';
      }
    }, 2000);
  }
  
  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }
  
  async function cancelOptimization() {
    if (!currentJobId) return;
    
    try {
      await optimizationApi.cancel(currentJobId);
      stopPolling();
      isOptimizing = false;
      optimizationStatus = 'failed';
      optimizationProgress = 0;
      currentJobId = null;
    } catch (error) {
      console.error('取消优化失败:', error);
      errorMessage = '取消优化失败';
    }
  }
  
  function getEstimatedImpact() {
    const base = selectedStrategy === 'full' ? 150 : 
                 selectedStrategy === 'deduplication' ? 60 :
                 selectedStrategy === 'quality' ? 90 : 75;
    
    const multiplier = aggressiveMode ? 1.5 : 1;
    return Math.floor(base * multiplier);
  }
</script>

<div class="space-y-8">
  <!-- 页面标题 -->
  <div>
    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">优化面板</h1>
    <p class="mt-2 text-gray-600 dark:text-gray-400">
      检测和优化记忆数据，提升系统性能和信息密度
    </p>
  </div>

  <!-- 错误提示 -->
  {#if errorMessage}
    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
      <div class="flex items-center">
        <svg class="w-5 h-5 text-red-600 dark:text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"/>
        </svg>
        <span class="text-red-800 dark:text-red-200">{errorMessage}</span>
        <button 
          on:click={() => errorMessage = null}
          class="ml-auto text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-200"
        >
          ✕
        </button>
      </div>
    </div>
  {/if}

  {#if isLoading}
    <!-- 加载状态 -->
    <div class="space-y-6">
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse">
        <div class="h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-6"></div>
        <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded"></div>
      </div>
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {#each Array(2) as _, i}
          <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse">
            <div class="h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-6"></div>
            <div class="space-y-4">
              {#each Array(3) as _, j}
                <div class="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <!-- 优化控制面板 -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">优化控制</h2>
      
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <!-- 策略选择 -->
        <div>
          <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">优化策略</h3>
          <div class="space-y-3">
            {#each strategies as strategy}
              <label class="flex items-start p-3 border rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-900/30 transition-colors duration-150
                {selectedStrategy === strategy.id ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-700'}">
                <input
                  type="radio"
                  name="strategy"
                  value={strategy.id}
                  bind:group={selectedStrategy}
                  class="mt-1 mr-3"
                />
                <div class="flex-1">
                  <div class="font-medium text-gray-900 dark:text-white">
                    {strategy.name}
                  </div>
                  <div class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                    {strategy.description}
                  </div>
                  <div class="text-xs text-gray-400 dark:text-gray-500 mt-2">
                    预计时间: {strategy.estimatedTime}
                  </div>
                </div>
              </label>
            {/each}
          </div>
        </div>
        
        <!-- 选项配置 -->
        <div>
          <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">优化选项</h3>
          <div class="space-y-4">
            <label class="flex items-center justify-between p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div>
                <div class="font-medium text-gray-900 dark:text-white">预览模式</div>
                <div class="text-sm text-gray-500 dark:text-gray-400">
                  仅分析问题，不执行优化
                </div>
              </div>
              <input
                type="checkbox"
                bind:checked={previewMode}
                class="w-5 h-5 rounded"
                disabled={isOptimizing}
              />
            </label>
            
            <label class="flex items-center justify-between p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div>
                <div class="font-medium text-gray-900 dark:text-white">激进模式</div>
                <div class="text-sm text-gray-500 dark:text-gray-400">
                  更严格的优化标准
                </div>
              </div>
              <input
                type="checkbox"
                bind:checked={aggressiveMode}
                class="w-5 h-5 rounded"
                disabled={isOptimizing}
              />
            </label>
            
            <div class="p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div class="font-medium text-gray-900 dark:text-white mb-2">超时时间</div>
              <div class="flex items-center space-x-4">
                <input
                  type="range"
                  min="10"
                  max="120"
                  step="5"
                  bind:value={timeoutMinutes}
                  class="flex-1"
                  disabled={isOptimizing}
                />
                <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                  {timeoutMinutes} 分钟
                </span>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 预估影响 -->
        <div>
          <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-4">预估影响</h3>
          <div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-4 space-y-3">
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">预计影响记忆:</span>
              <span class="font-medium text-gray-900 dark:text-white">
                ~{getEstimatedImpact()} 条
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">预计节省空间:</span>
              <span class="font-medium text-green-600 dark:text-green-400">
                ~{(getEstimatedImpact() * 0.15).toFixed(1)}MB
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">预计提升质量:</span>
              <span class="font-medium text-blue-600 dark:text-blue-400">
                +{aggressiveMode ? '15' : '10'}%
              </span>
            </div>
            <div class="pt-3 border-t border-gray-200 dark:border-gray-700">
              <div class="text-sm text-gray-500 dark:text-gray-400">
                {previewMode ? '预览模式不会实际修改数据' : '优化将永久修改记忆数据'}
              </div>
            </div>
          </div>
          
          <!-- 操作按钮 -->
          <div class="mt-6 space-y-3">
            {#if isOptimizing}
              <button
                on:click={cancelOptimization}
                class="w-full px-4 py-3 bg-red-500 hover:bg-red-600 text-white rounded-lg font-medium transition-colors duration-200"
              >
                取消优化
              </button>
            {:else}
              <button
                on:click={startOptimization}
                class="w-full px-4 py-3 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium transition-colors duration-200"
              >
                {previewMode ? '分析问题' : '开始优化'}
              </button>
            {/if}
            
            <button
              on:click={() => console.log('导出报告')}
              class="w-full px-4 py-3 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium transition-colors duration-200"
            >
              导出优化报告
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 优化进度 -->
    {#if isOptimizing}
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">优化进度</h2>
        
        <div class="space-y-6">
          <!-- 进度条 -->
          <div>
            <div class="flex justify-between mb-2">
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                {optimizationStatus === 'analyzing' ? '分析问题中...' :
                 optimizationStatus === 'executing' ? '执行优化中...' :
                 optimizationStatus === 'completed' ? '优化完成' : '优化失败'}
              </span>
              <span class="text-sm font-medium text-gray-900 dark:text-white">
                {optimizationProgress}%
              </span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
              <div
                class="h-3 rounded-full bg-blue-500 transition-all duration-300"
                style={`width: ${optimizationProgress}%`}
              ></div>
            </div>
          </div>
          
          <!-- 状态信息 -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
              <div class="text-sm text-blue-700 dark:text-blue-300">当前阶段</div>
              <div class="text-lg font-medium text-blue-900 dark:text-blue-100 mt-1">
                {optimizationStatus === 'analyzing' ? '问题分析' :
                 optimizationStatus === 'executing' ? '执行优化' :
                 optimizationStatus === 'completed' ? '完成' : '失败'}
              </div>
            </div>
            
            <div class="p-4 bg-green-50 dark:bg-green-900/20 rounded-lg">
              <div class="text-sm text-green-700 dark:text-green-300">已处理记忆</div>
              <div class="text-lg font-medium text-green-900 dark:text-green-100 mt-1">
                {Math.floor(optimizationProgress * 1.5)} 条
              </div>
            </div>
            
            <div class="p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
              <div class="text-sm text-purple-700 dark:text-purple-300">预计剩余时间</div>
              <div class="text-lg font-medium text-purple-900 dark:text-purple-100 mt-1">
                {Math.max(0, Math.floor((100 - optimizationProgress) * 0.3))} 分钟
              </div>
            </div>
          </div>
          
          <!-- 实时日志 -->
          <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
            <div class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">实时日志</div>
            <div class="space-y-2 max-h-40 overflow-y-auto">
              {#each Array(Math.floor(optimizationProgress / 10)) as _, i}
                <div class="text-sm text-gray-600 dark:text-gray-400">
                  [{new Date(Date.now() - (10 - i) * 1000).toLocaleTimeString('zh-CN', {hour12: false})}] 
                  {optimizationStatus === 'analyzing' ? '分析记忆 #' + (i * 10 + 1) + '...' :
                   '优化记忆 #' + (i * 10 + 1) + '...'}
                </div>
              {/each}
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- 检测到的问题 -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white">检测到的问题</h2>
        <button
          on:click={() => console.log('重新检测')}
          class="px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg text-sm font-medium"
        >
          重新检测
        </button>
      </div>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {#each detectedIssues as issue}
          <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-gray-300 dark:hover:border-gray-600 transition-colors duration-150">
            <div class="flex items-center justify-between mb-2">
              <span class={`px-2 py-1 rounded text-xs font-medium ${getSeverityColor(issue.severity)}`}>
                {issue.severity === 'high' ? '高' : issue.severity === 'medium' ? '中' : '低'}
              </span>
              <span class="text-2xl font-bold text-gray-900 dark:text-white">
                {issue.count}
              </span>
            </div>
            <div class="font-medium text-gray-900 dark:text-white mb-1">
              {issue.type}
            </div>
            <div class="text-sm text-gray-500 dark:text-gray-400">
              {issue.description}
            </div>
            <div class="mt-3">
              <button
                on:click={() => console.log('查看详情', issue.type)}
                class="w-full px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded"
              >
                查看详情
              </button>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- 优化历史 -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">优化历史</h2>
      
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-gray-50 dark:bg-gray-900/50">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                优化ID
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                策略
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                状态
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                开始时间
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                耗时
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                影响记忆
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                节省空间
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                操作
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each optimizationHistory as record}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-900/30">
                <td class="px-4 py-3">
                  <div class="font-mono text-sm text-gray-900 dark:text-white">
                    {record.id}
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div class="text-sm text-gray-700 dark:text-gray-300">
                    {record.strategy}
                  </div>
                </td>
                <td class="px-4 py-3">
                  <span class={`px-2 py-1 rounded text-xs font-medium ${getStatusColor(record.status)}`}>
                    {record.status === 'completed' ? '完成' : 
                     record.status === 'running' ? '进行中' : '失败'}
                  </span>
                </td>
                <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                  {record.startedAt}
                </td>
                <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                  {record.duration}
                </td>
                <td class="px-4 py-3">
                  <div class="text-sm font-medium text-gray-900 dark:text-white">
                    {record.memoriesAffected}
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div class="text-sm font-medium text-green-600 dark:text-green-400">
                    {record.spaceSaved}
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div class="flex space-x-2">
                    <button
                      on:click={() => console.log('查看报告', record.id)}
                      class="text-sm text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300"
                    >
                      报告
                    </button>
                    {#if record.status === 'completed'}
                      <button
                        on:click={() => console.log('撤销', record.id)}
                        class="text-sm text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
                      >
                        撤销
                      </button>
                    {/if}
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      
      <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <div class="text-sm text-gray-500 dark:text-gray-400">
            共 {optimizationHistory.length} 次优化记录
          </div>
          <button
            on:click={() => console.log('清空历史')}
            class="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg"
          >
            清空历史记录
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>