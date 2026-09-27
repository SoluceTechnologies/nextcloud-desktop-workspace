import { describe, expect, it } from 'vitest';
import { withMembership } from '@/lib/withMembership';

describe('withMembership', () => {
  it('adds and removes an item without mutating the input', () => {
    const original = new Set(['a']);
    expect([...withMembership(original, 'b', true)]).toEqual(['a', 'b']);
    expect([...withMembership(original, 'a', false)]).toEqual([]);
    expect([...original]).toEqual(['a']);
  });

  it('returns the same set when nothing changes', () => {
    const original = new Set(['a']);
    expect(withMembership(original, 'a', true)).toBe(original);
    expect(withMembership(original, 'b', false)).toBe(original);
  });
});
