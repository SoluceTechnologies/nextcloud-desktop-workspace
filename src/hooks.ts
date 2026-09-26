import { listen } from '@tauri-apps/api/event';
import { createElement, useEffect, useRef, useState, type ReactNode } from 'react';
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

type NoticeData = { text: string; path?: string };

/** Transient message in the tab bar: startup notice or last finished download. */
export function useNotice(): ReactNode {
  const [notice, setNotice] = useState<NoticeData | null>(null);
  useEffect(() => {
    api.takeNotice().then((text) => text && setNotice({ text }));
  }, []);
  useTauriEvent<{ path: string | null; success: boolean }>('download-finished', (d) => {
    const name = d.path?.split(/[\\/]/).pop() ?? 'file';
    setNotice(d.success ? { text: `Downloaded ${name}`, path: d.path ?? undefined } : { text: 'Download failed' });
  });
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(null), 8000);
    return () => clearTimeout(t);
  }, [notice]);
  if (!notice) return null;
  const path = notice.path;
  return createElement(
    'div',
    { className: 'notice', role: 'status' },
    notice.text,
    path ? createElement('button', { onClick: () => api.revealDownload(path) }, 'Show') : null,
  );
}

/** Tabs whose page is loading (`tab-loading` events). The timeout covers loads that fail: wry reports no end then. */
export function useLoadingTabs(): Set<string> {
  const [loading, setLoading] = useState<Set<string>>(() => new Set());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  useTauriEvent<{ tab: string; loading: boolean }>('tab-loading', ({ tab, loading: on }) => {
    clearTimeout(timers.current.get(tab));
    if (on) timers.current.set(tab, setTimeout(() => update(tab, false), 30_000));
    update(tab, on);
  });
  function update(tab: string, on: boolean) {
    setLoading((cur) => {
      if (cur.has(tab) === on) return cur;
      const next = new Set(cur);
      if (on) next.add(tab);
      else next.delete(tab);
      return next;
    });
  }
  return loading;
}
