import type { Download } from '@/lib/types';
import { percent } from './DownloadsPanel';
import './Downloads.css';

const RADIUS = 8;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

export function DownloadsButton({ downloads, onOpen }: { downloads: Download[]; onOpen: () => void }) {
  if (downloads.length === 0) return null;
  const active = downloads.filter((download) => download.state === 'active');
  const known = active.map(percent).filter((value): value is number => value !== null);
  const indeterminate = active.length > 0 && known.length < active.length;
  const overall = known.length ? known.reduce((sum, value) => sum + value, 0) / known.length : 100;
  const label = active.length ? `Downloads, ${active.length} in progress` : 'Downloads';
  return (
    <button type="button" className={indeterminate ? 'downloads-button busy' : 'downloads-button'} aria-label={label} title={label} onClick={onOpen}>
      <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
        <circle className="track" cx="10" cy="10" r={RADIUS} />
        {active.length > 0 && (
          <circle
            className="value"
            cx="10"
            cy="10"
            r={RADIUS}
            strokeDasharray={CIRCUMFERENCE}
            strokeDashoffset={indeterminate ? CIRCUMFERENCE * 0.7 : CIRCUMFERENCE * (1 - overall / 100)}
          />
        )}
        <path d="M10 6v6M7.5 9.5 10 12l2.5-2.5" />
      </svg>
      {active.length > 0 && <span className="count">{active.length}</span>}
    </button>
  );
}
