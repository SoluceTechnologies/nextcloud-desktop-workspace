import type { ReactNode } from 'react';
import { MonitorIcon, MoonIcon, SunIcon } from '@/components/icons';
import { api } from '@/lib/api';
import type { Appearance } from '@/lib/types';
import './AppearanceSwitch.css';

const APPEARANCES: { value: Appearance; label: string; icon: ReactNode }[] = [
  { value: 'system', label: 'System', icon: <MonitorIcon size={15} /> },
  { value: 'light', label: 'Light', icon: <SunIcon size={15} /> },
  { value: 'dark', label: 'Dark', icon: <MoonIcon size={15} /> },
];

export function AppearanceSwitch({ theme }: { theme: Appearance }) {
  return (
    <div className="appearance" role="radiogroup" aria-label="Appearance">
      {APPEARANCES.map((a) => (
        <button
          key={a.value}
          role="radio"
          aria-checked={theme === a.value}
          aria-label={a.label}
          title={a.label}
          onClick={() => api.setTheme(a.value)}
        >
          {a.icon}
        </button>
      ))}
    </div>
  );
}
