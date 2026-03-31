// Global tenant store - shared across all pages
import { writable, derived } from 'svelte/store';
import apiClient from '../api';

export interface TenantInfo {
  id: string;
  userCount: number;
  sessionCount: number;
  agentCount: number;
  totalFiles: number;
  totalSize: number;
}

// Current selected tenant
export const currentTenant = writable<string>('');

// All available tenants
export const tenants = writable<string[]>([]);

// Tenant info cache
export const tenantInfo = writable<Map<string, TenantInfo>>(new Map());

// Loading state
export const tenantLoading = writable<boolean>(false);

// Track if initial load has been done
let initialized = false;

// Check if already initialized
export function isInitialized(): boolean {
  return initialized;
}

// Mark as initialized
export function markInitialized(): void {
  initialized = true;
}

// Reset initialization state (for refresh)
export function resetInitialization(): void {
  initialized = false;
}

// Initialize tenant store
export async function initTenants(): Promise<void> {
  // Skip if already initialized
  if (initialized) {
    return;
  }
  
  tenantLoading.set(true);
  try {
    const tenantList = await apiClient.listTenants();
    tenants.set(tenantList);
    
    // Auto-select first tenant if available
    if (tenantList.length > 0) {
      const current = await apiClient.getCurrentTenant();
      if (current) {
        currentTenant.set(current);
      } else {
        currentTenant.set(tenantList[0]);
        await apiClient.switchTenant(tenantList[0]);
      }
    }
    
    initialized = true;
  } catch (e) {
    console.error('Failed to load tenants:', e);
  } finally {
    tenantLoading.set(false);
  }
}

// Switch tenant and update global state
export async function switchTenant(tenantId: string): Promise<void> {
  // Skip if already on this tenant
  let currentTenantValue = '';
  currentTenant.subscribe(v => currentTenantValue = v)();
  if (currentTenantValue === tenantId) {
    return;
  }
  
  tenantLoading.set(true);
  try {
    await apiClient.switchTenant(tenantId);
    currentTenant.set(tenantId);
  } catch (e) {
    console.error('Failed to switch tenant:', e);
    throw e;
  } finally {
    tenantLoading.set(false);
  }
}

// Load info for a specific tenant (optimized: don't switch tenant)
export async function loadTenantInfo(tenantId: string, force: boolean = false): Promise<TenantInfo> {
  // Check cache first (unless force refresh)
  if (!force) {
    let cachedInfo: TenantInfo | undefined;
    tenantInfo.subscribe(map => cachedInfo = map.get(tenantId))();
    if (cachedInfo) {
      return cachedInfo;
    }
  }
  
  // Get current tenant
  let currentTenantValue = '';
  currentTenant.subscribe(v => currentTenantValue = v)();
  
  // Only switch if we're not already on this tenant
  const needSwitch = currentTenantValue !== tenantId;
  
  if (needSwitch) {
    await apiClient.switchTenant(tenantId);
  }
  
  try {
    const sessions = await apiClient.getSessions();
    
    // Get directory info with recursive counting
    let userCount = 0;
    let userSize = 0;
    let agentCount = 0;
    let agentSize = 0;
    let sessionCount = 0;
    let sessionSize = 0;
    let resourceCount = 0;
    let resourceSize = 0;
    let totalFiles = 0;
    let totalSize = 0;
    
    // Helper to add stats
    const addStats = (stats: { file_count: number; total_size: number }) => {
      totalFiles += stats.file_count;
      totalSize += stats.total_size;
    };
    
    try {
      const userStats = await apiClient.getDirectoryStats('cortex://user');
      userCount = userStats.file_count;
      userSize = userStats.total_size;
      addStats(userStats);
    } catch (e) {
      console.error('Failed to get user stats:', e);
    }
    
    try {
      const agentStats = await apiClient.getDirectoryStats('cortex://agent');
      agentCount = agentStats.file_count;
      agentSize = agentStats.total_size;
      addStats(agentStats);
    } catch (e) {
      console.error('Failed to get agent stats:', e);
    }
    
    try {
      const sessionStats = await apiClient.getDirectoryStats('cortex://session');
      sessionCount = sessionStats.file_count;
      sessionSize = sessionStats.total_size;
      addStats(sessionStats);
    } catch (e) {
      console.error('Failed to get session stats:', e);
    }
    
    try {
      const resourceStats = await apiClient.getDirectoryStats('cortex://resources');
      resourceCount = resourceStats.file_count;
      resourceSize = resourceStats.total_size;
      addStats(resourceStats);
    } catch (e) {
      console.error('Failed to get resource stats:', e);
    }
    
    const info: TenantInfo = {
      id: tenantId,
      userCount,
      sessionCount: sessions.length > 0 ? sessions.length : sessionCount,
      agentCount,
      totalFiles,
      totalSize
    };
    
    // Update cache
    tenantInfo.update(map => {
      map.set(tenantId, info);
      return map;
    });
    
    return info;
  } finally {
    // Only switch back if we switched in the first place
    if (needSwitch && currentTenantValue) {
      await apiClient.switchTenant(currentTenantValue);
    }
  }
}

// Helper to get current value synchronously
function getCurrentTenantValue(): string {
  let value = '';
  currentTenant.subscribe(v => value = v)();
  return value;
}
