import { useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';

export function useOfflineTabs(): Set<string> {
  const [offline, setOffline] = useState<Set<string>>(() => new Set());
  useTauriEvent<{ tab: string; offline: boolean }>('tab-offline', ({ tab, offline: on }) => {
    setOffline((cur) => {
      if (cur.has(tab) === on) return cur;
      const next = new Set(cur);
      if (on) next.add(tab);
      else next.delete(tab);
      return next;
    });
  });
  return offline;
}
