import { api } from '@/lib/api';
import './EmptyWorkspace.css';

export function Unreachable({ tab, server }: { tab: string; server: string }) {
  return (
    <div className="empty" role="alert">
      <h2>Server unreachable</h2>
      <p>
        {server} is not answering. Check your connection; the page reloads by itself as soon as the server is back.
      </p>
      <button className="btn primary" onClick={() => api.retryTab(tab)}>
        Try again
      </button>
    </div>
  );
}
