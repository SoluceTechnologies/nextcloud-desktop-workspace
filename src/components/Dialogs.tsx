import { useState, type KeyboardEvent } from 'react';
import { api } from '../api';
import { AlertIcon } from '../icons';
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

/** Without `onDone` it is the first-run screen (no workspace yet, nothing to cancel). */
export function AddForm({ onDone }: { onDone?: () => void }) {
  const [url, setUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const insecure = /^http:\/\//i.test(url.trim());
  const welcome = !onDone;
  return (
    <form
      className={welcome ? 'sheet welcome' : 'sheet'}
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
      <h1>{welcome ? 'Connect to Nextcloud' : 'Add a server'}</h1>
      <p className="lede">
        The address of your Nextcloud server. You sign in on the next screen
        {welcome ? '; more servers can be added later from the sidebar.' : '.'}
      </p>
      <label className="field-label" htmlFor="server-url">
        Server address
      </label>
      <div className={error ? 'field invalid' : 'field'}>
        <input
          id="server-url"
          autoFocus
          inputMode="url"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="cloud.example.com"
          aria-invalid={error ? true : undefined}
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            setError(null);
          }}
        />
      </div>
      {insecure && (
        <p className="note warn">
          <AlertIcon size={15} />
          This address is not encrypted (http). Use it only on a network you trust.
        </p>
      )}
      {error && (
        <p className="note error" role="alert">
          <AlertIcon size={15} />
          {error}
        </p>
      )}
      <div className="actions">
        {onDone && (
          <button type="button" className="btn ghost" onClick={onDone}>
            Cancel
          </button>
        )}
        <button type="submit" className="btn primary" disabled={busy || !url.trim()}>
          {busy && <span className="spinner" aria-hidden="true" />}
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
      className="sheet"
      onKeyDown={onEscape(onDone)}
      onSubmit={(e) => {
        e.preventDefault();
        api.renameWorkspace(ws.id, name).then(onDone);
      }}
    >
      <h1>Rename workspace</h1>
      <p className="lede">Leave empty to use the name shown by the server.</p>
      <label className="field-label" htmlFor="ws-name">
        Name
      </label>
      <div className="field">
        <input id="ws-name" autoFocus spellCheck={false} value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <div className="actions">
        <button type="button" className="btn ghost" onClick={onDone}>
          Cancel
        </button>
        <button type="submit" className="btn primary">
          Save
        </button>
      </div>
    </form>
  );
}

function Confirm(props: { title: string; body: string; action: string; onConfirm: () => Promise<void>; onDone: () => void }) {
  return (
    <div className="sheet" role="alertdialog" aria-labelledby="confirm-title" onKeyDown={onEscape(props.onDone)}>
      <h1 id="confirm-title">{props.title}</h1>
      <p className="lede">{props.body}</p>
      <div className="actions">
        <button type="button" className="btn ghost" autoFocus onClick={props.onDone}>
          Cancel
        </button>
        <button type="button" className="btn danger" onClick={() => props.onConfirm().then(props.onDone)}>
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
          body="Its tabs, settings and browsing data are deleted from this computer. You will be signed out."
          action="Remove"
          onConfirm={() => api.removeWorkspace(ws.id)}
          onDone={onClose}
        />
      );
    case 'confirm-clear':
      return (
        <Confirm
          title={`Clear browsing data of ${ws.name}?`}
          body="Cookies, storage and cache of this workspace are deleted. You will be signed out."
          action="Clear data"
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
          action="Close tab"
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
      <h2>No open tabs</h2>
      <p>Open a Nextcloud app to start working in this workspace.</p>
      <button className="btn primary" onClick={() => api.appsMenu(ws)}>
        Open an app
      </button>
    </div>
  );
}
