import { CloseIcon } from '@/components/icons';
import { api } from '@/lib/api';
import { onEscape } from '@/lib/keys';
import type { Download } from '@/lib/types';
import { formatBytes } from './formatBytes';
import './Downloads.css';

export function percent(download: Download): number | null {
  if (!download.total) return null;
  return Math.min(100, Math.floor((download.received / download.total) * 100));
}

function status(download: Download): string {
  switch (download.state) {
    case 'completed':
      return `Done · ${formatBytes(download.received || download.total || 0)}`;
    case 'cancelled':
      return 'Cancelled';
    case 'failed':
      return `Failed: ${download.error ?? 'unknown error'}`;
    case 'active': {
      const done = percent(download);
      if (done === null) return download.received ? formatBytes(download.received) : 'Downloading…';
      return `${formatBytes(download.received)} of ${formatBytes(download.total ?? 0)} · ${done} %`;
    }
  }
}

export function DownloadsPanel({ downloads, onClose }: { downloads: Download[]; onClose: () => void }) {
  return (
    <div className="sheet downloads" role="dialog" aria-labelledby="downloads-title" onKeyDown={onEscape(onClose)}>
      <header>
        <h1 id="downloads-title">Downloads</h1>
        <button type="button" className="icon" aria-label="Close downloads" autoFocus onClick={onClose}>
          <CloseIcon size={14} />
        </button>
      </header>
      {downloads.length === 0 && <p className="lede">No downloads yet.</p>}
      <ul>
        {[...downloads].reverse().map((download) => {
          const done = percent(download);
          return (
            <li key={download.id} className={download.state}>
              <div className="name" title={download.path ?? download.fileName}>
                {download.fileName}
              </div>
              {download.state === 'active' && (
                <div
                  className={done === null ? 'bar indeterminate' : 'bar'}
                  role="progressbar"
                  aria-label={download.fileName}
                  aria-valuemin={done === null ? undefined : 0}
                  aria-valuemax={done === null ? undefined : 100}
                  aria-valuenow={done ?? undefined}
                >
                  <span style={done === null ? undefined : { width: `${done}%` }} />
                </div>
              )}
              <div className="status">{status(download)}</div>
              <div className="row-actions">
                {download.state === 'active' && download.cancellable && (
                  <button type="button" className="btn ghost" aria-label={`Cancel ${download.fileName}`} onClick={() => api.cancelDownload(download.id)}>
                    Cancel
                  </button>
                )}
                {download.state === 'completed' && download.path && (
                  <button type="button" className="btn ghost" aria-label={`Show ${download.fileName}`} onClick={() => api.revealDownload(download.path!)}>
                    Show
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
