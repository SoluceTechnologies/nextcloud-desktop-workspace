import { api } from '@/lib/api';
import type { NoticeData } from './useNotice';
import './Notice.css';

export function Notice({ notice }: { notice: NoticeData }) {
  const { path, tabId } = notice;
  return (
    <div className="notice" role="status">
      {notice.text}
      {path && <button onClick={() => api.revealDownload(path)}>Show</button>}
      {tabId && <button onClick={() => api.openInBrowser(tabId)}>Open in browser</button>}
    </div>
  );
}
