import { useEffect, useState } from 'react';
import { api } from '@/lib/api';
import { useTauriEvent } from '@/lib/useTauriEvent';

export type NoticeData = { text: string; path?: string };

export function useNotice(): NoticeData | null {
  const [notice, setNotice] = useState<NoticeData | null>(null);
  useEffect(() => {
    api.takeNotice().then((text) => text && setNotice({ text }));
  }, []);
  useTauriEvent<{ path: string | null; success: boolean }>('download-finished', (d) => {
    const name = d.path?.split(/[\\/]/).pop() ?? 'file';
    setNotice(d.success ? { text: `Downloaded ${name}`, path: d.path ?? undefined } : { text: 'Download failed' });
  });
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(null), 8000);
    return () => clearTimeout(t);
  }, [notice]);
  return notice;
}
