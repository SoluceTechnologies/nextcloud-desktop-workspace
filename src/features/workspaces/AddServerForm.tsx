import { useState } from 'react';
import { AlertIcon } from '@/components/icons';
import { api } from '@/lib/api';
import { onEscape } from '@/lib/keys';

export function AddServerForm({ onDone }: { onDone?: () => void }) {
  const [url, setUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const insecure = /^http:\/\//i.test(url.trim());
  const welcome = !onDone;
  return (
    <form
      className={welcome ? 'sheet welcome' : 'sheet'}
      onKeyDown={onDone ? onEscape(onDone) : undefined}
      onSubmit={async (event) => {
        event.preventDefault();
        setBusy(true);
        setError(null);
        try {
          await api.addWorkspace(url);
          onDone?.();
        } catch (failure) {
          setError(String(failure));
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
          onChange={(event) => {
            setUrl(event.target.value);
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
