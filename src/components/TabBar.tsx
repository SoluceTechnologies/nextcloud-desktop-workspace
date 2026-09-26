import { useState, type ReactNode } from 'react';
import { api } from '../api';
import { CloseIcon, PinIcon, PlusIcon } from '../icons';
import type { Workspace } from '../types';
import { move } from '../util';

export function TabBar({ workspace, notice }: { workspace: Workspace | null; notice: ReactNode }) {
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  if (!workspace) return <header className="tabbar">{notice}</header>;
  const ws = workspace.id;
  const ids = workspace.tabs.map((t) => t.id);
  return (
    <header className="tabbar">
      <div className="tabs" role="tablist" aria-label={workspace.name}>
        {workspace.tabs.map((t, i) => {
          const selected = t.id === workspace.activeTabId;
          return (
            <div
              key={t.id}
              role="tab"
              tabIndex={0}
              aria-selected={selected}
              className={['tab', selected && 'active', t.pinned && 'pinned'].filter(Boolean).join(' ')}
              title={t.title}
              draggable
              onDragStart={() => setDragFrom(i)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={() => {
                if (dragFrom !== null && dragFrom !== i) api.reorderTabs(ws, move(ids, dragFrom, i));
                setDragFrom(null);
              }}
              onClick={() => api.activateTab(ws, t.id)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') api.activateTab(ws, t.id);
              }}
              onAuxClick={(e) => {
                if (e.button === 1 && !t.pinned) api.closeTab(ws, t.id);
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                api.tabMenu(ws, t.id);
              }}
            >
              {t.pinned && (
                <span className="pin">
                  <PinIcon size={12} />
                </span>
              )}
              <span className="title">{t.title}</span>
              {!t.pinned && (
                <button
                  className="close"
                  aria-label={`Close ${t.title}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    api.closeTab(ws, t.id);
                  }}
                >
                  <CloseIcon size={12} />
                </button>
              )}
            </div>
          );
        })}
      </div>
      <button className="add" title="Open app" aria-label="Open app" onClick={() => api.appsMenu(ws)}>
        <PlusIcon size={16} />
      </button>
      {notice}
    </header>
  );
}
