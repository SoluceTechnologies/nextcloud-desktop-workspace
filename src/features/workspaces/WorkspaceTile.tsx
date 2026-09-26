import { initials } from './initials';
import './WorkspaceTile.css';

export function TileFace({ icon, name }: { icon: string | null; name: string }) {
  return icon ? <img src={icon} alt="" draggable={false} /> : <span className="initials">{initials(name)}</span>;
}
