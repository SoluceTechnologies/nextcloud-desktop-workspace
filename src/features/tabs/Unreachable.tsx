import { api } from '@/lib/api';
import type { OfflineReason } from './useOfflineTabs';
import './EmptyWorkspace.css';

export function Unreachable({ tabId, serverName, reason }: { tabId: string; serverName: string; reason: OfflineReason }) {
  if (reason === 'crashed') {
    return (
      <div className="empty" role="alert">
        <h2>Page stopped</h2>
        <p>This page stopped unexpectedly.</p>
        <button className="btn primary" onClick={() => api.retryTab(tabId)}>
          Reload
        </button>
      </div>
    );
  }
  return (
    <div className="empty" role="alert">
      <h2>Server unreachable</h2>
      <p>
        {serverName} is not answering. Check your connection; the page reloads by itself as soon as the server is
        back.
      </p>
      <button className="btn primary" onClick={() => api.retryTab(tabId)}>
        Try again
      </button>
    </div>
  );
}
