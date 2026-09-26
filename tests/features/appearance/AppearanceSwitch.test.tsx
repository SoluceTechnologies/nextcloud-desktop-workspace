import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AppearanceSwitch } from '@/features/appearance/AppearanceSwitch';
import { api } from '@/lib/api';

vi.mock('@/lib/api', () => ({ api: { setTheme: vi.fn() } }));

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('AppearanceSwitch', () => {
  it('shows the chosen appearance and changes it', () => {
    render(<AppearanceSwitch theme="dark" />);
    expect(screen.getByRole('radio', { name: 'Dark' }).getAttribute('aria-checked')).toBe('true');
    expect(screen.getByRole('radio', { name: 'System' }).getAttribute('aria-checked')).toBe('false');
    fireEvent.click(screen.getByRole('radio', { name: 'Light' }));
    expect(api.setTheme).toHaveBeenCalledWith('light');
  });
});
