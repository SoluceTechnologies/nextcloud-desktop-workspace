import { useRef, useState } from 'react';
import { AlertIcon } from '@/components/icons';
import { api } from '@/lib/api';
import { onEscape } from '@/lib/keys';
import type { Workspace } from '@/lib/types';
import { imageToIcon } from './imageToIcon';
import { TileFace } from './WorkspaceTile';
import './EditWorkspaceForm.css';

export function EditWorkspaceForm({ workspace, onDone }: { workspace: Workspace; onDone: () => void }) {
  const [name, setName] = useState(workspace.name);
  const [icon, setIcon] = useState<string | null | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const shownIcon = icon === undefined ? workspace.icon : icon;
  const customIcon = icon === undefined ? workspace.iconCustom : icon !== null;
  return (
    <form
      className="sheet"
      onKeyDown={onEscape(onDone)}
      onSubmit={async (event) => {
        event.preventDefault();
        if (name !== workspace.name) await api.renameWorkspace(workspace.id, name);
        if (icon !== undefined) await api.setWorkspaceIcon(workspace.id, icon);
        onDone();
      }}
    >
      <h1>Edit workspace</h1>
      <div className="icon-row">
        <span className="tile-preview">
          <TileFace icon={shownIcon} name={name.trim() || workspace.name} />
        </span>
        <div className="icon-actions">
          <button type="button" className="btn ghost" onClick={() => fileInput.current?.click()}>
            Choose image…
          </button>
          {customIcon && (
            <button type="button" className="btn ghost" onClick={() => setIcon(null)}>
              Use server icon
            </button>
          )}
        </div>
        <input
          ref={fileInput}
          type="file"
          accept="image/*"
          hidden
          onChange={async (event) => {
            const picked = event.target.files?.[0];
            event.target.value = '';
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
      <label className="field-label" htmlFor="workspace-name">
        Name
      </label>
      <div className="field">
        <input id="workspace-name" spellCheck={false} value={name} onChange={(event) => setName(event.target.value)} />
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
