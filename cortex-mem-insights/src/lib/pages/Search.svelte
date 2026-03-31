<script lang="ts">
  import apiClient from '../api';
  import type { SearchResult } from '../types';
  import { currentTenant } from '../stores/tenant';
  import { searchState, setLoading, setResults, setError, updateKeyword, updateScope, updateLimit } from '../stores/search';
  import TenantSelector from '../components/TenantSelector.svelte';

  // Use reactive state from store
  let keyword = $derived($searchState.keyword);
  let scope = $derived($searchState.scope);
  let limit = $derived($searchState.limit);
  let results = $derived($searchState.results);
  let loading = $derived($searchState.loading);
  let error = $derived($searchState.error);
  let searched = $derived($searchState.searched);

  async function handleSearch() {
    if (!keyword.trim()) return;
    
    setLoading(true);
    setError('');
    
    try {
      const searchResults = await apiClient.search(keyword, scope, limit);
      setResults(searchResults);
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : 'Search failed';
      // Handle vector search not available
      if (errMsg.includes('Vector search not available') || errMsg.includes('not configured')) {
        setError('Search service is not configured. Please configure Qdrant and Embedding service in the backend.');
      } else {
        setError(errMsg);
      }
    } finally {
      setLoading(false);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      handleSearch();
    }
  }
</script>

<h1>Search Test</h1>

<TenantSelector />

<div class="search-form">
  <div class="form-row">
    <div class="form-group" style="flex: 2;">
      <label for="keyword">Keyword</label>
      <input
        type="text"
        id="keyword"
        class="form-input"
        placeholder="Enter search keyword..."
        value={keyword}
        oninput={(e) => updateKeyword(e.currentTarget.value)}
        onkeydown={handleKeydown}
      />
    </div>
    
    <div class="form-group">
      <label for="scope">Scope</label>
      <select 
        id="scope" 
        class="form-select" 
        value={scope}
        onchange={(e) => updateScope(e.currentTarget.value)}
      >
        <option value="all">All</option>
        <option value="user">User</option>
        <option value="session">Session</option>
        <option value="agent">Agent</option>
      </select>
    </div>
    
    <div class="form-group">
      <label for="limit">Limit</label>
      <input
        type="number"
        id="limit"
        class="form-input"
        min="1"
        max="100"
        value={limit}
        oninput={(e) => updateLimit(parseInt(e.currentTarget.value) || 10)}
      />
    </div>
    
    <div class="form-group" style="display: flex; align-items: flex-end;">
      <button class="btn btn-primary" onclick={handleSearch} disabled={loading}>
        {loading ? 'Searching...' : 'Search'}
      </button>
    </div>
  </div>
</div>

{#if error}
  <div class="error-message">{error}</div>
{/if}

{#if loading}
  <div class="loading">
    <div class="spinner"></div>
    <span>Searching...</span>
  </div>
{:else if searched && results.length === 0}
  <div class="empty-state">
    <div class="empty-state-icon">🔍</div>
    <div class="empty-state-title">No results</div>
    <div class="empty-state-description">Try different keywords or adjust the scope</div>
  </div>
{:else if results.length > 0}
  <div class="results-list">
    <div class="results-header">
      Found {results.length} results
    </div>
    
    {#each results as result}
      <div class="result-card">
        <div class="result-header">
          <span class="result-uri">{result.uri}</span>
          <span class="result-score">{result.score.toFixed(2)}</span>
        </div>
        <div class="result-snippet">{result.snippet}</div>
        {#if result.content}
          <details class="result-details">
            <summary>View full content</summary>
            <pre class="result-content">{result.content}</pre>
          </details>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .search-form {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.25rem;
    margin-bottom: 1.5rem;
  }

  .form-row {
    display: flex;
    gap: 1rem;
    align-items: flex-end;
  }

  .form-group {
    flex: 1;
  }

  .form-group label {
    display: block;
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .form-input,
  .form-select {
    width: 100%;
    padding: 0.625rem 0.875rem;
    font-size: 0.875rem;
    background: var(--bg-dark);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .form-input:focus,
  .form-select:focus {
    outline: none;
    border-color: var(--primary);
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .results-header {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .result-card {
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem;
  }

  .result-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .result-uri {
    font-size: 0.875rem;
    color: var(--primary-light);
    word-break: break-all;
  }

  .result-score {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: linear-gradient(135deg, var(--primary) 0%, var(--primary-dark) 100%);
    color: white;
    border-radius: 4px;
    font-weight: 500;
  }

  .result-snippet {
    font-size: 0.875rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .result-details {
    margin-top: 0.75rem;
  }

  .result-details summary {
    font-size: 0.75rem;
    color: var(--primary-light);
    cursor: pointer;
  }

  /* Use more specific selector to override global styles */
  details.result-details pre.result-content {
    display: block;
    margin-top: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-dark);
    border-radius: 8px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    max-height: 500px;
    overflow: auto;
    white-space: pre-wrap;
    word-wrap: break-word;
    /* Reset global styles */
    -webkit-line-clamp: unset;
    -webkit-box-orient: unset;
  }
</style>
