import type { Product } from '@/lib/types';

export interface SearchFilter {
  category?: string;
  subcategory?: string;
  owner?: string;
  status?: 'active' | 'inactive';
  minTrustScore?: number;
  dateRange?: {
    from: number;
    to: number;
  };
}

export interface SearchQuery {
  text?: string;
  filters?: SearchFilter;
  offset?: number;
  limit?: number;
}

export interface SavedQuery {
  id: string;
  name: string;
  query: SearchQuery;
  createdAt: number;
  userId: string;
}

export interface SearchResult {
  items: Product[];
  total: number;
  offset: number;
  limit: number;
  facets: {
    categories: Record<string, number>;
    statuses: Record<string, number>;
    owners: Record<string, number>;
  };
}

// In-memory storage for saved queries (would be database in production)
const savedQueries = new Map<string, SavedQuery>();

export function searchProducts(products: Product[], query: SearchQuery): SearchResult {
  let results = [...products];

  // Full-text search
  if (query.text) {
    const searchTerm = query.text.toLowerCase();
    results = results.filter(
      (p) =>
        p.name.toLowerCase().includes(searchTerm) ||
        p.origin.toLowerCase().includes(searchTerm) ||
        p.id.toLowerCase().includes(searchTerm),
    );
  }

  // Apply filters
  if (query.filters) {
    if (query.filters.category) {
      results = results.filter((p) => p.category === query.filters!.category);
    }
    if (query.filters.subcategory) {
      results = results.filter((p) => p.subcategory === query.filters!.subcategory);
    }
    if (query.filters.owner) {
      results = results.filter((p) => p.owner === query.filters!.owner);
    }
    if (query.filters.status) {
      results = results.filter((p) => (p.active ? 'active' : 'inactive') === query.filters!.status);
    }
    if (query.filters.dateRange) {
      results = results.filter(
        (p) =>
          p.timestamp >= query.filters!.dateRange!.from &&
          p.timestamp <= query.filters!.dateRange!.to,
      );
    }
  }

  // Calculate facets
  const facets = {
    categories: {} as Record<string, number>,
    statuses: {} as Record<string, number>,
    owners: {} as Record<string, number>,
  };

  results.forEach((p) => {
    if (p.category) {
      facets.categories[p.category] = (facets.categories[p.category] || 0) + 1;
    }
    const status = p.active ? 'active' : 'inactive';
    facets.statuses[status] = (facets.statuses[status] || 0) + 1;
    facets.owners[p.owner] = (facets.owners[p.owner] || 0) + 1;
  });

  // Pagination
  const offset = query.offset || 0;
  const limit = Math.min(query.limit || 50, 100);
  const items = results.slice(offset, offset + limit);

  return {
    items,
    total: results.length,
    offset,
    limit,
    facets,
  };
}

export function saveQuery(userId: string, name: string, query: SearchQuery): SavedQuery {
  const id = `query-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  const saved: SavedQuery = {
    id,
    name,
    query,
    createdAt: Date.now(),
    userId,
  };
  savedQueries.set(id, saved);
  return saved;
}

export function getSavedQueries(userId: string): SavedQuery[] {
  return Array.from(savedQueries.values()).filter((q) => q.userId === userId);
}

export function getSavedQuery(id: string): SavedQuery | undefined {
  return savedQueries.get(id);
}

export function deleteSavedQuery(id: string): boolean {
  return savedQueries.delete(id);
}

export function getRecentSearches(userId: string, limit: number = 5): SearchQuery[] {
  // In production, this would query a database
  // For now, return empty array
  return [];
}
