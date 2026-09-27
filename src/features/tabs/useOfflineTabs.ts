import { useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';
import { withMembership } from '@/lib/withMembership';

export function useOfflineTabs(): Set<string> {
  const [offlineTabs, setOfflineTabs] = useState<Set<string>>(() => new Set());
  useTauriEvent<{ tabId: string; offline: boolean }>('tab-offline', ({ tabId, offline }) => {
    setOfflineTabs((current) => withMembership(current, tabId, offline));
  });
  return offlineTabs;
}
