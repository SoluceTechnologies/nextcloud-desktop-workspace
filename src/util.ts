const HOST_PREFIXES = new Set(['www', 'cloud', 'nextcloud', 'nc', 'drive', 'files']);

/** Sidebar fallback when a workspace has no icon: "cloud.soluce.com" → "S", "Soluce Cloud" → "SC". */
export function initials(name: string): string {
  const n = name.trim();
  if (!n) return '?';
  if (!/\s/.test(n) && n.includes('.')) {
    const labels = n.split('.');
    const main = labels.length > 2 && HOST_PREFIXES.has(labels[0].toLowerCase()) ? labels[1] : labels[0];
    return main.charAt(0).toUpperCase();
  }
  return n
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w.charAt(0))
    .join('')
    .toUpperCase();
}

/** Stable colour per workspace id for the initials badge. */
export function hue(id: string): number {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

export function move<T>(items: T[], from: number, to: number): T[] {
  const out = items.slice();
  const [item] = out.splice(from, 1);
  out.splice(to, 0, item);
  return out;
}
