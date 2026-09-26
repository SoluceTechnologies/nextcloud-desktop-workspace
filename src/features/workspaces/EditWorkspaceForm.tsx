import { useRef, useState } from 'react';
import { AlertIcon } from '@/components/icons';
import { api } from '@/lib/api';
import { onEscape } from '@/lib/keys';
import type { Workspace } from '@/lib/types';
import { imageToIcon } from './imageToIcon';
import { TileFace } from './WorkspaceTile';
import './EditWorkspaceForm.css';

export function EditWorkspaceForm({ ws, onDone }: { ws: Workspace; onDone: () => void }) {
  const [name, setName] = useState(ws.name);
  const [icon, setIcon] = useState<string | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const file = useRef<HTMLInputElement>(null);
  const shown = icon === undefined ? ws.icon : icon;
  const custom = icon === undefined ? ws.iconCustom : icon !== null;
  return (
    <form
      className="sheet"
      onKeyDown={onEscape(onDone)}
      onSubmit={async (e) => {
        e.preventDefault();
        if (name !== ws.name) await api.renameWorkspace(ws.id, name);
        if (icon !== undefined) await api.setWorkspaceIcon(ws.id, icon);
        onDone();
      }}
    >
      <h1>Edit workspace</h1>
      <div className="icon-row">
        <span className="tile-preview">
          <TileFace icon={shown} name={name.trim() || ws.name} />
        </span>
        <div className="icon-actions">
          <button type="button" className="btn ghost" onClick={() => file.current?.click()}>
            Choose image…
          </button>
          {custom && (
            <button type="button" className="btn ghost" onClick={() => setIcon(null)}>
              Use server icon
            </button>
          )}
        </div>
        <input
          ref={file}
          type="file"
          accept="image/*"
          hidden
          onChange={async (e) => {
            const picked = e.target.files?.[0];
            e.target.value = '';
            if (!picked) return;
            try {
              setIcon(await imageToIcon(picked));
              setError(null);
            } catch {
              setError('This file could not be read as an image.');
            }
          }}
        />
      </div>
      <p className="hint">Without an image of your own, the server's logo is used, or the initials of the name.</p>
      {error && (
        <p className="note error" role="alert">
          <AlertIcon size={15} />
          {error}
        </p>
      )}
      <label className="field-label" htmlFor="ws-name">
        Name
      </label>
      <div className="field">
        <input id="ws-name" spellCheck={false} value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <p className="hint">Leave empty to use the name shown by the server.</p>
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
