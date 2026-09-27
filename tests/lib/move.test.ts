import { describe, expect, it } from 'vitest';
import { move } from '@/lib/move';

describe('move', () => {
  it('moves an item without mutating the input', () => {
    const original = ['a', 'b', 'c'];
    expect(move(original, 0, 2)).toEqual(['b', 'c', 'a']);
    expect(move(original, 2, 0)).toEqual(['c', 'a', 'b']);
    expect(original).toEqual(['a', 'b', 'c']);
  });
});
