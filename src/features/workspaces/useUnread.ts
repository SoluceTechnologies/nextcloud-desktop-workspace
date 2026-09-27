import { useState } from 'react';
import { useTauriEvent } from '@/lib/useTauriEvent';

export function useUnread(): Record<string, number> {
  const [unread, setUnread] = useState<Record<string, number>>({});
  useTauriEvent<Record<string, number>>('unread', setUnread);
  return unread;
}
