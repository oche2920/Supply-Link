'use client';

import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import type { SearchResult, SavedQuery } from '@/lib/services/searchService';

interface SearchState {
  searchText: string;
  filters: {
    category?: string;
    status?: 'active' | 'inactive';
  };
  results: SearchResult | null;
  savedQueries: SavedQuery[];
  loading: boolean;
  error: string | null;
}

export function ProductSearchDiscovery() {
  const [state, setState] = useState<SearchState>({
    searchText: '',
    filters: {},
    results: null,
    savedQueries: [],
    loading: false,
    error: null,
  });

  const handleSearch = async () => {
    setState((s) => ({ ...s, loading: true, error: null }));

    try {
      const res = await fetch('/api/v1/products/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          text: state.searchText,
          filters: state.filters,
          offset: 0,
          limit: 50,
        }),
      });

      if (!res.ok) {
        throw new Error('Search failed');
      }

      const data = await res.json();
      setState((s) => ({ ...s, results: data, loading: false }));
    } catch (err) {
      setState((s) => ({
        ...s,
        error: err instanceof Error ? err.message : 'Unknown error',
        loading: false,
      }));
    }
  };

  const handleSaveQuery = async () => {
    try {
      const res = await fetch('/api/v1/products/saved-queries', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: `Search: ${state.searchText || 'All'}`,
          query: {
            text: state.searchText,
            filters: state.filters,
          },
        }),
      });

      if (!res.ok) {
        throw new Error('Failed to save query');
      }

      const saved = await res.json();
      setState((s) => ({
        ...s,
        savedQueries: [...s.savedQueries, saved],
      }));
    } catch (err) {
      setState((s) => ({
        ...s,
        error: err instanceof Error ? err.message : 'Failed to save query',
      }));
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Product Search & Discovery</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">Search</label>
            <input
              type="text"
              placeholder="Search by name, origin, or ID..."
              value={state.searchText}
              onChange={(e) => setState((s) => ({ ...s, searchText: e.target.value }))}
              className="w-full px-3 py-2 border rounded"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-2">Category</label>
              <select
                value={state.filters.category || ''}
                onChange={(e) =>
                  setState((s) => ({
                    ...s,
                    filters: { ...s.filters, category: e.target.value || undefined },
                  }))
                }
                className="w-full px-3 py-2 border rounded"
              >
                <option value="">All Categories</option>
                <option value="agricultural">Agricultural</option>
                <option value="pharmaceutical">Pharmaceutical</option>
                <option value="electronics">Electronics</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">Status</label>
              <select
                value={state.filters.status || ''}
                onChange={(e) =>
                  setState((s) => ({
                    ...s,
                    filters: {
                      ...s.filters,
                      status: (e.target.value as 'active' | 'inactive') || undefined,
                    },
                  }))
                }
                className="w-full px-3 py-2 border rounded"
              >
                <option value="">All Statuses</option>
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
              </select>
            </div>
          </div>

          <div className="flex gap-2">
            <Button onClick={handleSearch} disabled={state.loading}>
              {state.loading ? 'Searching...' : 'Search'}
            </Button>
            <Button variant="outline" onClick={handleSaveQuery} disabled={!state.results}>
              Save Query
            </Button>
          </div>

          {state.error && <div className="text-red-600 text-sm">{state.error}</div>}
        </CardContent>
      </Card>

      {state.results && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>
                Results ({state.results.total} found, showing {state.results.items.length})
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {state.results.items.map((product) => (
                  <div key={product.id} className="border rounded p-3 hover:bg-gray-50">
                    <h4 className="font-semibold">{product.name}</h4>
                    <p className="text-sm text-gray-600">Origin: {product.origin}</p>
                    <p className="text-sm text-gray-600">Owner: {product.owner.slice(0, 12)}...</p>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Faceted Filters</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <h4 className="font-semibold mb-2">Categories</h4>
                <div className="space-y-1">
                  {Object.entries(state.results.facets.categories).map(([cat, count]) => (
                    <button
                      key={cat}
                      onClick={() =>
                        setState((s) => ({
                          ...s,
                          filters: { ...s.filters, category: cat },
                        }))
                      }
                      className="block text-sm text-blue-600 hover:underline"
                    >
                      {cat} ({count})
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <h4 className="font-semibold mb-2">Status</h4>
                <div className="space-y-1">
                  {Object.entries(state.results.facets.statuses).map(([status, count]) => (
                    <button
                      key={status}
                      onClick={() =>
                        setState((s) => ({
                          ...s,
                          filters: {
                            ...s.filters,
                            status: status as 'active' | 'inactive',
                          },
                        }))
                      }
                      className="block text-sm text-blue-600 hover:underline"
                    >
                      {status} ({count})
                    </button>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>

          {state.savedQueries.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle>Saved Queries</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {state.savedQueries.map((query) => (
                    <button
                      key={query.id}
                      className="block w-full text-left px-3 py-2 border rounded hover:bg-gray-50"
                    >
                      {query.name}
                    </button>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}
        </>
      )}
    </div>
  );
}
