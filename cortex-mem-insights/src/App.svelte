<script lang="ts">
  import Dashboard from './lib/pages/Dashboard.svelte';
  import Memories from './lib/pages/Memories.svelte';
  import Search from './lib/pages/Search.svelte';
  
  let currentPath = $state(window.location.pathname);
  
  function navigate(path: string) {
    window.history.pushState({}, '', path);
    currentPath = path;
  }
  
  function isActive(path: string): boolean {
    if (path === '/') return currentPath === '/';
    return currentPath.startsWith(path);
  }
</script>

<div class="app-container">
  <nav class="navbar">
    <div class="nav-brand">
      <span class="brand-icon">🧠</span>
      <span class="brand-text">Cortex Mem</span>
    </div>
    <div class="nav-links">
      <a href="/" class:active={isActive('/') && currentPath === '/'} onclick={(e) => { e.preventDefault(); navigate('/'); }}>
        Dashboard
      </a>
      <a href="/memories" class:active={isActive('/memories')} onclick={(e) => { e.preventDefault(); navigate('/memories'); }}>
        Memories
      </a>
      <a href="/search" class:active={isActive('/search')} onclick={(e) => { e.preventDefault(); navigate('/search'); }}>
        Search
      </a>
    </div>
  </nav>

  <main class="main-content">
    {#if currentPath === '/'}
      <Dashboard />
    {:else if currentPath.startsWith('/memories')}
      <Memories />
    {:else if currentPath.startsWith('/search')}
      <Search />
    {:else}
      <div class="empty-state">
        <div class="empty-state-icon">🔍</div>
        <div class="empty-state-title">Page not found</div>
        <div class="empty-state-description">The page you're looking for doesn't exist.</div>
      </div>
    {/if}
  </main>
</div>
