import { describe, expect, it } from 'vitest';
import { move } from '@/lib/move';

describe('move', () => {
  it('moves an item without mutating the input', () => {
    const src = ['a', 'b', 'c'];
    expect(move(src, 0, 2)).toEqual(['b', 'c', 'a']);
    expect(move(src, 2, 0)).toEqual(['c', 'a', 'b']);
    expect(src).toEqual(['a', 'b', 'c']);
  });
});
