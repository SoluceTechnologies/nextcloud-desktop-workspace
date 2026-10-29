import { useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';

export type OfflineReason = 'unreachable' | 'crashed';

export function useOfflineTabs(): Map<string, OfflineReason> {
  const [offlineTabs, setOfflineTabs] = useState<Map<string, OfflineReason>>(() => new Map());
  useTauriEvent<{ tabId: string; offline: boolean; reason: OfflineReason }>(
    'tab-offline',
    ({ tabId, offline, reason }) => {
      setOfflineTabs((current) => {
        const next = new Map(current);
        if (offline) next.set(tabId, reason);
        else next.delete(tabId);
        return next;
      });
    },
  );
  return offlineTabs;
}
