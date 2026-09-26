import { describe, expect, it } from 'vitest';
import { initials } from '@/features/workspaces/initials';

describe('initials', () => {
  it('uses the meaningful host label for host names', () => {
    expect(initials('cloud.soluce.com')).toBe('S');
    expect(initials('nextcloud.client-a.fr')).toBe('C');
    expect(initials('occos.fr')).toBe('O');
  });
  it('uses up to two word initials for names', () => {
    expect(initials('Soluce Cloud')).toBe('SC');
    expect(initials('client a')).toBe('CA');
    expect(initials('OCCOS')).toBe('O');
  });
  it('falls back to ?', () => {
    expect(initials('  ')).toBe('?');
  });
});
