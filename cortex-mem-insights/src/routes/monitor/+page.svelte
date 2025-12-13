<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  
  let isLoading = true;
  let autoRefresh = true;
  let refreshInterval: number;
  
  // 系统状态
  let systemStatus = {
    cortexMemService: { status: 'connected', latency: 45, version: '1.0.0' },
    qdrant: { status: 'connected', latency: 28, version: '1.7.0', collectionCount: 3 },
    llmService: { status: 'connected', latency: 320, provider: 'OpenAI', model: 'gpt-4' },
    memoryUsage: { used: 245, total: 1024, percentage: 24 },
    cpuUsage: { percentage: 18 },
    network: { activeConnections: 12, throughput: '1.2 MB/s' }
  };
  
  // 性能指标
  let performanceMetrics = [
    { name: 'API响应时间', value: 145, unit: 'ms', trend: 'down', threshold: 500 },
    { name: '搜索延迟', value: 230, unit: 'ms', trend: 'stable', threshold: 1000 },
    { name: '记忆写入', value: 420, unit: 'ms', trend: 'up', threshold: 2000 },
    { name: '优化执行', value: 1850, unit: 'ms', trend: 'stable', threshold: 5000 }
  ];
  
  // 实时日志
  let realtimeLogs = [
    { time: '14:30:25', level: 'info', message: '记忆检索请求: user_001, 结果: 12条' },
    { time: '14:30:18', level: 'info', message: '新增记忆: ID mem_1246, 类型: Personal' },
    { time: '14:29:55', level: 'warning', message: 'LLM API延迟较高: 420ms' },
    { time: '14:29:30', level: 'info', message: '健康检查通过: 所有服务正常' },
    { time: '14:28:45', level: 'error', message: 'Qdrant连接超时，已重试成功' }
  ];
  
  // 告警
  let alerts = [
    { id: 'alert_001', level: 'warning', message: '内存使用率超过80%', time: '14:25:30', acknowledged: false },
    { id: 'alert_002', level: 'error', message: 'LLM服务响应超时', time: '14:20:15', acknowledged: true },
    { id: 'alert_003', level: 'info', message: '备份任务完成', time: '14:15:00', acknowledged: true }
  ];
  
  onMount(() => {
    // 模拟加载数据
    setTimeout(() => {
      isLoading = false;
    }, 1000);
    
    // 设置自动刷新
    if (autoRefresh) {
      refreshInterval = setInterval(() => {
        updateMetrics();
      }, 5000);
    }
  });
  
  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });
  
  function updateMetrics() {
    // 模拟更新指标
    systemStatus = {
      ...systemStatus,
      memoryUsage: {
        ...systemStatus.memoryUsage,
        used: systemStatus.memoryUsage.used + Math.random() * 10 - 5,
        percentage: ((systemStatus.memoryUsage.used + Math.random() * 10 - 5) / systemStatus.memoryUsage.total * 100)
      },
      cpuUsage: {
        percentage: 15 + Math.random() * 10
      }
    };
    
    performanceMetrics = performanceMetrics.map(metric => ({
      ...metric,
      value: metric.value + Math.random() * 20 - 10
    }));
    
    // 添加新日志
    const now = new Date();
    const newLog = {
      time: now.toLocaleTimeString('zh-CN', {hour12: false}),
      level: Math.random() > 0.8 ? 'warning' : 'info',
      message: `系统检查: ${['内存正常', '连接稳定', '服务健康'][Math.floor(Math.random() * 3)]}`
    };
    
    realtimeLogs.unshift(newLog);
    if (realtimeLogs.length > 20) {
      realtimeLogs.pop();
    }
  }
  
  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    
    if (autoRefresh) {
      refreshInterval = setInterval(() => {
        updateMetrics();
      }, 5000);
    } else if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  }
  
  function getStatusColor(status: string) {
    switch (status) {
      case 'connected': return 'text-green-500';
      case 'connecting': return 'text-yellow-500';
      case 'disconnected': return 'text-red-500';
      default: return 'text-gray-500';
    }
  }
  
  function getLevelColor(level: string) {
    switch (level) {
      case 'error': return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300';
      case 'warning': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300';
      case 'info': return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
    }
  }
  
  function getTrendIcon(trend: string) {
    switch (trend) {
      case 'up': return '↗️';
      case 'down': return '↘️';
      default: return '➡️';
    }
  }
  
  function getTrendColor(trend: string) {
    switch (trend) {
      case 'up': return 'text-red-500';
      case 'down': return 'text-green-500';
      default: return 'text-gray-500';
    }
  }
  
  function acknowledgeAlert(alertId: string) {
    const alert = alerts.find(a => a.id === alertId);
    if (alert) {
      alert.acknowledged = true;
    }
  }
