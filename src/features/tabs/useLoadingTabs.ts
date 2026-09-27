import { useRef, useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';
import { withMembership } from '@/lib/withMembership';

const LOADING_TIMEOUT_MS = 30_000;

export function useLoadingTabs(): Set<string> {
  const [loadingTabs, setLoadingTabs] = useState<Set<string>>(() => new Set());
  const timeouts = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const setLoading = (tabId: string, loading: boolean) =>
    setLoadingTabs((current) => withMembership(current, tabId, loading));
  useTauriEvent<{ tabId: string; loading: boolean }>('tab-loading', ({ tabId, loading }) => {
    clearTimeout(timeouts.current.get(tabId));
    if (loading) timeouts.current.set(tabId, setTimeout(() => setLoading(tabId, false), LOADING_TIMEOUT_MS));
    setLoading(tabId, loading);
  });
  return loadingTabs;
}
