<script lang="ts">
  import { language, setLanguage, t } from '$lib/i18n';
  import { languageOptions } from '$lib/i18n';
  
  let showDropdown = false;
  
  function toggleDropdown() {
    showDropdown = !showDropdown;
  }
  
  function selectLanguage(lang: 'en' | 'zh' | 'ja') {
    setLanguage(lang);
    showDropdown = false;
  }
  
  // 点击外部关闭下拉菜单
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.language-switcher')) {
      showDropdown = false;
    }
  }
  
  // 添加全局点击监听
  if (typeof window !== 'undefined') {
    window.addEventListener('click', handleClickOutside);
  }
  
  // 组件销毁时移除监听
  import { onDestroy } from 'svelte';
  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class="language-switcher relative">
  <button
    type="button"
    class="flex items-center space-x-2 px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
    on:click={toggleDropdown}
    aria-label="Change language"
    aria-expanded={showDropdown}
  >
    <span class="w-5 h-5">
      {#if $language === 'en'}
        🇺🇸
      {:else if $language === 'zh'}
        🇨🇳
      {:else if $language === 'ja'}
        🇯🇵
      {/if}
    </span>
    <span class="hidden sm:inline">{$t('common.language')}</span>
    <svg
      class={`w-4 h-4 transition-transform ${showDropdown ? 'rotate-180' : ''}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>
  
  {#if showDropdown}
    <div class="absolute right-0 mt-2 w-48 bg-white dark:bg-gray-800 rounded-lg shadow-lg py-1 z-50 border border-gray-200 dark:border-gray-700">
      {#each languageOptions as option}
        <button
          type="button"
          class={`flex items-center w-full px-4 py-2 text-sm ${
            $language === option.value
              ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400'
              : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
          }`}
          on:click={() => selectLanguage(option.value)}
        >
          <span class="w-5 h-5 mr-3">
            {#if option.value === 'en'}
              🇺🇸
            {:else if option.value === 'zh'}
              🇨🇳
            {:else if option.value === 'ja'}
              🇯🇵
            {/if}
          </span>
          <span>{option.label}</span>
          {#if $language === option.value}
            <svg class="w-4 h-4 ml-auto text-blue-500" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
            </svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .language-switcher {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  }
</style>