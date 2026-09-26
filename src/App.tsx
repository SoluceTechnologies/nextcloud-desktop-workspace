import { useEffect, useState, type ReactNode } from 'react';
import './App.css';
import { api } from './api';
import { AddForm, DialogView, EmptyWorkspace, type Dialog } from './components/Dialogs';
import { Sidebar } from './components/Sidebar';
import { TabBar } from './components/TabBar';
import { useAppState, useNotice, useTauriEvent } from './hooks';

export default function App() {
  const state = useAppState();
  const [dialog, setDialog] = useState<Dialog | null>(null);
  useTauriEvent<Dialog>('ui-request', setDialog);
  const notice = useNotice();
  // Content webviews sit above the shell; hide them while a dialog is open.
  useEffect(() => {
    api.setOverlay(dialog !== null);
  }, [dialog]);

  if (!state) return null;
  const active = state.workspaces.find((w) => w.id === state.activeWorkspaceId) ?? null;
  const close = () => setDialog(null);

  let content: ReactNode = null;
  if (dialog) content = <DialogView dialog={dialog} state={state} onClose={close} />;
  else if (state.workspaces.length === 0) content = <AddForm />;
  else if (active && active.tabs.length === 0) content = <EmptyWorkspace ws={active.id} />;

  return (
    <div className="shell">
      <Sidebar state={state} adding={dialog?.kind === 'add'} onAdd={() => setDialog({ kind: 'add' })} onActivate={close} />
      {/* Adding a server is not about the current workspace: hide its tabs meanwhile. */}
      <TabBar workspace={dialog?.kind === 'add' ? null : active} notice={notice} />
      <main className="content">{content}</main>
    </div>
  );
}
