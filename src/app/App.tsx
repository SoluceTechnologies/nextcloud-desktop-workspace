import { useEffect, useState, type ReactNode } from 'react';
import { Notice } from '@/features/notices/Notice';
import { useNotice } from '@/features/notices/useNotice';
import { EmptyWorkspace } from '@/features/tabs/EmptyWorkspace';
import { TabBar } from '@/features/tabs/TabBar';
import { Unreachable } from '@/features/tabs/Unreachable';
import { useLoadingTabs } from '@/features/tabs/useLoadingTabs';
import { useOfflineTabs } from '@/features/tabs/useOfflineTabs';
import { AddServerForm } from '@/features/workspaces/AddServerForm';
import { Sidebar } from '@/features/workspaces/Sidebar';
import { useUnread } from '@/features/workspaces/useUnread';
import { api } from '@/lib/api';
import type { AppState, Workspace } from '@/lib/types';
import { useAppState } from '@/lib/useAppState';
import { useTauriEvent } from '@/lib/useTauriEvent';
import { DialogHost, type Dialog } from './DialogHost';
import './App.css';

export default function App() {
  const state = useAppState();
  const [dialog, setDialog] = useState<Dialog | null>(null);
  useTauriEvent<Dialog>('ui-request', setDialog);
  const notice = useNotice();
  const loadingTabs = useLoadingTabs();
  const offlineTabs = useOfflineTabs();
  const unread = useUnread();

  useEffect(() => {
    api.setOverlay(dialog !== null);
  }, [dialog]);

  if (!state) return null;
  const activeWorkspace = state.workspaces.find((workspace) => workspace.id === state.activeWorkspaceId) ?? null;
  const closeDialog = () => setDialog(null);
  const adding = dialog?.kind === 'add';
  const bare = adding || state.workspaces.length === 0;

  return (
    <div className={bare ? 'shell bare' : 'shell'}>
      <Sidebar
        state={state}
        unread={unread}
        adding={adding}
        onAdd={() => setDialog({ kind: 'add' })}
        onActivate={closeDialog}
      />
      {!bare && (
        <TabBar
          workspace={activeWorkspace}
          loadingTabs={loadingTabs}
          notice={notice && <Notice notice={notice} />}
        />
      )}
      <main className="content">
        <Content
          state={state}
          dialog={dialog}
          activeWorkspace={activeWorkspace}
          offlineTabs={offlineTabs}
          onCloseDialog={closeDialog}
        />
      </main>
    </div>
  );
}

function Content(props: {
  state: AppState;
  dialog: Dialog | null;
  activeWorkspace: Workspace | null;
  offlineTabs: Set<string>;
  onCloseDialog: () => void;
}): ReactNode {
  const { state, dialog, activeWorkspace, offlineTabs, onCloseDialog } = props;
  if (dialog) return <DialogHost dialog={dialog} state={state} onClose={onCloseDialog} />;
  if (state.workspaces.length === 0) return <AddServerForm />;
  if (!activeWorkspace) return null;
  if (activeWorkspace.tabs.length === 0) return <EmptyWorkspace workspaceId={activeWorkspace.id} />;
  const activeTabId = activeWorkspace.activeTabId;
  if (activeTabId && offlineTabs.has(activeTabId)) {
    return <Unreachable tabId={activeTabId} serverName={activeWorkspace.name} />;
  }
  return null;
}
