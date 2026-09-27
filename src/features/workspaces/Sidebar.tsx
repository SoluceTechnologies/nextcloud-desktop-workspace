import { useState } from 'react';
import { PlusIcon } from '@/components/icons';
import { AppearanceSwitch } from '@/features/appearance/AppearanceSwitch';
import { api } from '@/lib/api';
import { move } from '@/lib/move';
import type { AppState } from '@/lib/types';
import { TileFace } from './WorkspaceTile';
import './Sidebar.css';

const MAX_BADGE_COUNT = 99;

export function Sidebar(props: {
  state: AppState;
  unread?: Record<string, number>;
  adding: boolean;
  onAdd: () => void;
  onActivate: () => void;
}) {
  const { state, unread = {}, adding, onAdd, onActivate } = props;
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const workspaceIds = state.workspaces.map((workspace) => workspace.id);
  return (
    <nav className="sidebar" aria-label="Workspaces">
      <ul>
        {state.workspaces.map((workspace, index) => {
          const active = !adding && workspace.id === state.activeWorkspaceId;
          const unreadCount = unread[workspace.id] ?? 0;
          const label = unreadCount > 0 ? `${workspace.name}, ${unreadCount} unread` : workspace.name;
          return (
            <li key={workspace.id} className={active ? 'active' : undefined}>
              <button
                className="ws"
                title={label}
                aria-label={label}
                aria-current={active ? 'page' : undefined}
                draggable
                onDragStart={() => setDraggedIndex(index)}
                onDragOver={(event) => event.preventDefault()}
                onDrop={() => {
                  if (draggedIndex !== null && draggedIndex !== index) {
                    api.reorderWorkspaces(move(workspaceIds, draggedIndex, index));
                  }
                  setDraggedIndex(null);
                }}
                onClick={() => {
                  api.activateWorkspace(workspace.id);
                  onActivate();
                }}
                onContextMenu={(event) => {
                  event.preventDefault();
                  api.workspaceMenu(workspace.id);
                }}
              >
                <TileFace icon={workspace.icon} name={workspace.name} />
              </button>
              {unreadCount > 0 && (
                <span className="badge" aria-hidden="true">
                  {unreadCount > MAX_BADGE_COUNT ? `${MAX_BADGE_COUNT}+` : unreadCount}
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
