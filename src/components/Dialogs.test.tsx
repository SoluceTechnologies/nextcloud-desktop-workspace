import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from '../api';
import type { AppState, Workspace } from '../types';
import { DialogView } from './Dialogs';

vi.mock('../api', () => ({
  api: { renameWorkspace: vi.fn(() => Promise.resolve()), setWorkspaceIcon: vi.fn(() => Promise.resolve()) },
}));

const ws = (over: Partial<Workspace>): Workspace => ({
  id: 'w',
  baseUrl: 'https://a.com/',
  name: 'Soluce Technologies',
  nameCustom: false,
  icon: 'data:image/png;base64,MINE',
  iconCustom: true,
  apps: [],
  tabs: [],
  activeTabId: null,
  ...over,
});
const state = (w: Workspace): AppState => ({ version: 1, workspaces: [w], activeWorkspaceId: 'w', theme: 'system' });

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Edit workspace', () => {
  it('goes back to the server icon and saves a changed name', async () => {
    const onClose = vi.fn();
    render(<DialogView dialog={{ kind: 'edit', ws: 'w' }} state={state(ws({}))} onClose={onClose} />);
    fireEvent.click(screen.getByRole('button', { name: 'Use server icon' }));
    expect(screen.getByText('ST')).toBeTruthy();
    fireEvent.change(screen.getByRole('textbox', { name: 'Name' }), { target: { value: 'Soluce' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(api.setWorkspaceIcon).toHaveBeenCalledWith('w', null);
    expect(api.renameWorkspace).toHaveBeenCalledWith('w', 'Soluce');
  });

  it('offers no reset for a server icon and changes nothing when nothing changed', async () => {
    const onClose = vi.fn();
    render(<DialogView dialog={{ kind: 'edit', ws: 'w' }} state={state(ws({ iconCustom: false }))} onClose={onClose} />);
    expect(screen.queryByRole('button', { name: 'Use server icon' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(api.setWorkspaceIcon).not.toHaveBeenCalled();
    expect(api.renameWorkspace).not.toHaveBeenCalled();
  });
});
