import { useRef, useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';

export function useLoadingTabs(): Set<string> {
  const [loading, setLoading] = useState<Set<string>>(() => new Set());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  useTauriEvent<{ tab: string; loading: boolean }>('tab-loading', ({ tab, loading: on }) => {
    clearTimeout(timers.current.get(tab));
    if (on) timers.current.set(tab, setTimeout(() => update(tab, false), 30_000));
    update(tab, on);
  });
  function update(tab: string, on: boolean) {
    setLoading((cur) => {
      if (cur.has(tab) === on) return cur;
      const next = new Set(cur);
      if (on) next.add(tab);
      else next.delete(tab);
      return next;
    });
  }
  return loading;
}
