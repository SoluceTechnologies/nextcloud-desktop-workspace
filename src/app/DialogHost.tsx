import { ConfirmSheet } from '@/components/ConfirmSheet';
import { AddServerForm } from '@/features/workspaces/AddServerForm';
import { EditWorkspaceForm } from '@/features/workspaces/EditWorkspaceForm';
import { api } from '@/lib/api';
import type { AppState } from '@/lib/types';

export type Dialog =
  | { kind: 'add' }
  | { kind: 'edit'; workspaceId: string }
  | { kind: 'confirm-remove'; workspaceId: string }
  | { kind: 'confirm-clear'; workspaceId: string }
  | { kind: 'confirm-close'; workspaceId: string; tabId: string };

export function DialogHost({ dialog, state, onClose }: { dialog: Dialog; state: AppState; onClose: () => void }) {
  if (dialog.kind === 'add') return <AddServerForm onDone={onClose} />;
  const workspace = state.workspaces.find((candidate) => candidate.id === dialog.workspaceId);
  if (!workspace) return null;
  switch (dialog.kind) {
    case 'edit':
      return <EditWorkspaceForm workspace={workspace} onDone={onClose} />;
    case 'confirm-remove':
      return (
        <ConfirmSheet
          title={`Remove ${workspace.name}?`}
          body="Its tabs, settings and browsing data are deleted from this computer. You will be signed out."
          action="Remove"
          onConfirm={() => api.removeWorkspace(workspace.id)}
          onDone={onClose}
        />
      );
    case 'confirm-clear':
      return (
        <ConfirmSheet
          title={`Clear browsing data of ${workspace.name}?`}
          body="Cookies, storage and cache of this workspace are deleted. You will be signed out."
          action="Clear data"
          onConfirm={() => api.clearBrowsingData(workspace.id)}
          onDone={onClose}
        />
      );
    case 'confirm-close': {
      const tab = workspace.tabs.find((candidate) => candidate.id === dialog.tabId);
      if (!tab) return null;
      return (
        <ConfirmSheet
          title={`Close pinned tab ${tab.title}?`}
          body="The tab and its page state are closed."
          action="Close tab"
          onConfirm={() => api.closeTab(workspace.id, tab.id)}
          onDone={onClose}
        />
      );
    }
  }
}
