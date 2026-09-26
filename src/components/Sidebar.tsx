import { useState } from 'react';
import { api } from '../api';
import { PlusIcon } from '../icons';
import type { AppState } from '../types';
import { hue, initials, move } from '../util';

/** `adding`: the Add view is open, so `+` is the current item; clicking a workspace leaves it (`onActivate`). */
export function Sidebar(props: { state: AppState; adding: boolean; onAdd: () => void; onActivate: () => void }) {
  const { state, adding, onAdd, onActivate } = props;
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const ids = state.workspaces.map((w) => w.id);
  return (
    <nav className="sidebar" aria-label="Workspaces">
      <ul>
        {state.workspaces.map((w, i) => {
          const active = !adding && w.id === state.activeWorkspaceId;
          return (
            <li key={w.id} className={active ? 'active' : undefined}>
              <button
                className="ws"
                title={w.name}
                aria-label={w.name}
                aria-current={active ? 'page' : undefined}
                draggable
                onDragStart={() => setDragFrom(i)}
                onDragOver={(e) => e.preventDefault()}
                onDrop={() => {
                  if (dragFrom !== null && dragFrom !== i) api.reorderWorkspaces(move(ids, dragFrom, i));
                  setDragFrom(null);
                }}
                onClick={() => {
                  api.activateWorkspace(w.id);
                  onActivate();
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  api.workspaceMenu(w.id);
                }}
              >
                {w.icon ? (
                  <img src={w.icon} alt="" draggable={false} />
                ) : (
                  <span className="initials" style={{ background: `hsl(${hue(w.id)} 52% 44%)` }}>
                    {initials(w.name)}
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
      {state.workspaces.length > 0 && <div className="sidebar-sep" aria-hidden="true" />}
      <div className={adding ? 'slot active' : 'slot'}>
        <button
          className="add"
          title="Add Nextcloud"
          aria-label="Add Nextcloud"
          aria-current={adding ? 'page' : undefined}
          onClick={onAdd}
        >
          <PlusIcon size={18} />
        </button>
      </div>
    </nav>
  );
}
