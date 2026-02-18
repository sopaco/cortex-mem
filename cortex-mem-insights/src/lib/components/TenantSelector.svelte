<script lang="ts">
  import { currentTenant, tenants, tenantLoading, switchTenant, initTenants } from '../stores/tenant';
  import { onMount } from 'svelte';

  let loading = $state(true);
  let selected = $state('');

  onMount(async () => {
    await initTenants();
    loading = false;
  });

  // Subscribe to store
  $effect(() => {
    const unsub = currentTenant.subscribe(v => {
      selected = v;
    });
    return unsub;
  });

  async function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newTenant = target.value;
    if (newTenant) {
      await switchTenant(newTenant);
    }
  }
</script>

{#if loading}
  <div class="tenant-selector">
    <span class="loading-text">Loading tenants...</span>
  </div>
{:else}
  <div class="tenant-selector">
    <label for="tenant-select">Tenant:</label>
    <select 
      id="tenant-select" 
      bind:value={selected}
      onchange={handleChange}
      disabled={$tenantLoading}
    >
      {#each $tenants as tenant}
        <option value={tenant}>{tenant}</option>
      {/each}
    </select>
    {#if $tenantLoading}
      <span class="switching-indicator">Switching...</span>
    {/if}
  </div>
{/if}

<style>
  .tenant-selector {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
    padding: 0.75rem 1rem;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .tenant-selector label {
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .tenant-selector select {
    flex: 1;
    max-width: 300px;
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
    background: var(--bg-dark);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }

  .tenant-selector select:focus {
    outline: none;
    border-color: var(--primary);
  }

  .switching-indicator {
    font-size: 0.75rem;
    color: var(--warning);
    animation: pulse 1s infinite;
  }

  .loading-text {
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
</style>
