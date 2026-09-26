import { ConfirmSheet } from '@/components/ConfirmSheet';
import { AddServerForm } from '@/features/workspaces/AddServerForm';
import { EditWorkspaceForm } from '@/features/workspaces/EditWorkspaceForm';
import { api } from '@/lib/api';
import type { AppState } from '@/lib/types';

export type Dialog =
  | { kind: 'add' }
  | { kind: 'edit'; ws: string }
  | { kind: 'confirm-remove'; ws: string }
  | { kind: 'confirm-clear'; ws: string }
  | { kind: 'confirm-close'; ws: string; tab: string };

export function DialogHost({ dialog, state, onClose }: { dialog: Dialog; state: AppState; onClose: () => void }) {
  if (dialog.kind === 'add') return <AddServerForm onDone={onClose} />;
  const ws = state.workspaces.find((w) => w.id === dialog.ws);
  if (!ws) return null;
  switch (dialog.kind) {
    case 'edit':
      return <EditWorkspaceForm ws={ws} onDone={onClose} />;
    case 'confirm-remove':
      return (
        <ConfirmSheet
          title={`Remove ${ws.name}?`}
          body="Its tabs, settings and browsing data are deleted from this computer. You will be signed out."
          action="Remove"
          onConfirm={() => api.removeWorkspace(ws.id)}
          onDone={onClose}
        />
      );
    case 'confirm-clear':
      return (
        <ConfirmSheet
          title={`Clear browsing data of ${ws.name}?`}
          body="Cookies, storage and cache of this workspace are deleted. You will be signed out."
          action="Clear data"
          onConfirm={() => api.clearBrowsingData(ws.id)}
          onDone={onClose}
        />
      );
    case 'confirm-close': {
      const tab = ws.tabs.find((t) => t.id === dialog.tab);
      if (!tab) return null;
      return (
        <ConfirmSheet
          title={`Close pinned tab ${tab.title}?`}
          body="The tab and its page state are closed."
          action="Close tab"
          onConfirm={() => api.closeTab(ws.id, tab.id)}
          onDone={onClose}
        />
      );
    }
  }
}
