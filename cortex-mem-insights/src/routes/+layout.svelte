<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  
  // 导航菜单项
  const navItems = [
    { name: '仪表盘', href: '/', icon: 'dashboard' },
    { name: '记忆浏览器', href: '/memories', icon: 'memory' },
    { name: '统计分析', href: '/analytics', icon: 'analytics' },
    { name: '优化面板', href: '/optimization', icon: 'optimize' },
    { name: '系统监控', href: '/monitor', icon: 'monitor' },
  ];
</script>

<div class="min-h-screen bg-gray-50 dark:bg-gray-900">
  <!-- 导航栏 -->
  <nav class="bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex justify-between h-16">
        <div class="flex">
          <!-- Logo -->
          <div class="flex-shrink-0 flex items-center">
            <div class="flex items-center space-x-2">
              <div class="w-8 h-8 bg-blue-500 rounded-lg flex items-center justify-center">
                <span class="text-white font-bold">CM</span>
              </div>
              <span class="text-xl font-bold text-gray-900 dark:text-white">
                cortex-mem-insights
              </span>
            </div>
          </div>
          
          <!-- 导航菜单 -->
          <div class="hidden sm:ml-6 sm:flex sm:space-x-8">
            {#each navItems as item}
              <a
                href={item.href}
                class="inline-flex items-center px-1 pt-1 border-b-2 text-sm font-medium transition-colors duration-200
                  {page.url.pathname === item.href
                    ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                    : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300 hover:border-gray-300 dark:hover:border-gray-600'}"
              >
                <span class="mr-2">📊</span>
                {item.name}
              </a>
            {/each}
          </div>
        </div>
        
        <!-- 右侧操作 -->
        <div class="flex items-center space-x-4">
          <button
            class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium transition-colors duration-200"
            on:click={() => console.log('刷新数据')}
          >
            刷新数据
          </button>
          <div class="relative">
            <div class="w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded-full flex items-center justify-center cursor-pointer">
              <span class="text-gray-600 dark:text-gray-300">👤</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </nav>

  <!-- 主内容区域 -->
  <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <slot />
  </main>

  <!-- 页脚 -->
  <footer class="bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 mt-8">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
      <div class="flex justify-between items-center">
        <div class="text-sm text-gray-500 dark:text-gray-400">
          © 2025 cortex-mem-insights. 版本 0.1.0
        </div>
        <div class="text-sm text-gray-500 dark:text-gray-400">
          连接到: <span class="text-green-500 font-medium">cortex-mem-service</span>
        </div>
      </div>
    </div>
  </footer>
</div>

<style>
  /* 自定义样式 */
  .nav-icon {
    font-size: 1.2rem;
  }
</style>