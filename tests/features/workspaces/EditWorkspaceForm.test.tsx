import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EditWorkspaceForm } from '@/features/workspaces/EditWorkspaceForm';
import { api } from '@/lib/api';
import type { Workspace } from '@/lib/types';

vi.mock('@/lib/api', () => ({
  api: { renameWorkspace: vi.fn(() => Promise.resolve()), setWorkspaceIcon: vi.fn(() => Promise.resolve()) },
}));

const workspace = (overrides: Partial<Workspace>): Workspace => ({
  id: 'w',
  baseUrl: 'https://a.com/',
  name: 'Soluce Technologies',
  nameCustom: false,
  icon: 'data:image/png;base64,MINE',
  iconCustom: true,
  apps: [],
  tabs: [],
  activeTabId: null,
  ...overrides,
});
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Edit workspace', () => {
  it('goes back to the server icon and saves a changed name', async () => {
    const onClose = vi.fn();
    render(<EditWorkspaceForm workspace={workspace({})} onDone={onClose} />);
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
    render(<EditWorkspaceForm workspace={workspace({ iconCustom: false })} onDone={onClose} />);
    expect(screen.queryByRole('button', { name: 'Use server icon' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(api.setWorkspaceIcon).not.toHaveBeenCalled();
    expect(api.renameWorkspace).not.toHaveBeenCalled();
  });
});
