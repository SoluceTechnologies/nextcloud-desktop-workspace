import { useState, type ReactNode } from 'react';
import { CloseIcon, PinIcon, PlusIcon } from '@/components/icons';
import { api } from '@/lib/api';
import { move } from '@/lib/move';
import type { Workspace } from '@/lib/types';
import './TabBar.css';

const MIDDLE_BUTTON = 1;

export function TabBar(props: { workspace: Workspace | null; loadingTabs: Set<string>; notice: ReactNode }) {
  const { workspace, loadingTabs, notice } = props;
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  if (!workspace) return <header className="tabbar">{notice}</header>;
  const workspaceId = workspace.id;
  const tabIds = workspace.tabs.map((tab) => tab.id);
  return (
    <header className="tabbar">
      <div className="tabs" role="tablist" aria-label={workspace.name}>
        {workspace.tabs.map((tab, index) => {
          const selected = tab.id === workspace.activeTabId;
          const loading = loadingTabs.has(tab.id);
          return (
            <div
              key={tab.id}
              role="tab"
              tabIndex={0}
              aria-selected={selected}
              aria-busy={loading}
              className={['tab', selected && 'active', tab.pinned && 'pinned'].filter(Boolean).join(' ')}
              title={tab.title}
              draggable
              onDragStart={() => setDraggedIndex(index)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={() => {
                if (draggedIndex !== null && draggedIndex !== index) {
                  api.reorderTabs(workspaceId, move(tabIds, draggedIndex, index));
                }
                setDraggedIndex(null);
              }}
              onClick={() => api.activateTab(workspaceId, tab.id)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') api.activateTab(workspaceId, tab.id);
              }}
              onAuxClick={(event) => {
                if (event.button === MIDDLE_BUTTON && !tab.pinned) api.closeTab(workspaceId, tab.id);
              }}
              onContextMenu={(event) => {
                event.preventDefault();
                api.tabMenu(workspaceId, tab.id);
              }}
            >
              {loading ? (
                <span className="loader" aria-hidden="true" />
              ) : (
                tab.pinned && (
                  <span className="pin">
                    <PinIcon size={12} />
                  </span>
                )
              )}
              <span className="title">{tab.title}</span>
              {!tab.pinned && (
                <button
                  className="close"
                  aria-label={`Close ${tab.title}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    api.closeTab(workspaceId, tab.id);
                  }}
                >
                  <CloseIcon size={12} />
                </button>
              )}
            </div>
          );
        })}
      </div>
      <button className="add" title="Open app" aria-label="Open app" onClick={() => api.appsMenu(workspaceId)}>
        <PlusIcon size={16} />
      </button>
      {notice}
      {workspace.activeTabId && loadingTabs.has(workspace.activeTabId) && (
        <div className="progress" role="progressbar" aria-label="Loading page" />
      )}
    </header>
  );
}
