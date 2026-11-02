import { useEffect, useState } from 'react';
import { api } from '@/lib/api';
import type { Download } from '@/lib/types';
import { useTauriEvent } from '@/lib/useTauriEvent';

export function useDownloads(): Download[] {
  const [downloads, setDownloads] = useState<Download[]>([]);
  useEffect(() => {
    api.listDownloads().then(setDownloads, () => {});
  }, []);
  useTauriEvent<Download>('download-changed', (changed) => {
    setDownloads((current) => {
      const index = current.findIndex((download) => download.id === changed.id);
      if (index < 0) return [...current, changed];
      const next = [...current];
      next[index] = changed;
      return next;
    });
  });
  return downloads;
}
