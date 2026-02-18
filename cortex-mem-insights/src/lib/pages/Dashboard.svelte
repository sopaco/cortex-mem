<script lang="ts">
  import { onMount } from 'svelte';
  import apiClient from '../api';
  import type { HealthStatus, SessionInfo } from '../types';
  import { tenants, tenantInfo, currentTenant, initTenants, switchTenant, loadTenantInfo, tenantLoading } from '../stores/tenant';
  import TenantSelector from '../components/TenantSelector.svelte';

  let health = $state<HealthStatus | null>(null);
  let loading = $state(true);
  let error = $state('');
  let refreshing = $state(false);
  let loadingTenantInfo = $state(false);

  // Helper to format file size
  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  async function loadData() {
    loading = true;
    error = '';
    
    try {
      const [healthData, tenantList] = await Promise.all([
        apiClient.getHealth(),
        apiClient.listTenants()
      ]);
      
      health = healthData;
      tenants.set(tenantList);
      
      // Load info for each tenant sequentially (not in parallel to avoid race conditions)
      for (const tenant of tenantList) {
        try {
          await loadTenantInfo(tenant);
        } catch (e) {
          console.error(`Failed to load info for tenant ${tenant}:`, e);
        }
      }
      
      // Auto-select first tenant if available
      if (tenantList.length > 0 && !$currentTenant) {
        await switchTenant(tenantList[0]);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load data';
    } finally {
      loading = false;
    }
  }

  async function handleTenantSelect(tenantId: string) {
    loadingTenantInfo = true;
    try {
      await switchTenant(tenantId);
      await loadData();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to switch tenant';
    } finally {
      loadingTenantInfo = false;
    }
  }

  onMount(() => {
    loadData();
  });

  function handleRefresh() {
    refreshing = true;
    loadData();
  }
</script>

<h1>Dashboard</h1>

<!-- Tenant Overview -->
<div class="section">
  <h2>Tenants Overview</h2>
  <div class="table-container">
    <table class="tenant-table">
      <thead>
        <tr>
          <th>Tenant ID</th>
          <th>👤 User Memories</th>
          <th>💬 Sessions</th>
          <th>🤖 Agents</th>
          <th>📚 Resources</th>
          <th>📄 Total Files</th>
          <th>💾 Storage Size</th>
          <th>Action</th>
        </tr>
      </thead>
      <tbody>
        {#each $tenants as tenant}
          {@const info = $tenantInfo.get(tenant)}
          <tr class:selected={$currentTenant === tenant}>
            <td class="tenant-id">{tenant}</td>
            <td>{info?.userCount ?? '-'}</td>
            <td>{info?.sessionCount ?? '-'}</td>
            <td>{info?.agentCount ?? '-'}</td>
            <td>{info?.totalFiles ? Math.max(0, info.totalFiles - (info.userCount || 0) - (info.sessionCount || 0) - (info.agentCount || 0)) : '-'}</td>
            <td>{info?.totalFiles ?? '-'}</td>
            <td>{info?.totalSize ? formatSize(info.totalSize) : '-'}</td>
            <td>
              <button 
                class="btn btn-sm btn-secondary"
                onclick={() => handleTenantSelect(tenant)}
              >
                Select
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<!-- Current Tenant Details -->
{#if $currentTenant}
  <div class="section">
    <div class="section-header">
      <h2>Current Tenant: {$currentTenant}</h2>
      <button class="refresh-btn" class:loading={refreshing} onclick={handleRefresh} disabled={refreshing}>
        {refreshing ? '⟳' : '↻'}
      </button>
    </div>

    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <span>Loading...</span>
      </div>
    {:else if error}
      <div class="error-message">{error}</div>
    {:else}
      <!-- Service Status -->
      <div class="health-grid">
        <div class="health-card">
          <div class="health-header">
            <span class="health-title">Service Status</span>
          </div>
          {#if health}
            <div style="display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem;">
              <span class="status-badge" class:healthy={health.status === 'healthy'} class:unhealthy={health.status !== 'healthy'}>
                <span class="status-dot"></span>
                {health.status === 'healthy' ? 'Healthy' : 'Unhealthy'}
              </span>
            </div>
            <div style="font-size: 0.875rem; color: var(--text-secondary);">
              <div><strong>Version:</strong> {health.version}</div>
              <div><strong>LLM Available:</strong> {health.llm_available ? 'Yes' : 'No'}</div>
            </div>
          {/if}
        </div>

        <div class="health-card">
          <div class="health-header">
            <span class="health-title">Storage</span>
          </div>
          {#if $tenantInfo.get($currentTenant)}
            {@const info = $tenantInfo.get($currentTenant)}
            <div style="font-size: 0.875rem; color: var(--text-secondary);">
              <div><strong>Total Size:</strong> {info?.totalSize ? formatSize(info.totalSize) : '0 B'}</div>
              <div><strong>Total Files:</strong> {info?.totalFiles ?? 0}</div>
              <div><strong>User Memories:</strong> {info?.userCount ?? 0} files</div>
              <div><strong>Sessions:</strong> {info?.sessionCount ?? 0} files</div>
              <div><strong>Agents:</strong> {info?.agentCount ?? 0} files</div>
            </div>
          {:else}
            <div style="font-size: 0.875rem; color: var(--text-secondary);">
              <div><strong>Total Size:</strong> 0 B</div>
              <div><strong>Total Files:</strong> 0</div>
              <div><strong>User Memories:</strong> 0</div>
              <div><strong>Sessions:</strong> 0</div>
              <div><strong>Agents:</strong> 0</div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .section {
    margin-bottom: 2rem;
  }

  .section h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 1rem;
    color: var(--text-primary);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }

  .section-header h2 {
    margin-bottom: 0;
  }

  .tenant-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 1rem;
  }

  .tenant-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tenant-card:hover {
    border-color: var(--primary);
    transform: translateY(-2px);
  }

  .tenant-card.selected {
    border-color: var(--primary);
    background: rgba(59, 130, 246, 0.1);
  }

  .tenant-name {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.75rem;
    word-break: break-all;
  }

  .tenant-stats {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .tenant-stat {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .stat-icon {
    font-size: 0.875rem;
  }

  .refresh-btn {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.5rem;
    cursor: pointer;
    color: var(--text-primary);
    font-size: 1rem;
    transition: all 0.2s ease;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .refresh-btn.loading {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Table styles */
  .table-container {
    overflow-x: auto;
    border-radius: 12px;
    border: 1px solid var(--border);
  }

  .tenant-table {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-card);
  }

  .tenant-table th,
  .tenant-table td {
    padding: 0.875rem 1rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .tenant-table th {
    background: var(--bg-hover);
    font-weight: 600;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }

  .tenant-table td {
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .tenant-table tbody tr {
    transition: background 0.15s ease;
  }

  .tenant-table tbody tr:hover {
    background: var(--bg-hover);
  }

  .tenant-table tbody tr.selected {
    background: rgba(59, 130, 246, 0.15);
  }

  .tenant-table tbody tr:last-child td {
    border-bottom: none;
  }

  .tenant-id {
    font-family: 'SF Mono', 'Monaco', monospace;
    font-size: 0.75rem;
    color: var(--text-secondary);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>