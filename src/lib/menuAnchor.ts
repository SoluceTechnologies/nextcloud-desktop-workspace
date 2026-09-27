// Window-relative, in CSS pixels: the shell webview covers the whole window.
export type MenuAnchor = { x: number; y: number };

export const atPointer = (event: { clientX: number; clientY: number }): MenuAnchor => ({
  x: event.clientX,
  y: event.clientY,
});

export const below = (element: Element): MenuAnchor => {
  const rect = element.getBoundingClientRect();
  return { x: rect.left, y: rect.bottom };
};
