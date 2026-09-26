import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef } from 'react';

export function useTauriEvent<T>(name: string, handler: (payload: T) => void) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    const unlisten = listen<T>(name, (e) => ref.current(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, [name]);
}
