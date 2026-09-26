import { useState, type ReactNode } from 'react';
import { api } from '../api';
import { MonitorIcon, MoonIcon, PlusIcon, SunIcon } from '../icons';
import type { AppState, Appearance } from '../types';
import { initials, move } from '../util';

const APPEARANCES: { value: Appearance; label: string; icon: ReactNode }[] = [
  { value: 'system', label: 'System', icon: <MonitorIcon size={15} /> },
  { value: 'light', label: 'Light', icon: <SunIcon size={15} /> },
  { value: 'dark', label: 'Dark', icon: <MoonIcon size={15} /> },
];

/** What a workspace tile shows: its icon, else the initials of its name. */
export function TileFace({ icon, name }: { icon: string | null; name: string }) {
  return icon ? <img src={icon} alt="" draggable={false} /> : <span className="initials">{initials(name)}</span>;
}

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
                <TileFace icon={w.icon} name={w.name} />
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
      <div className="appearance" role="radiogroup" aria-label="Appearance">
        {APPEARANCES.map((a) => (
          <button
            key={a.value}
            role="radio"
            aria-checked={state.theme === a.value}
            aria-label={a.label}
            title={a.label}
            onClick={() => api.setTheme(a.value)}
          >
            {a.icon}
          </button>
        ))}
      </div>
    </nav>
  );
}
