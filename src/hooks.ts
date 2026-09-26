import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';
import { api } from './api';
import type { AppState } from './types';

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

/** Latest engine snapshot: initial get_state, then every state-changed event. */
export function useAppState(): AppState | null {
  const [state, setState] = useState<AppState | null>(null);
  useTauriEvent<AppState>('state-changed', setState);
  useEffect(() => {
    api.getState().then((s) => setState((cur) => cur ?? s));
  }, []);
  return state;
}
