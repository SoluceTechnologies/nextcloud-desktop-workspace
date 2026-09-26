import { describe, expect, it } from 'vitest';
import { initials, move } from './util';

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

describe('move', () => {
  it('moves an item without mutating the input', () => {
    const src = ['a', 'b', 'c'];
    expect(move(src, 0, 2)).toEqual(['b', 'c', 'a']);
    expect(move(src, 2, 0)).toEqual(['c', 'a', 'b']);
    expect(src).toEqual(['a', 'b', 'c']);
  });
});
