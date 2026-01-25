import { useState, useCallback, useRef } from 'react';

interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

interface UseOptimisticDataOptions {
  cacheDuration?: number;
}

interface UseOptimisticDataReturn<T> {
  data: T;
  isLoading: boolean;
  hasError: boolean;
  fetchData: (fetchFn: () => Promise<T>, forceRefresh?: boolean) => Promise<void>;
  clearCache: () => void;
}

export function useOptimisticData<T>(
  initialData: T,
  options: UseOptimisticDataOptions = {}
): UseOptimisticDataReturn<T> {
  const { cacheDuration = 30000 } = options;
  
  const [data, setData] = useState<T>(initialData);
  const [isLoading, setIsLoading] = useState(true);
  const [hasError, setHasError] = useState(false);
  const cacheRef = useRef<CacheEntry<T> | null>(null);

  const fetchData = useCallback(
    async (fetchFn: () => Promise<T>, forceRefresh: boolean = false) => {
      const now = Date.now();
      
      if (
        !forceRefresh &&
        cacheRef.current &&
        now - cacheRef.current.timestamp < cacheDuration
      ) {
        setData(cacheRef.current.data);
        setIsLoading(false);
        setHasError(false);
        return;
      }

      const hasExistingData = 
        data !== initialData || 
        (Array.isArray(data) && data.length > 0) ||
        (typeof data === 'object' && data !== null && Object.keys(data).length > 0);

      if (!hasExistingData) {
        setIsLoading(true);
      }
      
      setHasError(false);

      try {
        const result = await fetchFn();
        
        cacheRef.current = {
          data: result,
          timestamp: Date.now(),
        };
        
        setData(result);
        setIsLoading(false);
        setHasError(false);
      } catch (err) {
        console.error('Failed to fetch data:', err);
        
        const isNetworkError =
          err instanceof TypeError ||
          (err instanceof Error &&
            (err.message.includes('fetch') ||
              err.message.includes('network') ||
              err.message.includes('Unable to connect') ||
              err.message.includes('Failed to fetch')));

        if (isNetworkError) {
          setHasError(true);
        } else {
          setIsLoading(false);
          setHasError(true);
        }
      }
    },
    [data, initialData, cacheDuration]
  );

  const clearCache = useCallback(() => {
    cacheRef.current = null;
  }, []);

  return {
    data,
    isLoading,
    hasError,
    fetchData,
    clearCache,
  };
}