import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from '../api';
import type { AppState } from '../types';
import { Sidebar } from './Sidebar';

vi.mock('../api', () => ({
  api: { activateWorkspace: vi.fn(), workspaceMenu: vi.fn(), reorderWorkspaces: vi.fn() },
}));

const state: AppState = {
  version: 1,
  activeWorkspaceId: 'b',
  workspaces: [
    { id: 'a', baseUrl: 'https://cloud.soluce.com/', name: 'cloud.soluce.com', nameCustom: false, icon: null, apps: [], tabs: [], activeTabId: null },
    { id: 'b', baseUrl: 'https://occos.fr/', name: 'OCCOS', nameCustom: true, icon: 'data:image/png;base64,AAAA', apps: [], tabs: [], activeTabId: null },
  ],
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Sidebar', () => {
  it('shows initials without icon, the icon otherwise, and marks the active workspace', () => {
    render(<Sidebar state={state} onAdd={() => {}} />);
    expect(screen.getByRole('button', { name: 'cloud.soluce.com' }).textContent).toBe('S');
    const occos = screen.getByRole('button', { name: 'OCCOS' });
    expect(occos.querySelector('img')).not.toBeNull();
    expect(occos.getAttribute('aria-current')).toBe('page');
  });

  it('activates on click and opens the native menu on right click', () => {
    render(<Sidebar state={state} onAdd={() => {}} />);
    const btn = screen.getByRole('button', { name: 'cloud.soluce.com' });
    fireEvent.click(btn);
    expect(api.activateWorkspace).toHaveBeenCalledWith('a');
    fireEvent.contextMenu(btn);
    expect(api.workspaceMenu).toHaveBeenCalledWith('a');
  });

  it('calls onAdd from the + button', () => {
    const onAdd = vi.fn();
    render(<Sidebar state={state} onAdd={onAdd} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add Nextcloud' }));
    expect(onAdd).toHaveBeenCalled();
  });
});
