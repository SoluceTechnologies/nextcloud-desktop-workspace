import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef } from 'react';

export function useTauriEvent<T>(name: string, handler: (payload: T) => void) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;
  useEffect(() => {
    const unlisten = listen<T>(name, (event) => handlerRef.current(event.payload));
    return () => {
      unlisten.then((stopListening) => stopListening());
    };
  }, [name]);
}
