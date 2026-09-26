import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api } from '../api';
import type { AppState } from '../types';
import { Sidebar } from './Sidebar';

vi.mock('../api', () => ({
  api: { activateWorkspace: vi.fn(), workspaceMenu: vi.fn(), reorderWorkspaces: vi.fn(), setTheme: vi.fn() },
}));

const state: AppState = {
  version: 1,
  activeWorkspaceId: 'b',
  theme: 'dark',
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
    render(<Sidebar state={state} adding={false} onAdd={() => {}} onActivate={() => {}} />);
    expect(screen.getByRole('button', { name: 'cloud.soluce.com' }).textContent).toBe('S');
    const occos = screen.getByRole('button', { name: 'OCCOS' });
    expect(occos.querySelector('img')).not.toBeNull();
    expect(occos.getAttribute('aria-current')).toBe('page');
  });

  it('activates on click and opens the native menu on right click', () => {
    render(<Sidebar state={state} adding={false} onAdd={() => {}} onActivate={() => {}} />);
    const btn = screen.getByRole('button', { name: 'cloud.soluce.com' });
    fireEvent.click(btn);
    expect(api.activateWorkspace).toHaveBeenCalledWith('a');
    fireEvent.contextMenu(btn);
    expect(api.workspaceMenu).toHaveBeenCalledWith('a');
  });

  it('while adding, marks + as current instead of the workspace, and a workspace click leaves it', () => {
    const onActivate = vi.fn();
    render(<Sidebar state={state} adding onAdd={() => {}} onActivate={onActivate} />);
    expect(screen.getByRole('button', { name: 'OCCOS' }).getAttribute('aria-current')).toBeNull();
    expect(screen.getByRole('button', { name: 'Add Nextcloud' }).getAttribute('aria-current')).toBe('page');
    fireEvent.click(screen.getByRole('button', { name: 'OCCOS' }));
    expect(api.activateWorkspace).toHaveBeenCalledWith('b');
    expect(onActivate).toHaveBeenCalled();
  });

  it('shows the chosen appearance and changes it', () => {
    render(<Sidebar state={state} adding={false} onAdd={() => {}} onActivate={() => {}} />);
    expect(screen.getByRole('radio', { name: 'Dark' }).getAttribute('aria-checked')).toBe('true');
    expect(screen.getByRole('radio', { name: 'System' }).getAttribute('aria-checked')).toBe('false');
    fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
    expect(api.setTheme).toHaveBeenCalledWith('light');
  });

  it('calls onAdd from the + button', () => {
    const onAdd = vi.fn();
    render(<Sidebar state={state} adding={false} onAdd={onAdd} onActivate={() => {}} />);
    fireEvent.click(screen.getByRole('button', { name: 'Add Nextcloud' }));
    expect(onAdd).toHaveBeenCalled();
  });
});
