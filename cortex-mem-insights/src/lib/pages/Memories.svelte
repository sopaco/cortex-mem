<script lang="ts">
  import { onMount } from 'svelte';
  import apiClient from '../api';
  import type { FileEntryResponse } from '../types';
  import { currentTenant, initTenants } from '../stores/tenant';
  import TenantSelector from '../components/TenantSelector.svelte';

  let currentPath = $state('cortex://user');
  let entries = $state<FileEntryResponse[]>([]);
  let selectedFile = $state<string | null>(null);
  let fileContent = $state<string | null>(null);
  let editContent = $state('');
  let isEditing = $state(false);
  let loading = $state(true);
  let error = $state('');

  async function loadDirectory(path: string) {
    loading = true;
    error = '';
    currentPath = path;
    selectedFile = null;
    fileContent = null;
    
    try {
      entries = await apiClient.listDirectory(path);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load directory';
    } finally {
      loading = false;
    }
  }

  async function loadFile(path: string) {
    loading = true;
    error = '';
    selectedFile = path;
    
    try {
      fileContent = await apiClient.readFile(path);
      editContent = fileContent;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load file';
    } finally {
      loading = false;
    }
  }

  async function saveFile() {
    if (!selectedFile) return;
    
    error = '';
    try {
      await apiClient.writeFile(selectedFile, editContent);
      fileContent = editContent;
      isEditing = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to save file';
    }
  }

  function goUp() {
    if (currentPath === 'cortex://user' || currentPath === 'cortex://') return;
    
    let parent = currentPath.endsWith('/') ? currentPath.slice(0, -1) : currentPath;
    const idx = parent.lastIndexOf('/');
    if (idx > 9) { // After "cortex://"
      loadDirectory(parent.slice(0, idx + 1));
    } else {
      loadDirectory('cortex://user');
    }
  }

  function handleItemClick(entry: FileEntryResponse) {
    if (entry.is_directory) {
      loadDirectory(entry.uri);
    } else {
      loadFile(entry.uri);
    }
  }

  function toggleEdit() {
    if (fileContent) {
      editContent = fileContent;
    }
    isEditing = !isEditing;
  }

  function cancelEdit() {
    if (fileContent) {
      editContent = fileContent;
    }
    isEditing = false;
  }

  // Reload when tenant changes
  $effect(() => {
    const tenant = $currentTenant;
    if (tenant) {
      loadDirectory('cortex://user');
    }
  });

  function navigateToRoot(root: string) {
    loadDirectory(`cortex://${root}`);
  }

  function renderMarkdown(content: string): string {
    let html = content
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre><code>$2</code></pre>')
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      .replace(/^### (.+)$/gm, '<h3>$1</h3>')
      .replace(/^## (.+)$/gm, '<h2>$1</h2>')
      .replace(/^# (.+)$/gm, '<h1>$1</h1>')
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/\*([^*]+)\*/g, '<em>$1</em>')
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
      .replace(/^\- (.+)$/gm, '<li>$1</li>')
      .replace(/^(\d+)\. (.+)$/gm, '<li>$2</li>')
      .replace(/\n\n/g, '</p><p>')
      .replace(/\n/g, '<br>');
    
    return `<p>${html}</p>`;
  }
</script>

<h1>Memory Management</h1>

<TenantSelector />

<!-- Directory Navigation -->
<div class="directory-nav">
  <button 
    class="nav-tab" 
    class:active={currentPath.startsWith('cortex://user')}
    onclick={() => navigateToRoot('user')}
  >
    👤 User
  </button>
  <button 
    class="nav-tab" 
    class:active={currentPath.startsWith('cortex://session')}
    onclick={() => navigateToRoot('session')}
  >
    💬 Session
  </button>
  <button 
    class="nav-tab" 
    class:active={currentPath.startsWith('cortex://agent')}
    onclick={() => navigateToRoot('agent')}
  >
    🤖 Agent
  </button>
  <button 
    class="nav-tab" 
    class:active={currentPath.startsWith('cortex://resources')}
    onclick={() => navigateToRoot('resources')}
  >
    📚 Resources
  </button>
</div>

<div class="file-browser">
  <!-- File Tree Panel -->
  <div class="card">
    <div class="file-tree-header">
      <button class="btn btn-secondary btn-sm" onclick={goUp} disabled={currentPath === 'cortex://user'}>
        ↑ Up
      </button>
      <span class="path-display">{currentPath}</span>
    </div>

    {#if loading}
      <div class="loading">
        <div class="spinner"></div>
        <span>Loading...</span>
      </div>
    {:else if error && !selectedFile}
      <div class="error-message">{error}</div>
    {:else if entries.length === 0}
      <div class="empty-state">
        <div class="empty-state-icon">📂</div>
        <div class="empty-state-title">Empty</div>
        <div class="empty-state-description">No files in this directory</div>
      </div>
    {:else}
      <div class="file-tree">
        {#each entries as entry}
          <div 
            class="file-tree-item" 
            class:directory={entry.is_directory}
            class:selected={selectedFile === entry.uri}
            onclick={() => handleItemClick(entry)}
            role="button"
            tabindex="0"
            onkeydown={(e) => e.key === 'Enter' && handleItemClick(entry)}
          >
            {entry.is_directory ? '📁 ' : '📄 '}
            {entry.name}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Content Panel -->
  <div class="card">
    {#if selectedFile}
      <div class="edit-actions">
        <span class="file-name">{selectedFile}</span>
        
        {#if isEditing}
          <button class="btn btn-secondary btn-sm" onclick={cancelEdit}>
            Cancel
          </button>
          <button class="btn btn-primary btn-sm" onclick={saveFile}>
            Save
          </button>
        {:else}
          <button class="btn btn-secondary btn-sm" onclick={toggleEdit}>
            Edit
          </button>
        {/if}
      </div>

      {#if isEditing}
        <textarea
          class="form-textarea"
          bind:value={editContent}
        ></textarea>
      {:else if fileContent !== null}
        <div class="content-preview">
          {@html renderMarkdown(fileContent)}
        </div>
      {:else}
        <div class="loading">
          <div class="spinner"></div>
          <span>Loading file...</span>
        </div>
      {/if}
    {:else}
      <div class="empty-state">
        <div class="empty-state-icon">📄</div>
        <div class="empty-state-title">No file selected</div>
        <div class="empty-state-description">Select a file to view or edit</div>
      </div>
    {/if}
  </div>
</div>

{#if error && selectedFile}
  <div class="error-message" style="margin-top: 1rem;">{error}</div>
{/if}

<style>
  .path-display {
    font-size: 0.85rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .file-name {
    font-size: 0.85rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
</style>