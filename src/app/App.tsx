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
import { useAppState } from '@/lib/useAppState';
import { useTauriEvent } from '@/lib/useTauriEvent';
import { DialogHost, type Dialog } from './DialogHost';
import './App.css';

export default function App() {
  const state = useAppState();
  const [dialog, setDialog] = useState<Dialog | null>(null);
  useTauriEvent<Dialog>('ui-request', setDialog);
  const notice = useNotice();
  const loading = useLoadingTabs();
  const offline = useOfflineTabs();
  const unread = useUnread();

  useEffect(() => {
    api.setOverlay(dialog !== null);
  }, [dialog]);

  if (!state) return null;
  const active = state.workspaces.find((w) => w.id === state.activeWorkspaceId) ?? null;
  const close = () => setDialog(null);

  const bare = dialog?.kind === 'add' || state.workspaces.length === 0;
  let content: ReactNode = null;
  if (dialog) content = <DialogHost dialog={dialog} state={state} onClose={close} />;
  else if (state.workspaces.length === 0) content = <AddServerForm />;
  else if (active && active.tabs.length === 0) content = <EmptyWorkspace ws={active.id} />;
  else if (active?.activeTabId && offline.has(active.activeTabId))
    content = <Unreachable tab={active.activeTabId} server={active.name} />;

  return (
    <div className={bare ? 'shell bare' : 'shell'}>
      <Sidebar state={state} unread={unread} adding={dialog?.kind === 'add'} onAdd={() => setDialog({ kind: 'add' })} onActivate={close} />
      {!bare && <TabBar workspace={active} loading={loading} notice={notice && <Notice notice={notice} />} />}
      <main className="content">{content}</main>
    </div>
  );
}
