(() => {
  if (window.top !== window || window.__ncwBridge) return;
  window.__ncwBridge = true;

  if (navigator.serviceWorker) {
    navigator.serviceWorker.register = () =>
      Promise.reject(new DOMException('Service workers are disabled in NC Workspaces', 'SecurityError'));
    navigator.serviceWorker
      .getRegistrations()
      .then((registrations) => registrations.forEach((registration) => registration.unregister()))
      .catch(() => {});
  }

  const invoke = (command, args) => {
    try {
      window.__TAURI_INTERNALS__.invoke(command, args).catch(() => {});
    } catch {}
  };

  const appOfPath = (path) => {
    const segments = path.split('/').filter(Boolean);
    const index = segments.findIndex((segment) => segment === 'apps' || segment === 'settings' || segment === 's');
    if (index < 0) return null;
    const kind = segments[index];
    const next = segments[index + 1];
    if (kind === 'settings') return 'settings';
    if (!next) return null;
    if (kind === 's') return 'share:' + next;
    return /^(user_oidc|user_saml|twofactor_)/.test(next) ? null : next;
  };

  addEventListener(
    'click',
    (event) => {
      if (event.button !== 0) return;
      const anchor = event.target instanceof Element ? event.target.closest('a[href]') : null;
      if (!anchor || anchor.hasAttribute('download') || (anchor.target && anchor.target !== '_self')) return;
      let url;
      try {
        url = new URL(anchor.href, location.href);
      } catch {
        return;
      }
      if (url.protocol !== 'http:' && url.protocol !== 'https:') return;
      const currentApp = appOfPath(location.pathname);
      const targetApp = appOfPath(url.pathname);
      const otherApp = currentApp && targetApp && currentApp !== targetApp;
      if (url.origin !== location.origin || otherApp) {
        event.preventDefault();
        event.stopImmediatePropagation();
        window.open(url.href, '_blank');
      }
    },
    true,
  );

  let lastReported = '';
  const reportLocation = () => {
    if (location.href === lastReported) return;
    lastReported = location.href;
    invoke('nc_report_location', { url: lastReported });
  };
  let reportTimer;
  const reportLocationSoon = () => {
    clearTimeout(reportTimer);
    reportTimer = setTimeout(reportLocation, 250);
  };
  for (const method of ['pushState', 'replaceState']) {
    const original = history[method];
    history[method] = function (...args) {
      const result = original.apply(this, args);
      reportLocationSoon();
      return result;
    };
  }
  addEventListener('popstate', reportLocationSoon);
  addEventListener('hashchange', reportLocationSoon);

  const decodeBase64 = (encoded) =>
    new TextDecoder().decode(Uint8Array.from(atob(encoded), (character) => character.charCodeAt(0)));

  const appLinks = () => {
    const initialState = document.getElementById('initial-state-core-apps');
    if (initialState) {
      try {
        return Object.values(JSON.parse(decodeBase64(initialState.value)))
          .filter((app) => app && app.href)
          .map((app) => ({ name: String(app.name ?? ''), href: new URL(app.href, location.href).href }));
      } catch {}
    }
    return [...document.querySelectorAll('.app-menu-entry a, #appmenu li a')].map((link) => ({
      name: (link.getAttribute('aria-label') || link.textContent || '').trim(),
      href: link.href,
    }));
  };

  const cssImage = (style, property) => {
    const match = style.getPropertyValue(property).match(/url\(\s*['"]?([^'")]+)/);
    return match ? new URL(match[1], location.href).href : null;
  };

  const ICON_SIZE = 128;
  const LOGO_PADDING = 20;

  const workspaceIcon = async () => {
    const style = getComputedStyle(document.body);
    const primaryColor = style.getPropertyValue('--color-primary').trim();
    if (!primaryColor) return null;
    const favicon = cssImage(style, '--image-favicon');
    const logo = favicon ? null : cssImage(style, '--image-logoheader') || cssImage(style, '--image-logo');
    if (!favicon && !logo) return '';
    try {
      const image = new Image();
      image.src = favicon || logo;
      await image.decode();
      const padding = favicon ? 0 : LOGO_PADDING;
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = ICON_SIZE;
      const context = canvas.getContext('2d');
      if (logo) {
        context.fillStyle = primaryColor;
        context.fillRect(0, 0, ICON_SIZE, ICON_SIZE);
      }
      const width = image.naturalWidth || ICON_SIZE;
      const height = image.naturalHeight || ICON_SIZE;
      const scale = Math.min((ICON_SIZE - 2 * padding) / width, (ICON_SIZE - 2 * padding) / height);
      const x = (ICON_SIZE - width * scale) / 2;
      const y = (ICON_SIZE - height * scale) / 2;
      context.drawImage(image, x, y, width * scale, height * scale);
      return canvas.toDataURL('image/png');
    } catch {
      return null;
    }
  };

  document.addEventListener('fullscreenchange', () => {
    invoke('nc_report_fullscreen', { fullscreen: !!document.fullscreenElement });
  });

  const onReady = async () => {
    reportLocation();
    invoke('nc_report_meta', { icon: await workspaceIcon(), apps: appLinks().slice(0, 64) });
  };
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', onReady, { once: true });
  else onReady();
})();
