import { useEffect, useState } from 'react';
import { api } from './api';
import type { AppState } from './types';
import { useTauriEvent } from './useTauriEvent';

export function useAppState(): AppState | null {
  const [state, setState] = useState<AppState | null>(null);
  useTauriEvent<AppState>('state-changed', setState);
  useEffect(() => {
    api.getState().then((s) => setState((cur) => cur ?? s));
  }, []);
  return state;
}
