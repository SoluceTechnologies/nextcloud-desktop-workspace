import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type { AppState } from './types';

vi.mock('./api', () => ({ api: new Proxy({}, { get: () => vi.fn(() => Promise.resolve()) }) }));

const state: AppState = {
  version: 1,
  activeWorkspaceId: 'w',
  workspaces: [
    {
      id: 'w',
      baseUrl: 'https://a.com/',
      name: 'A',
      nameCustom: false,
      icon: null,
      apps: [],
      activeTabId: 't',
      tabs: [{ id: 't', appId: 'files', title: 'Files', url: 'https://a.com/apps/files/', pinned: false }],
    },
  ],
};

vi.mock('./hooks', () => ({
  useAppState: () => state,
  useTauriEvent: () => {},
  useNotice: () => null,
}));

afterEach(cleanup);

describe('App', () => {
  it('hides the active workspace tabs while the Add dialog is open', () => {
    render(<App />);
    expect(screen.getAllByRole('tab')).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: 'Add Nextcloud' }));
    expect(screen.queryAllByRole('tab')).toHaveLength(0);
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.getAllByRole('tab')).toHaveLength(1);
  });

  it('leaves the Add view when a workspace is clicked', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: 'Add Nextcloud' }));
    fireEvent.click(screen.getByRole('button', { name: 'A' }));
    expect(screen.queryByText('Server URL')).toBeNull();
    expect(screen.getAllByRole('tab')).toHaveLength(1);
  });
});
