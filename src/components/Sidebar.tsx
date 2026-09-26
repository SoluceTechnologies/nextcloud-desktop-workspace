import { useState } from 'react';
import { api } from '../api';
import type { AppState } from '../types';
import { hue, initials, move } from '../util';

export function Sidebar({ state, onAdd }: { state: AppState; onAdd: () => void }) {
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const ids = state.workspaces.map((w) => w.id);
  return (
    <nav className="sidebar" aria-label="Workspaces">
      <ul>
        {state.workspaces.map((w, i) => {
          const active = w.id === state.activeWorkspaceId;
          return (
            <li key={w.id}>
              <button
                className={active ? 'ws active' : 'ws'}
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
                onClick={() => api.activateWorkspace(w.id)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  api.workspaceMenu(w.id);
                }}
              >
                {w.icon ? (
                  <img src={w.icon} alt="" />
                ) : (
                  <span className="initials" style={{ background: `hsl(${hue(w.id)} 45% 42%)` }}>
                    {initials(w.name)}
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
      <button className="add" title="Add Nextcloud" aria-label="Add Nextcloud" onClick={onAdd}>
        +
      </button>
    </nav>
  );
}
