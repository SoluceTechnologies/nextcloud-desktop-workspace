import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Unreachable } from '@/features/tabs/Unreachable';
import { api } from '@/lib/api';

vi.mock('@/lib/api', () => ({ api: { retryTab: vi.fn() } }));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Unreachable', () => {
  it('shows the crash copy and reloads', () => {
    render(<Unreachable tabId="t1" serverName="Cloud" reason="crashed" />);
    expect(screen.getByRole('heading', { name: 'Page stopped' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Reload' }));
    expect(api.retryTab).toHaveBeenCalledWith('t1');
  });

  it('keeps the unreachable copy', () => {
    render(<Unreachable tabId="t1" serverName="Cloud" reason="unreachable" />);
    expect(screen.getByRole('heading', { name: 'Server unreachable' })).toBeTruthy();
  });
});
