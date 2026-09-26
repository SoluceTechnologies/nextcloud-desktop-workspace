export function move<T>(items: T[], from: number, to: number): T[] {
  const out = items.slice();
  const [item] = out.splice(from, 1);
  out.splice(to, 0, item);
  return out;
}