</script>

<div class="space-y-8">
  <!-- 页面标题 -->
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold text-gray-900 dark:text-white">系统监控</h1>
      <p class="mt-2 text-gray-600 dark:text-gray-400">
        实时监控系统状态、性能指标和运行日志
      </p>
    </div>
    <div class="flex items-center space-x-4">
      <label class="flex items-center space-x-2">
        <input
          type="checkbox"
          bind:checked={autoRefresh}
          on:change={toggleAutoRefresh}
          class="w-4 h-4 rounded"
        />
        <span class="text-sm text-gray-700 dark:text-gray-300">自动刷新</span>
      </label>
      <button
        on:click={updateMetrics}
        class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium"
      >
        立即刷新
      </button>
    </div>
  </div>

  {#if isLoading}
    <!-- 加载状态 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      {#each Array(3) as _, i}
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse">
          <div class="h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-6"></div>
          <div class="space-y-4">
            {#each Array(3) as _, j}
              <div class="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- 系统状态概览 -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- 服务状态 -->
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">服务状态</h2>
        
        <div class="space-y-4">
          {#each Object.entries(systemStatus).slice(0, 3) as [service, data]}
            <div class="p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div class="flex items-center justify-between mb-2">
                <div class="flex items-center space-x-2">
                  <div class={`w-2 h-2 rounded-full ${getStatusColor(data.status)}`}></div>
                  <span class="font-medium text-gray-900 dark:text-white">
                    {service === 'cortexMemService' ? 'cortex-mem-service' : 
                     service === 'qdrant' ? 'Qdrant 数据库' : 
                     'LLM 服务'}
                  </span>
                </div>
                <span class={`text-sm font-medium ${getStatusColor(data.status)}`}>
                  {data.status === 'connected' ? '已连接' : 
                   data.status === 'connecting' ? '连接中' : '已断开'}
                </span>
              </div>
              
              <div class="grid grid-cols-2 gap-2 text-sm text-gray-600 dark:text-gray-400">
                <div>延迟: <span class="font-medium">{data.latency}ms</span></div>
                <div>
                  {service === 'cortexMemService' ? `版本: ${data.version}` :
                   service === 'qdrant' ? `集合: ${data.collectionCount}` :
                   `模型: ${data.model}`}
                </div>
              </div>
            </div>
          {/each}
        </div>
        
        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <button
            on:click={() => console.log('检查所有服务')}
            class="w-full px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium"
          >
            检查所有服务
          </button>
        </div>
      </div>

      <!-- 资源使用 -->
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">资源使用</h2>
        
        <div class="space-y-6">
          <!-- 内存使用 -->
          <div>
            <div class="flex justify-between mb-2">
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">内存使用</span>
              <span class="text-sm font-medium text-gray-900 dark:text-white">
                {systemStatus.memoryUsage.percentage.toFixed(1)}%
              </span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
              <div
                class={`h-3 rounded-full ${
                  systemStatus.memoryUsage.percentage > 80 ? 'bg-red-500' :
                  systemStatus.memoryUsage.percentage > 60 ? 'bg-yellow-500' :
                  'bg-green-500'
                }`}
                style={`width: ${systemStatus.memoryUsage.percentage}%`}
              ></div>
            </div>
            <div class="flex justify-between mt-1 text-sm text-gray-500 dark:text-gray-400">
              <span>{systemStatus.memoryUsage.used.toFixed(1)} MB</span>
              <span>{systemStatus.memoryUsage.total} MB</span>
            </div>
          </div>
          
          <!-- CPU使用 -->
          <div>
            <div class="flex justify-between mb-2">
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">CPU使用</span>
              <span class="text-sm font-medium text-gray-900 dark:text-white">
                {systemStatus.cpuUsage.percentage.toFixed(1)}%
              </span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
              <div
                class={`h-3 rounded-full ${
                  systemStatus.cpuUsage.percentage > 70 ? 'bg-red-500' :
                  systemStatus.cpuUsage.percentage > 40 ? 'bg-yellow-500' :
                  'bg-green-500'
                }`}
                style={`width: ${systemStatus.cpuUsage.percentage}%`}
              ></div>
            </div>
          </div>
          
          <!-- 网络 -->
          <div class="p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
            <div class="text-sm font-medium text-gray-900 dark:text-white mb-2">网络状态</div>
            <div class="grid grid-cols-2 gap-2 text-sm text-gray-600 dark:text-gray-400">
              <div>活跃连接: <span class="font-medium">{systemStatus.network.activeConnections}</span></div>
              <div>吞吐量: <span class="font-medium">{systemStatus.network.throughput}</span></div>
            </div>
          </div>
        </div>
        
        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <button
            on:click={() => console.log('资源优化')}
            class="w-full px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium"
          >
            资源优化建议
          </button>
        </div>
      </div>

      <!-- 性能指标 -->
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">性能指标</h2>
        
        <div class="space-y-4">
          {#each performanceMetrics as metric}
            <div class="p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div class="flex items-center justify-between mb-2">
                <span class="font-medium text-gray-900 dark:text-white">
                  {metric.name}
                </span>
                <div class="flex items-center space-x-2">
                  <span class={`text-sm ${getTrendColor(metric.trend)}`}>
                    {getTrendIcon(metric.trend)}
                  </span>
                  <span class="text-lg font-bold text-gray-900 dark:text-white">
                    {metric.value.toFixed(0)}{metric.unit}
                  </span>
                </div>
              </div>
              
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  class={`h-2 rounded-full ${
                    metric.value > metric.threshold * 0.8 ? 'bg-red-500' :
                    metric.value > metric.threshold * 0.6 ? 'bg-yellow-500' :
                    'bg-green-500'
                  }`}
                  style={`width: ${(metric.value / metric.threshold) * 100}%`}
                ></div>
              </div>
              
              <div class="flex justify-between mt-1 text-xs text-gray-500 dark:text-gray-400">
                <span>阈值: {metric.threshold}{metric.unit}</span>
                <span>使用率: {((metric.value / metric.threshold) * 100).toFixed(1)}%</span>
              </div>
            </div>
          {/each}
        </div>
        
        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <button
            on:click={() => console.log('性能报告')}
            class="w-full px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium"
          >
            生成性能报告
          </button>
        </div>
      </div>
    </div>

    <!-- 告警和日志 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
      <!-- 告警 -->
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">系统告警</h2>
          <span class="px-2 py-1 bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 rounded text-sm font-medium">
            {alerts.filter(a => !a.acknowledged).length} 个未处理
          </span>
        </div>
        
        <div class="space-y-3">
          {#each alerts as alert}
            <div class={`p-3 border rounded-lg ${
              alert.acknowledged ? 'border-gray-200 dark:border-gray-700' : 'border-red-200 dark:border-red-700'
            }`}>
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <div class="flex items-center space-x-2 mb-1">
                    <span class={`px-2 py-1 rounded text-xs font-medium ${getLevelColor(alert.level)}`}>
                      {alert.level === 'error' ? '错误' : 
                       alert.level === 'warning' ? '警告' : '信息'}
                    </span>
                    {#if !alert.acknowledged}
                      <span class="px-2 py-1 bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300 rounded text-xs">
                        未处理
                      </span>
                    {/if}
                  </div>
                  <p class="text-sm text-gray-900 dark:text-white">
                    {alert.message}
                  </p>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    {alert.time}
                  </p>
                </div>
                {#if !alert.acknowledged}
                  <button
                    on:click={() => acknowledgeAlert(alert.id)}
                    class="ml-2 px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded"
                  >
                    确认
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
        
        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <div class="flex space-x-3">
            <button
              on:click={() => console.log('查看所有告警')}
              class="flex-1 px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium"
            >
              查看所有告警
            </button>
            <button
              on:click={() => console.log('清空已处理')}
              class="flex-1 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium"
            >
              清空已处理
            </button>
          </div>
        </div>
      </div>

      <!-- 实时日志 -->
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
        <div class="flex items-center justify-between mb-6">
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white">实时日志</h2>
          <div class="flex items-center space-x-2">
            <span class="text-sm text-gray-500 dark:text-gray-400">
              最后更新: {new Date().toLocaleTimeString('zh-CN', {hour12: false})}
            </span>
            <button
              on:click={() => realtimeLogs = []}
              class="px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded"
            >
              清空
            </button>
          </div>
        </div>
        
        <div class="h-64 overflow-y-auto border border-gray-200 dark:border-gray-700 rounded-lg p-4">
          {#if realtimeLogs.length === 0}
            <div class="h-full flex items-center justify-center text-gray-500 dark:text-gray-400">
              暂无日志
            </div>
          {:else}
            <div class="space-y-2">
              {#each realtimeLogs as log}
                <div class="flex items-start space-x-3 text-sm">
                  <div class="flex-shrink-0 w-16 text-gray-500 dark:text-gray-400">
                    {log.time}
                  </div>
                  <div class="flex-shrink-0">
                    <span class={`px-2 py-0.5 rounded text-xs ${getLevelColor(log.level)}`}>
                      {log.level === 'error' ? 'ERR' : 
                       log.level === 'warning' ? 'WARN' : 'INFO'}
                    </span>
                  </div>
                  <div class="flex-1 text-gray-900 dark:text-white">
                    {log.message}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
        
        <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
          <div class="flex space-x-3">
            <button
              on:click={() => console.log('导出日志')}
              class="flex-1 px-4 py-2 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg font-medium"
            >
              导出日志
            </button>
            <button
              on:click={() => console.log('日志设置')}
              class="flex-1 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg font-medium"
            >
              日志设置
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 监控工具 -->
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6">
      <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-6">监控工具</h2>
      
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <button
          class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-700 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all duration-200"
          on:click={() => console.log('健康检查')}
        >
          <div class="flex items-center space-x-3">
            <div class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center">
              <span class="text-xl">❤️</span>
            </div>
            <div class="text-left">
              <p class="font-medium text-gray-900 dark:text-white">健康检查</p>
              <p class="text-sm text-gray-500 dark:text-gray-400">全面检查系统健康状态</p>
            </div>
          </div>
        </button>
        
        <button
          class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-green-300 dark:hover:border-green-700 hover:bg-green-50 dark:hover:bg-green-900/20 transition-all duration-200"
          on:click={() => console.log('性能测试')}
        >
          <div class="flex items-center space-x-3">
            <div class="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-lg flex items-center justify-center">
              <span class="text-xl">⚡</span>
            </div>
            <div class="text-left">
              <p class="font-medium text-gray-900 dark:text-white">性能测试</p>
              <p class="text-sm text-gray-500 dark:text-gray-400">运行性能基准测试</p>
            </div>
          </div>
        </button>
        
        <button
          class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 hover:border-purple-300 dark:hover:border-purple-700 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-all duration-200"
          on:click={() => console.log('诊断工具')}
        >
          <div class="flex items-center space-x-3">
            <div class="w-10 h-10 bg-purple-100 dark:bg-purple-900/30 rounded-lg flex items-center justify-center">
              <span class="text-xl">🔧</span>
            </div>
            <div class="text-left">
              <p class="font-medium text-gray-900 dark:text-white">诊断工具</p>
              <p class="text-sm text-gray-500 dark:text-gray-400">系统问题诊断和修复</p>
            </div>
          </div>
        </button>
      </div>
    </div>
  {/if}
</div>