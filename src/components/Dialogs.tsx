import { useState, type KeyboardEvent } from 'react';
import { api } from '../api';
import type { AppState, Workspace } from '../types';

/** Dialog requests: `add` from the sidebar, the others from native menus (`ui-request` event). */
export type Dialog =
  | { kind: 'add' }
  | { kind: 'rename'; ws: string }
  | { kind: 'confirm-remove'; ws: string }
  | { kind: 'confirm-clear'; ws: string }
  | { kind: 'confirm-close'; ws: string; tab: string };

const onEscape = (close: () => void) => (e: KeyboardEvent) => {
  if (e.key === 'Escape') close();
};

export function AddForm({ onDone }: { onDone?: () => void }) {
  const [url, setUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const insecure = /^http:\/\//i.test(url.trim());
  return (
    <form
      className="dialog"
      onKeyDown={onDone ? onEscape(onDone) : undefined}
      onSubmit={async (e) => {
        e.preventDefault();
        setBusy(true);
        setError(null);
        try {
          await api.addWorkspace(url);
          onDone?.();
        } catch (err) {
          setError(String(err));
        } finally {
          setBusy(false);
        }
      }}
    >
      <h1>Add Nextcloud</h1>
      <label htmlFor="server-url">Server URL</label>
      <input
        id="server-url"
        autoFocus
        inputMode="url"
        placeholder="https://cloud.example.com"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
      />
      {insecure && <p className="warn">This address is not encrypted (http).</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className="actions">
        {onDone && (
          <button type="button" onClick={onDone}>
            Cancel
          </button>
        )}
        <button type="submit" className="primary" disabled={busy || !url.trim()}>
          Connect
        </button>
      </div>
    </form>
  );
}

function RenameForm({ ws, onDone }: { ws: Workspace; onDone: () => void }) {
  const [name, setName] = useState(ws.name);
  return (
    <form
      className="dialog"
      onKeyDown={onEscape(onDone)}
      onSubmit={(e) => {
        e.preventDefault();
        api.renameWorkspace(ws.id, name).then(onDone);
      }}
    >
      <h1>Edit name</h1>
      <label htmlFor="ws-name">Name</label>
      <input id="ws-name" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      <p className="hint">Leave empty to use the name shown by the server.</p>
      <div className="actions">
        <button type="button" onClick={onDone}>
          Cancel
        </button>
        <button type="submit" className="primary">
          Save
        </button>
      </div>
    </form>
  );
}

function Confirm(props: { title: string; body: string; action: string; onConfirm: () => Promise<void>; onDone: () => void }) {
  return (
    <div className="dialog" role="alertdialog" aria-labelledby="confirm-title" onKeyDown={onEscape(props.onDone)}>
      <h1 id="confirm-title">{props.title}</h1>
      <p>{props.body}</p>
      <div className="actions">
        <button type="button" autoFocus onClick={props.onDone}>
          Cancel
        </button>
        <button type="button" className="danger" onClick={() => props.onConfirm().then(props.onDone)}>
          {props.action}
        </button>
      </div>
    </div>
  );
}

export function DialogView({ dialog, state, onClose }: { dialog: Dialog; state: AppState; onClose: () => void }) {
  if (dialog.kind === 'add') return <AddForm onDone={onClose} />;
  const ws = state.workspaces.find((w) => w.id === dialog.ws);
  if (!ws) return null;
  switch (dialog.kind) {
    case 'rename':
      return <RenameForm ws={ws} onDone={onClose} />;
    case 'confirm-remove':
      return (
        <Confirm
          title={`Remove ${ws.name}?`}
          body="Its tabs, settings and browsing data are deleted from this computer. You will be logged out."
          action="Remove"
          onConfirm={() => api.removeWorkspace(ws.id)}
          onDone={onClose}
        />
      );
    case 'confirm-clear':
      return (
        <Confirm
          title={`Clear browsing data of ${ws.name}?`}
          body="Cookies, storage and cache of this workspace are deleted. You will be logged out."
          action="Clear"
          onConfirm={() => api.clearBrowsingData(ws.id)}
          onDone={onClose}
        />
      );
    case 'confirm-close': {
      const tab = ws.tabs.find((t) => t.id === dialog.tab);
      if (!tab) return null;
      return (
        <Confirm
          title={`Close pinned tab ${tab.title}?`}
          body="The tab and its page state are closed."
          action="Close"
          onConfirm={() => api.closeTab(ws.id, tab.id)}
          onDone={onClose}
        />
      );
    }
  }
}

export function EmptyWorkspace({ ws }: { ws: string }) {
  return (
    <div className="empty">
      <p>No app open in this workspace.</p>
      <button className="primary" onClick={() => api.appsMenu(ws)}>
        Open app
      </button>
    </div>
  );
}
