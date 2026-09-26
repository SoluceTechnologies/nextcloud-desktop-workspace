import type { KeyboardEvent } from 'react';

export const onEscape = (close: () => void) => (e: KeyboardEvent) => {
  if (e.key === 'Escape') close();
};
