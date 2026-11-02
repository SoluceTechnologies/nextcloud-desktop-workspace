import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DownloadsButton } from '@/features/downloads/DownloadsButton';
import { DownloadsPanel } from '@/features/downloads/DownloadsPanel';
import { formatBytes } from '@/features/downloads/formatBytes';
import { api } from '@/lib/api';
import type { Download } from '@/lib/types';

vi.mock('@/lib/api', () => ({ api: { cancelDownload: vi.fn(), revealDownload: vi.fn() } }));

const base: Download = {
  id: 'd1',
  fileName: 'report.pdf',
  path: '/home/me/Downloads/report.pdf',
  received: 12_900_000,
  total: 40_000_000,
  state: 'active',
  error: null,
  cancellable: true,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('formatBytes', () => {
  it('uses decimal units with one decimal', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(12_900_000)).toBe('12.9 MB');
  });
});

describe('DownloadsPanel', () => {
  it('shows progress of an active download and cancels it', () => {
    render(<DownloadsPanel downloads={[base]} onClose={() => {}} />);
    expect(screen.getByText('12.9 MB of 40.0 MB · 32 %')).toBeTruthy();
    expect(screen.getByRole('progressbar', { name: 'report.pdf' }).getAttribute('aria-valuenow')).toBe('32');
    fireEvent.click(screen.getByRole('button', { name: 'Cancel report.pdf' }));
    expect(api.cancelDownload).toHaveBeenCalledWith('d1');
  });

  it('shows bytes only when the size is unknown, without a cancel button when not cancellable', () => {
    render(<DownloadsPanel downloads={[{ ...base, total: null, cancellable: false }]} onClose={() => {}} />);
    expect(screen.getByText('12.9 MB')).toBeTruthy();
    expect(screen.getByRole('progressbar', { name: 'report.pdf' }).getAttribute('aria-valuenow')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Cancel report.pdf' })).toBeNull();
  });

  it('reveals a finished download and explains a failed one', () => {
    render(
      <DownloadsPanel
        downloads={[
          { ...base, state: 'completed' },
          { ...base, id: 'd2', fileName: 'big.iso', state: 'failed', error: 'incomplete: received 1 of 2 bytes' },
        ]}
        onClose={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Show report.pdf' }));
    expect(api.revealDownload).toHaveBeenCalledWith('/home/me/Downloads/report.pdf');
    expect(screen.getByText('Failed: incomplete: received 1 of 2 bytes')).toBeTruthy();
  });
});

describe('DownloadsButton', () => {
  it('is hidden without downloads', () => {
    const { container } = render(<DownloadsButton downloads={[]} onOpen={() => {}} />);
    expect(container.innerHTML).toBe('');
  });

  it('announces active downloads and opens the panel', () => {
    const onOpen = vi.fn();
    render(<DownloadsButton downloads={[base, { ...base, id: 'd2', state: 'completed' }]} onOpen={onOpen} />);
    fireEvent.click(screen.getByRole('button', { name: 'Downloads, 1 in progress' }));
    expect(onOpen).toHaveBeenCalled();
  });
});
