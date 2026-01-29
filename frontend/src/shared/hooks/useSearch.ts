import { useState, useEffect } from 'react';
import { useDebounce } from 'use-debounce';
import { search } from '../api/client';
import { SearchResults } from '../types/search';

export function useSearch() {
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedQuery] = useDebounce(searchQuery, 300);
  const [results, setResults] = useState<SearchResults | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    if (debouncedQuery) {
      setIsLoading(true);
      search(debouncedQuery)
        .then((res) => {
          setResults(res);
        })
        .catch((err) => {
          console.error('Search failed:', err);
          setResults(null);
        })
        .finally(() => {
          setIsLoading(false);
        });
    } else {
      setResults(null);
      setIsLoading(false);
    }
  }, [debouncedQuery]);

  return {
    searchQuery,
    setSearchQuery,
    results,
    isLoading,
  };
}
