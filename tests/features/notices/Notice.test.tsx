import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Notice } from '@/features/notices/Notice';
import { api } from '@/lib/api';

vi.mock('@/lib/api', () => ({ api: { openInBrowser: vi.fn(), revealDownload: vi.fn() } }));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('Notice', () => {
  it('offers to continue a call in the browser', () => {
    render(<Notice notice={{ text: 'Calls need WebRTC', tabId: 't1' }} />);
    fireEvent.click(screen.getByRole('button', { name: 'Open in browser' }));
    expect(api.openInBrowser).toHaveBeenCalledWith('t1');
  });

  it('reveals a finished download', () => {
    render(<Notice notice={{ text: 'Downloaded a.pdf', path: '/d/a.pdf' }} />);
    fireEvent.click(screen.getByRole('button', { name: 'Show' }));
    expect(api.revealDownload).toHaveBeenCalledWith('/d/a.pdf');
  });
});
