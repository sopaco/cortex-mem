// Search state store - persists search state across page navigation
import { writable } from 'svelte/store';
import type { SearchResult } from '../types';

export interface SearchState {
  keyword: string;
  scope: string;
  limit: number;
  results: SearchResult[];
  loading: boolean;
  error: string;
  searched: boolean;
}

const defaultState: SearchState = {
  keyword: '',
  scope: 'all',
  limit: 10,
  results: [],
  loading: false,
  error: '',
  searched: false
};

// Create the store
export const searchState = writable<SearchState>(defaultState);

// Helper functions
export function updateKeyword(value: string) {
  searchState.update(s => ({ ...s, keyword: value }));
}

export function updateScope(value: string) {
  searchState.update(s => ({ ...s, scope: value }));
}

export function updateLimit(value: number) {
  searchState.update(s => ({ ...s, limit: value }));
}

export function setResults(results: SearchResult[]) {
  searchState.update(s => ({ ...s, results, searched: true }));
}

export function setLoading(loading: boolean) {
  searchState.update(s => ({ ...s, loading }));
}

export function setError(error: string) {
  searchState.update(s => ({ ...s, error, results: [] }));
}

export function clearResults() {
  searchState.update(s => ({ ...s, results: [], searched: false, error: '' }));
}

// Reset to default state
export function resetSearch() {
  searchState.set(defaultState);
}
