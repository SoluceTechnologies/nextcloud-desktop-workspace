import { useEffect, useState } from 'react';
import { api } from '@/lib/api';
import { useTauriEvent } from '@/lib/useTauriEvent';

export type NoticeData = { text: string; path?: string };

const NOTICE_DURATION_MS = 8000;

export function useNotice(): NoticeData | null {
  const [notice, setNotice] = useState<NoticeData | null>(null);
  useEffect(() => {
    api.takeNotice().then((text) => text && setNotice({ text }));
  }, []);
  useTauriEvent<string>('notice', (text) => setNotice({ text }));
  useTauriEvent<{ path: string | null; success: boolean }>('download-finished', (download) => {
    const fileName = download.path?.split(/[\\/]/).pop() ?? 'file';
    setNotice(
      download.success
        ? { text: `Downloaded ${fileName}`, path: download.path ?? undefined }
        : { text: 'Download failed' },
    );
  });
  useEffect(() => {
    if (!notice) return;
    const timeout = setTimeout(() => setNotice(null), NOTICE_DURATION_MS);
    return () => clearTimeout(timeout);
  }, [notice]);
  return notice;
}
