export function move<T>(items: T[], from: number, to: number): T[] {
  const reordered = items.slice();
  const [item] = reordered.splice(from, 1);
  reordered.splice(to, 0, item);
  return reordered;
}
