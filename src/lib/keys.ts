import type { KeyboardEvent } from 'react';

export const onEscape = (close: () => void) => (event: KeyboardEvent) => {
  if (event.key === 'Escape') close();
};
