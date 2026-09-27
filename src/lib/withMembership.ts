export function withMembership<T>(current: Set<T>, item: T, member: boolean): Set<T> {
  if (current.has(item) === member) return current;
  const next = new Set(current);
  if (member) next.add(item);
  else next.delete(item);
  return next;
}
