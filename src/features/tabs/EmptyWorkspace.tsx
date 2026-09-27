import { api } from '@/lib/api';
import { below } from '@/lib/menuAnchor';
import './EmptyWorkspace.css';

export function EmptyWorkspace({ workspaceId }: { workspaceId: string }) {
  return (
    <div className="empty">
      <h2>No open tabs</h2>
      <p>Open a Nextcloud app to start working in this workspace.</p>
      <button className="btn primary" onClick={(event) => api.appsMenu(workspaceId, below(event.currentTarget))}>
        Open an app
      </button>
    </div>
  );
}
