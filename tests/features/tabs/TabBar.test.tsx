import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { TabBar } from '@/features/tabs/TabBar';
import { api } from '@/lib/api';
import type { Workspace } from '@/lib/types';

vi.mock('@/lib/api', () => ({
  api: { activateTab: vi.fn(), closeTab: vi.fn(), tabMenu: vi.fn(), appsMenu: vi.fn(), reorderTabs: vi.fn() },
}));

const workspace: Workspace = {
  id: 'w',
  baseUrl: 'https://a.com/',
  name: 'A',
  nameCustom: false,
  icon: null, iconCustom: false,
  apps: [],
  activeTabId: 't2',
  tabs: [
    { id: 't1', appId: 'files', title: 'Files', url: 'https://a.com/apps/files/', pinned: true },
    { id: 't2', appId: 'deck', title: 'Deck', url: 'https://a.com/apps/deck/', pinned: false },
  ],
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('TabBar', () => {
  it('renders tabs with selection and close buttons only for unpinned tabs', () => {
    render(<TabBar workspace={workspace} loadingTabs={new Set()} notice={null} />);
    expect(screen.getAllByRole('tab').map((tab) => tab.getAttribute('aria-selected'))).toEqual(['false', 'true']);
    expect(screen.queryByRole('button', { name: 'Close Files' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Close Deck' })).toBeTruthy();
  });

  it('marks loading tabs busy and shows a progress bar while the selected one loads', () => {
    const { rerender } = render(<TabBar workspace={workspace} loadingTabs={new Set(['t1'])} notice={null} />);
    expect(screen.getAllByRole('tab').map((tab) => tab.getAttribute('aria-busy'))).toEqual(['true', 'false']);
    expect(screen.queryByRole('progressbar')).toBeNull();
    rerender(<TabBar workspace={workspace} loadingTabs={new Set(['t2'])} notice={null} />);
    expect(screen.getByRole('progressbar')).toBeTruthy();
  });

  it('closing does not also activate', () => {
    render(<TabBar workspace={workspace} loadingTabs={new Set()} notice={null} />);
    fireEvent.click(screen.getByRole('button', { name: 'Close Deck' }));
    expect(api.closeTab).toHaveBeenCalledWith('w', 't2');
    expect(api.activateTab).not.toHaveBeenCalled();
  });

  it('activates on click, opens tab menu on right click, app picker on +', () => {
    render(<TabBar workspace={workspace} loadingTabs={new Set()} notice={null} />);
    const filesTab = screen.getAllByRole('tab')[0];
    fireEvent.click(filesTab);
    expect(api.activateTab).toHaveBeenCalledWith('w', 't1');
    fireEvent.contextMenu(filesTab);
    expect(api.tabMenu).toHaveBeenCalledWith('w', 't1');
    fireEvent.click(screen.getByRole('button', { name: 'Open app' }));
    expect(api.appsMenu).toHaveBeenCalledWith('w');
  });
});
