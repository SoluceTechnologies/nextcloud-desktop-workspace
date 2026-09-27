import { useState } from 'react';
import { PlusIcon } from '@/components/icons';
import { AppearanceSwitch } from '@/features/appearance/AppearanceSwitch';
import { api } from '@/lib/api';
import { move } from '@/lib/move';
import type { AppState } from '@/lib/types';
import { TileFace } from './WorkspaceTile';
import './Sidebar.css';

export function Sidebar(props: {
  state: AppState;
  unread?: Record<string, number>;
  adding: boolean;
  onAdd: () => void;
  onActivate: () => void;
}) {
  const { state, unread = {}, adding, onAdd, onActivate } = props;
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const ids = state.workspaces.map((w) => w.id);
  return (
    <nav className="sidebar" aria-label="Workspaces">
      <ul>
        {state.workspaces.map((w, i) => {
          const active = !adding && w.id === state.activeWorkspaceId;
          const count = unread[w.id] ?? 0;
          const label = count > 0 ? `${w.name}, ${count} unread` : w.name;
          return (
            <li key={w.id} className={active ? 'active' : undefined}>
              <button
                className="ws"
                title={label}
                aria-label={label}
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
                <TileFace icon={w.icon} name={w.name} />
              </button>
              {count > 0 && (
                <span className="badge" aria-hidden="true">
                  {count > 99 ? '99+' : count}
                </span>
              )}
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
      <AppearanceSwitch theme={state.theme} />
    </nav>
  );
}
