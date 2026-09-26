const HOST_PREFIXES = new Set(['www', 'cloud', 'nextcloud', 'nc', 'drive', 'files']);

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
