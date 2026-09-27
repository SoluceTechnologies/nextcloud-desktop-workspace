const HOST_PREFIXES = new Set(['www', 'cloud', 'nextcloud', 'nc', 'drive', 'files']);

export function initials(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return '?';
  if (!/\s/.test(trimmed) && trimmed.includes('.')) {
    const labels = trimmed.split('.');
    const mainLabel = labels.length > 2 && HOST_PREFIXES.has(labels[0].toLowerCase()) ? labels[1] : labels[0];
    return mainLabel.charAt(0).toUpperCase();
  }
  return trimmed
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => word.charAt(0))
    .join('')
    .toUpperCase();
}
