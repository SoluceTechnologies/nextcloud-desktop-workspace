// Injected into every Nextcloud page (main frame) by webviews.rs. Report-only bridge to the shell:
// the page can say where it is and what it looks like, and nothing else (spec §6).
(() => {
  if (window.top !== window || window.__ncwBridge) return;
  window.__ncwBridge = true;

  const invoke = (cmd, args) => {
    try {
      window.__TAURI_INTERNALS__.invoke(cmd, args).catch(() => {});
    } catch {
      // not granted on this origin (identity provider pages, external sites)
    }
  };

  // Mirror of router.rs app_id(): the app a path belongs to, or null for pages that own no tab.
  const appKey = (path) => {
    const segs = path.split('/').filter(Boolean);
    const i = segs.findIndex((s) => s === 'apps' || s === 'settings' || s === 's');
    if (i < 0) return null;
    const kind = segs[i];
    const next = segs[i + 1];
    if (kind === 'settings') return 'settings';
    if (!next) return null;
    if (kind === 's') return 'share:' + next;
    return /^(user_oidc|user_saml|twofactor_)/.test(next) ? null : next;
  };

  // Links to another origin or another app become window.open → on_new_window → shell routing (spec §5.4).
  addEventListener(
    'click',
    (e) => {
      if (e.button !== 0) return;
      const a = e.target instanceof Element ? e.target.closest('a[href]') : null;
      if (!a || a.hasAttribute('download') || (a.target && a.target !== '_self')) return;
      let url;
      try {
        url = new URL(a.href, location.href);
      } catch {
        return;
      }
      if (url.protocol !== 'http:' && url.protocol !== 'https:') return;
      const here = appKey(location.pathname);
      const there = appKey(url.pathname);
      if (url.origin !== location.origin || (here && there && here !== there)) {
        e.preventDefault();
        e.stopImmediatePropagation();
        window.open(url.href, '_blank');
      }
    },
    true,
  );

  // Location: page load + SPA changes.
  let last = '';
  const report = () => {
    if (location.href === last) return;
    last = location.href;
    invoke('nc_report_location', { url: last });
  };
  let timer;
  const soon = () => {
    clearTimeout(timer);
    timer = setTimeout(report, 250);
  };
  for (const k of ['pushState', 'replaceState']) {
    const orig = history[k];
    history[k] = function (...args) {
      const r = orig.apply(this, args);
      soon();
      return r;
    };
  }
  addEventListener('popstate', soon);
  addEventListener('hashchange', soon);

  // App menu: hidden initial state first (complete list), DOM links as fallback.
  const decode = (b64) => new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  const apps = () => {
    const el = document.getElementById('initial-state-core-apps');
    if (el) {
      try {
        return Object.values(JSON.parse(decode(el.value)))
          .filter((x) => x && x.href)
          .map((x) => ({ name: String(x.name ?? ''), href: new URL(x.href, location.href).href }));
      } catch {
        // fall through
      }
    }
    return [...document.querySelectorAll('.app-menu-entry a, #appmenu li a')].map((a) => ({
      name: (a.getAttribute('aria-label') || a.textContent || '').trim(),
      href: a.href,
    }));
  };

  // Workspace icon, as a 128 px PNG data URL (≤ 64 KB on the Rust side): the server's custom favicon,
  // else its custom logo on the theme colour, else '' (nothing custom: the shell shows initials).
  // Nextcloud theming defines --image-favicon/logoheader/logo only for images an admin uploaded; the
  // default favicon is the current app's icon, which says nothing about the server. null = unknown
  // (theming CSS not applied yet, image failed): the shell keeps what it has. Drawn through a canvas
  // because servers send these as octet-stream, ICO, SVG or large PNGs; <img> sniffs all of them.
  const cssImage = (style, name) => {
    const m = style.getPropertyValue(name).match(/url\(\s*['"]?([^'")]+)/);
    return m ? new URL(m[1], location.href).href : null;
  };
  const icon = async () => {
    const style = getComputedStyle(document.body);
    const primary = style.getPropertyValue('--color-primary').trim();
    if (!primary) return null;
    const favicon = cssImage(style, '--image-favicon');
    const logo = favicon ? null : cssImage(style, '--image-logoheader') || cssImage(style, '--image-logo');
    if (!favicon && !logo) return '';
    try {
      const img = new Image();
      img.src = favicon || logo;
      await img.decode();
      const size = 128;
      const pad = favicon ? 0 : 20;
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = size;
      const g = canvas.getContext('2d');
      if (logo) {
        g.fillStyle = primary;
        g.fillRect(0, 0, size, size);
      }
      const iw = img.naturalWidth || size;
      const ih = img.naturalHeight || size;
      const scale = Math.min((size - 2 * pad) / iw, (size - 2 * pad) / ih);
      g.drawImage(img, (size - iw * scale) / 2, (size - ih * scale) / 2, iw * scale, ih * scale);
      return canvas.toDataURL('image/png');
    } catch {
      return null;
    }
  };

  document.addEventListener('fullscreenchange', () => {
    invoke('nc_report_fullscreen', { on: !!document.fullscreenElement });
  });

  const ready = async () => {
    report();
    invoke('nc_report_meta', { icon: await icon(), apps: apps().slice(0, 64) });
  };
  if (document.readyState === 'loading') addEventListener('DOMContentLoaded', ready, { once: true });
  else ready();
})();
