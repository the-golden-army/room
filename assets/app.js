/* ROOM client — vanilla JS, no build step. */
(() => {
  'use strict';

  const BOOT = window.__BOOT__ || {};
  const ME = BOOT.me || { id: 0, name: 'Me', initials: '?' };
  const SETTINGS = BOOT.settings || {};

  const $ = (sel, root) => (root || document).querySelector(sel);
  const $$ = (sel, root) => Array.from((root || document).querySelectorAll(sel));

  const esc = (s) =>
    String(s == null ? '' : s).replace(/[&<>"']/g, (c) =>
      ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c])
    );

  const state = {
    users: new Map((BOOT.users || []).map((u) => [u.id, u])),
    devices: new Map((BOOT.devices || []).map((d) => [d.id, d])),
    section: BOOT.section || 'chat',
    ws: null,
    wsOpen: false,
    convId: 0,
    lastMsgId: 0,
    typingTimers: new Map(),
    uploads: new Map(),
    call: null,
    pc: new Map(),
    localStream: null,
    screenStream: null,
    micOn: true,
    camOn: true,
    selectedFile: null,
    fileView: 'list',
    composing: false,
    replyTo: 0,
  };

  /* ------------------------------------------------------------ theme */

  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
  function applyTheme(mode) {
    const dark = mode === 'dark' || (mode === 'system' && prefersDark.matches);
    document.documentElement.classList.toggle('dark', !!dark);
    const btn = $('#theme-btn');
    if (btn) btn.innerHTML = icon(dark ? 'sun' : 'moon', 'h-[21px] w-[21px]') + '<span class="rail-label">Theme</span>';
    document.querySelectorAll('meta[name="color-scheme"]').forEach((m) =>
      m.setAttribute('content', dark ? 'dark' : 'light')
    );
  }
  function currentTheme() {
    const cls = document.documentElement.classList.contains('dark');
    if (SETTINGS.theme === 'system') return cls ? 'dark' : 'light';
    return cls ? 'dark' : 'light';
  }
  async function setTheme(mode) {
    SETTINGS.theme = mode;
    const next = mode === 'dark' ? 'dark' : mode === 'light' ? 'light' : (prefersDark.matches ? 'dark' : 'light');
    applyTheme(mode);
    try {
      await fetch('/api/admin/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ theme: next }),
      });
    } catch (e) {}
  }
  prefersDark.addEventListener('change', () => {
    if ((SETTINGS.theme || 'light') === 'system') applyTheme('system');
  });

  /* ------------------------------------------------------------ icons */

  const ICONS = {
    chat: '<path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/>',
    phone: '<path d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"/>',
    folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
    shield: '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>',
    search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
    plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
    send: '<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>',
    paperclip: '<path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"/>',
    smile: '<circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" x2="9.01" y1="9" y2="9"/><line x1="15" x2="15.01" y1="9" y2="9"/>',
    reply: '<path d="m9 17-5-5 5-5"/><path d="M4 12h9a5 5 0 0 1 5 5v1"/>',
    trash: '<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>',
    edit: '<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/>',
    more: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
    moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
    x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>',
    upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 8 5-5 5 5"/><path d="M12 3v12"/>',
    'folder-plus': '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/><path d="M12 10v6"/><path d="M9 13h6"/>',
    file: '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>',
    'file-text': '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
    image: '<rect width="18" height="18" x="3" y="3" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>',
    video: '<path d="m22 8-6 4 6 4V8Z"/><rect width="14" height="12" x="2" y="6" rx="2"/>',
    'video-off': '<path d="M10.5 6H8a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-2.5"/><path d="M22 8l-4 2.5V8"/><path d="m2 2 20 20"/>',
    mic: '<path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/>',
    'mic-off': '<path d="M9 9v3a3 3 0 0 0 5.12 2.12"/><path d="M15 9.34V5a3 3 0 0 0-5.94-.6"/><path d="M19 10v2a7 7 0 0 1-.11 1.23"/><path d="M5 10v2a7 7 0 0 0 12 5"/><path d="M12 19v3"/><path d="m2 2 20 20"/>',
    share: '<path d="M13 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-3"/><path d="M8 21h8"/><path d="M12 17v4"/><path d="m17 8 5-5"/><path d="M12 3h5v5"/>',
    'phone-off': '<path d="M10.68 13.31a16 16 0 0 0 3.41 2.6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7 2 2 0 0 1 1.72 2v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.42 19.42 0 0 1-3.33-2.67m-2.67-3.34a19.79 19.79 0 0 1-2.05-8.36A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91"/><path d="m2 2 20 20"/>',
    users: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
    clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
    'chev-left': '<path d="m15 18-6-6 6-6"/>',
    'chev-right': '<path d="m9 18 6-6-6-6"/>',
    'chev-down': '<path d="m6 9 6 6 6-6"/>',
    bell: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>',
    'bell-off': '<path d="M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5"/><path d="M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/><path d="m2 2 20 20"/>',
    drive: '<path d="M22 12H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><path d="M6 16h.01"/><path d="M10 16h.01"/>',
    monitor: '<rect width="20" height="14" x="2" y="3" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/>',
    smartphone: '<rect width="14" height="20" x="5" y="2" rx="2"/><path d="M12 18h.01"/>',
    user: '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    lock: '<rect width="18" height="11" x="3" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    logout: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5"/><path d="M21 12H9"/>',
    settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
    warning: '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
    archive: '<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>',
    refresh: '<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>',
    power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>',
    server: '<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><path d="M6 6h.01"/><path d="M6 18h.01"/>',
    activity: '<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>',
    inbox: '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>',
    code: '<path d="m16 18 6-6-6-6"/><path d="m8 6-6 6 6 6"/>',
    grid: '<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>',
    list: '<path d="M8 6h13"/><path d="M8 12h13"/><path d="M8 18h13"/><path d="M3 6h.01"/><path d="M3 12h.01"/><path d="M3 18h.01"/>',
    restore: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>',
    key: '<path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4"/>',
    wifi: '<path d="M5 13a10 10 0 0 1 14 0"/><path d="M8.5 16.5a5 5 0 0 1 7 0"/><path d="M2 8.82a15 15 0 0 1 20 0"/><path d="M12 20h.01"/>',
  };

  function icon(name, cls) {
    const body = ICONS[name] || '<circle cx="12" cy="12" r="9"/>';
    return `<svg class="${cls || 'h-5 w-5 shrink-0'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
  }

  /* ------------------------------------------------------------ utils */

  const timeHHMM = (ms) => {
    if (!ms) return '';
    const d = new Date(ms);
    return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0');
  };
  const rel = (ms) => {
    if (!ms) return '—';
    const diff = Date.now() - ms;
    if (diff < 45000) return 'just now';
    if (diff < 3600000) return Math.round(diff / 60000) + 'm ago';
    if (diff < 86400000) return Math.round(diff / 3600000) + 'h ago';
    if (diff < 7 * 86400000) return Math.round(diff / 86400000) + 'd ago';
    const d = new Date(ms);
    return d.toLocaleDateString(undefined, { day: '2-digit', month: 'short' });
  };
  const bytes = (n) => {
    if (n < 0) return '0 B';
    const u = ['B', 'KB', 'MB', 'GB', 'TB'];
    let i = 0, v = n;
    while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
    return (i ? v.toFixed(1) : v) + ' ' + u[i];
  };
  const clock = (ms) => {
    const t = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
    const p = (x) => String(x).padStart(2, '0');
    return h ? `${p(h)}:${p(m)}:${p(s)}` : `${p(m)}:${p(s)}`;
  };

  function bodyHtml(src) {
    const e = esc(src);
    return e
      .replace(/(https?:\/\/[^\s<>"']+)/g, (u) => {
        const clean = u.replace(/[),.;]+$/, '');
        return `<a href="${clean}" target="_blank" rel="noreferrer noopener">${clean}</a>`;
      })
      .replace(/\n/g, '<br>');
  }

  function userById(id) {
    return state.users.get(id) || { id, name: 'Unknown', initials: '?', color: '#64748b', presence: 'offline' };
  }

  function avatarHtml(u, cls, presence) {
    const dot = presence
      ? `<span class="presence ${u.presence === 'online' ? 'on' : u.presence === 'away' ? 'away' : ''}"></span>`
      : '';
    const inner = u.avatar ? esc(u.avatar) : esc(u.initials || '?');
    return `<span class="relative inline-block shrink-0 ${cls}"><span class="grid h-full w-full place-items-center rounded-full font-semibold text-white" style="background:${u.color};font-size:.72em">${inner}</span>${dot}</span>`;
  }

  /* ------------------------------------------------------------ toast + modal */

  function toast(msg, kind) {
    const box = $('#toasts');
    const el = document.createElement('div');
    el.className = 'toast pointer-events-auto';
    const color = kind === 'error' ? '#ef4444' : kind === 'ok' ? '#10b981' : '#5333f4';
    const ic = kind === 'error' ? 'warning' : kind === 'ok' ? 'check' : 'info';
    el.innerHTML = `<span style="color:${color}" class="mt-0.5">${icon(ic, 'h-4 w-4')}</span><div class="flex-1 text-[13px] leading-snug">${esc(msg)}</div>`;
    box.appendChild(el);
    setTimeout(() => {
      el.classList.add('out');
      setTimeout(() => el.remove(), 200);
    }, 3400);
  }

  function modal(title, body, footer) {
    closeModal();
    const root = $('#modal-root');
    const el = document.createElement('div');
    el.className = 'modal-backdrop';
    el.setAttribute('data-modal', '');
    el.innerHTML = `<div class="modal-card max-h-[88dvh] flex flex-col">
      <div class="flex items-start gap-3 border-b line px-5 py-4">
        <div class="flex-1 text-[15px] font-bold">${esc(title)}</div>
        <button class="btn btn-ghost btn-icon" data-close type="button" aria-label="Close">${icon('x', 'h-4 w-4')}</button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">${body}</div>
      <div class="flex items-center justify-end gap-2 border-t line px-5 py-3.5">${footer || ''}</div>
    </div>`;
    el.addEventListener('click', (e) => {
      if (e.target === el || e.target.closest('[data-close]')) closeModal();
    });
    root.appendChild(el);
    const f = el.querySelector('input,textarea,select,button');
    if (f) setTimeout(() => f.focus(), 30);
    return el;
  }
  function closeModal() {
    $$('#modal-root [data-modal]').forEach((n) => n.remove());
  }
  function confirmDialog(title, body, actionLabel, fn, danger) {
    const el = modal(
      title,
      `<div class="text-[13px] leading-relaxed ink2">${body}</div>`,
      `<button class="btn btn-ghost" data-close type="button">Cancel</button>
       <button class="btn ${danger ? 'btn-danger' : 'btn-brand'}" data-confirm type="button">${esc(actionLabel)}</button>`
    );
    el.querySelector('[data-confirm]').addEventListener('click', async () => {
      try { await fn(); } catch (e) { toast(e.message || 'Action failed', 'error'); }
      closeModal();
    });
  }
  function busy(btn, on) {
    if (!btn) return;
    btn.dataset.busy = on ? '1' : '';
    btn.disabled = !!on;
    if (on) btn.prepend(spinnerEl());
  }
  function spinnerEl() {
    const s = document.createElement('span');
    s.className = 'inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent';
    s.style.color = 'var(--color-brand-600)';
    return s;
  }

  /* ------------------------------------------------------------ api */

  async function api(path, opts) {
    const o = Object.assign({ headers: {} }, opts || {});
    // Only plain objects/arrays are JSON. Binary payloads (ArrayBuffer, Blob,
    // File, typed arrays, URLSearchParams) must be sent as-is: JSON.stringify
    // turns an ArrayBuffer into "{}" and silently uploads two empty bytes.
    const raw =
      o.body instanceof FormData ||
      o.body instanceof Blob ||
      o.body instanceof ArrayBuffer ||
      ArrayBuffer.isView(o.body) ||
      o.body instanceof URLSearchParams ||
      (typeof ReadableStream !== 'undefined' && o.body instanceof ReadableStream);
    if (o.body && typeof o.body !== 'string' && !raw) {
      o.headers['Content-Type'] = 'application/json';
      o.body = JSON.stringify(o.body);
    }
    const res = await fetch(path, o);
    if (res.status === 401) {
      location.href = '/gate';
      throw new Error('unauthorised');
    }
    const ct = res.headers.get('content-type') || '';
    let data = null;
    if (ct.includes('application/json')) data = await res.json();
    if (!res.ok) throw new Error((data && data.error) || `Request failed (${res.status})`);
    return data;
  }

  /* ------------------------------------------------------------ websocket */

  function wsUrl() {
    const proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
    return proto + location.host + '/ws';
  }
  function connectWs() {
    if (state.ws && (state.ws.readyState === WebSocket.OPEN || state.ws.readyState === WebSocket.CONNECTING)) return;
    const ws = new WebSocket(wsUrl());
    state.ws = ws;
    ws.onopen = () => {
      state.wsOpen = true;
      document.body.classList.remove('ws-off');
      ws.send(JSON.stringify({ t: 'presence:active' }));
    };
    ws.onmessage = (ev) => {
      let msg;
      try { msg = JSON.parse(ev.data); } catch (e) { return; }
      handleWs(msg);
    };
    ws.onclose = () => {
      state.wsOpen = false;
      document.body.classList.add('ws-off');
      setTimeout(connectWs, 1200 + Math.random() * 1500);
    };
    ws.onerror = () => {};
  }
  function wsSend(obj) {
    if (state.ws && state.ws.readyState === WebSocket.OPEN) state.ws.send(JSON.stringify(obj));
  }

  function handleWs(msg) {
    switch (msg.t) {
      case 'presence':
        (msg.users || []).forEach((id) => {
          const u = state.users.get(id);
          if (u) u.presence = 'online';
        });
        state.users.forEach((u) => {
          if (!(msg.users || []).includes(u.id)) u.presence = u.presence === 'away' ? 'away' : 'offline';
        });
        renderPresence();
        break;
      case 'message:new':
        if (msg.message && msg.message.conversation_id === state.convId) appendMessage(msg.message, true);
        refreshConvList(msg.conversation_id);
        break;
      case 'message:updated':
        if (msg.message) replaceMessage(msg.message);
        break;
      case 'message:deleted':
        removeMessage(msg.message_id);
        break;
      case 'typing':
        showTyping(msg.conversation_id, msg.name);
        break;
      case 'conversations:changed':
        loadConversations();
        break;
      case 'users:changed':
        loadUsers();
        break;
      case 'requests:changed':
      case 'devices:changed':
        if (state.section === 'admin') reloadAdmin();
        updatePendingBadge();
        break;
      case 'call:incoming':
        showIncomingCall(msg);
        break;
      case 'call:peer':
        if (state.call && msg.call_id === state.call.id) callPeerEvent(msg);
        break;
      case 'call:ended':
        if (state.call && msg.call_id === state.call.id) endCallUi('Call ended');
        break;
      case 'call:signal':
        if (state.call && msg.call_id === state.call.id) handleSignal(msg);
        break;
      case 'server:shutdown':
        toast('Server is shutting down…', 'info');
        break;
      case 'server:restart':
        toast('Server restarting — reconnecting…', 'info');
        setTimeout(() => location.reload(), 2500);
        break;
    }
  }

  function renderPresence() {
    const me = $('#me-avatar');
    if (me) me.innerHTML = avatarHtml(ME, 'h-10 w-10 text-sm', true);
    $$('.presence').forEach(() => {});
    const t = $('#details-scroll');
    if (t && state.convId) loadDetails(state.convId);
  }

  function updatePendingBadge() {
    api('/api/admin/requests')
      .then((d) => {
        const n = (d.pending || []).length;
        BOOT.pending_requests = n;
        $$('#rail-nav .rail-btn, #mobnav .rail-btn').forEach((a) => {
          if (a.dataset.nav !== 'admin') return;
          let b = a.querySelector('.badge');
          if (n > 0) {
            if (!b) {
              b = document.createElement('span');
              b.className = 'badge absolute right-1 top-1 grid h-4 min-w-4 place-items-center rounded-full px-1 text-[9px] font-bold text-white';
              b.style.background = '#ef4444';
              a.querySelector('span.relative')?.appendChild(b);
            }
            b.textContent = n;
          } else if (b) b.remove();
        });
      })
      .catch(() => {});
  }

  /* ------------------------------------------------------------ chat */

  async function loadUsers() {
    try {
      const d = await api('/api/users');
      d.users.forEach((u) => state.users.set(u.id, u));
      BOOT.users = Array.from(state.users.values());
    } catch (e) {}
  }

  async function loadConversations() {
    try {
      const d = await api('/api/conversations');
      BOOT.conversations = d.conversations || [];
      const list = $('#conv-list');
      if (list) {
        list.innerHTML = (d.conversations || []).map((c) => convItemHtml(c)).join('') || emptyBlock('No conversations yet');
      }
    } catch (e) {}
  }

  function convItemHtml(c) {
    const preview = c.last_preview
      ? (c.kind === 'group' && c.last_sender ? `${esc(c.last_sender)}: ` : '') + esc(c.last_preview)
      : 'No messages yet';
    const unread =
      c.unread > 0
        ? `<span class="ml-auto grid h-[18px] min-w-[18px] place-items-center rounded-full px-1 text-[10px] font-bold text-white" style="background:${c.muted ? 'var(--color-ink3)' : 'var(--color-brand-600)'}">${c.unread > 99 ? '99+' : c.unread}</span>`
        : c.muted
        ? `<span class="ml-auto ink3">${icon('bell-off', 'h-3.5 w-3.5')}</span>`
        : '';
    return `<a href="/chat/${c.id}" class="conv-item ${c.id === state.convId ? 'active' : ''}" data-conv="${c.id}" data-kind="${c.kind}" data-unread="${c.unread > 0 ? '1' : '0'}">
      ${avatarHtml({ id: 0, initials: c.initials, color: c.color, presence: '' }, 'h-10 w-10 text-[13px]', false)}
      <span class="min-w-0 flex-1"><span class="flex items-center gap-1.5">
        <span class="truncate text-[13.5px] font-semibold">${esc(c.title)}</span>
        <span class="ml-auto shrink-0 text-[10px] ink3">${c.last_message_at ? timeHHMM(c.last_message_at) : ''}</span></span>
        <span class="mt-0.5 block truncate text-xs ink3">${preview}</span></span>${unread}</a>`;
  }

  function emptyBlock(text) {
    return `<div class="px-4 py-8 text-center text-[12.5px] ink3">${esc(text)}</div>`;
  }

  function refreshConvList(cid) {
    loadConversations();
  }

  function msgHtml(m, grouped) {
    if (m.deleted) return `<div class="px-4 py-[3px] text-xs italic ink3" data-msg="${m.id}">Message deleted</div>`;
    const u = userById(m.sender_id);
    const avatarSlot = grouped
      ? '<div class="w-9 shrink-0"></div>'
      : `<div class="w-9 shrink-0 pt-0.5">${avatarHtml({ ...u, avatar: m.sender_avatar, initials: m.sender_initials, color: m.sender_color }, 'h-9 w-9 text-xs', false)}</div>`;
    const head = grouped || m.mine
      ? ''
      : `<div class="mb-0.5 flex items-baseline gap-2"><span class="text-[12.5px] font-semibold" style="color:${m.sender_color}">${esc(m.sender_name)}</span></div>`;
    const reply =
      m.reply_to_id > 0
        ? `<div class="mb-1 flex items-center gap-2 rounded-lg border-l-2 px-2.5 py-1 text-[11.5px]" style="border-color:${m.sender_color};background:color-mix(in srgb, ${m.sender_color} 7%, transparent)">
           <span class="font-semibold" style="color:${m.sender_color}">${esc(m.reply_to_name || '')}</span>
           <span class="truncate ink3">${esc(m.reply_to_body || '')}</span></div>`
        : '';
    const reacts = (m.reactions || []).length
      ? `<div class="mt-1 flex flex-wrap gap-1">${m.reactions
          .map(
            (r) => `<button class="chip ${r.mine ? 'brand' : ''} reaction-btn" data-message="${m.id}" data-emoji="${esc(r.emoji)}" style="${
              r.mine
                ? 'background:color-mix(in srgb, var(--color-brand-600) 14%, transparent);border-color:color-mix(in srgb, var(--color-brand-600) 32%, transparent);color:var(--color-ink)'
                : 'background:var(--color-panel2);color:var(--color-ink2)'
            }">${esc(r.emoji)} <span class="opacity-70">${r.users.length}</span></button>`
          )
          .join('')}</div>`
      : '';
    const mineActions = m.mine
      ? `<button class="btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg" data-act="edit" title="Edit">${icon('edit', 'h-3.5 w-3.5')}</button>
         <button class="btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg hover:!text-red-500" data-act="delete" title="Delete">${icon('trash', 'h-3.5 w-3.5')}</button>`
      : '';
    return `<div class="group relative bubble-row${m.mine ? ' own' : ''}" data-msg="${m.id}" data-sender="${m.sender_id}" data-grouped="${grouped ? '1' : '0'}">
      ${avatarSlot}
      <div class="flex min-w-0 flex-1 flex-col ${m.mine ? 'items-end' : 'items-start'}">${head}
        <div class="bubble">${reply}<div class="msg-body">${bodyHtml(m.body)}</div><div class="bubble-meta"><span>${timeHHMM(m.created_at)}</span>${m.edited ? '<span>(edited)</span>' : ''}</div></div>
        ${reacts}
      </div>
      <div class="pointer-events-none absolute -top-3 ${m.mine ? 'left-3' : 'right-3'} z-10 flex translate-y-1 items-center gap-0.5 rounded-xl border line panel px-1 py-0.5 opacity-0 shadow-soft transition-all group-hover:pointer-events-auto group-hover:translate-y-0 group-hover:opacity-100">
        <button class="btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg" data-act="reply" title="Reply">${icon('reply', 'h-3.5 w-3.5')}</button>
        <button class="btn btn-ghost btn-icon !h-7 !w-7 !rounded-lg" data-act="react" title="React">${icon('smile', 'h-3.5 w-3.5')}</button>
        ${mineActions}
      </div>
    </div>`;
  }

  function appendMessage(m, scroll) {
    const box = $('#messages');
    if (!box) return;
    const empty = box.querySelector('.pointer-events-none, [data-empty]');
    if (empty) empty.remove();
    const last = box.querySelector('[data-msg]:last-child');
    const grouped = last && last.dataset.sender === String(m.sender_id) && Math.abs(m.created_at - (+last.dataset.ts || 0)) < 300000;
    const div = document.createElement('div');
    div.innerHTML = msgHtml(m, grouped);
    const el = div.firstElementChild;
    el.dataset.ts = m.created_at;
    box.appendChild(el);
    if (scroll !== false) box.scrollTop = box.scrollHeight;
  }

  function replaceMessage(m) {
    const el = document.querySelector(`[data-msg="${m.id}"]`);
    if (!el) { appendMessage(m, false); return; }
    const div = document.createElement('div');
    div.innerHTML = msgHtml(m, el.dataset.grouped === '1');
    const nel = div.firstElementChild;
    nel.dataset.ts = m.created_at;
    el.replaceWith(nel);
  }

  function removeMessage(id) {
    const el = document.querySelector(`[data-msg="${id}"]`);
    if (el) el.outerHTML = `<div class="px-4 py-[3px] text-xs italic ink3" data-msg="${id}">Message deleted</div>`;
  }

  function showTyping(cid, name) {
    if (cid !== state.convId) return;
    const t = $('#typing');
    if (!t) return;
    t.textContent = `${name} is typing…`;
    clearTimeout(state.typingTimers.get(cid));
    state.typingTimers.set(cid, setTimeout(() => (t.textContent = ''), 3200));
  }

  async function loadDetails(cid) {
    try {
      const d = await api(`/api/conversations/${cid}`);
      const c = d.conversation;
      if (!c) return;
      state.convId = c.id;
      const panel = $('#sidecol');
      if (panel && c.kind === 'group') {
        panel.innerHTML =
          `<div class="flex items-center gap-2 border-b line px-4 py-3"><h2 class="flex-1 truncate text-[13px] font-bold uppercase tracking-[0.06em] ink2">Details</h2></div>` +
          `<div class="min-h-0 flex-1 overflow-y-auto p-3"><div class="flex items-center gap-3 px-1 pb-3">${avatarHtml({ id: 0, initials: c.initials, color: c.color, presence: '' }, 'h-12 w-12 text-base', false)}<div><div class="text-sm font-bold">${esc(c.title)}</div><div class="text-xs ink3">Group · ${c.members.length} members</div></div></div>` +
          c.members.map((id) => { const u = userById(id); return `<div class="navlink">${avatarHtml(u, 'h-8 w-8 text-[11px]', true)}<span class="min-w-0 flex-1 truncate">${esc(u.name)}</span><span class="text-[11px] ink3">${u.presence === 'online' ? 'online' : 'offline'}</span></div>`; }).join('') +
          `<div class="mt-3 flex flex-col gap-1 px-1"><button class="navlink" data-act="mute-conv" type="button">${icon('bell', 'h-4 w-4')}<span>Mute notifications</span><span class="ml-auto chip" data-muted="${c.muted ? '1' : '0'}">${c.muted ? 'Muted' : 'On'}</span></button>
           <button class="navlink" data-act="mark-read" type="button">${icon('check', 'h-4 w-4')}<span>Mark as read</span></button>
           <a class="navlink" href="/files?conv=${c.id}">${icon('folder', 'h-4 w-4')}<span>Shared files</span>${icon('chev-right', 'h-3.5 w-3.5 ml-auto')}</a></div></div>`;
      }
    } catch (e) {}
  }

  function initComposer() {
    const form = $('#composer');
    if (!form) return;
    const input = $('#composer-input');
    state.convId = +form.dataset.conv || 0;
    const autoGrow = () => {
      input.style.height = 'auto';
      input.style.height = Math.min(input.scrollHeight, 160) + 'px';
    };
    input.addEventListener('input', () => {
      autoGrow();
      if (state.convId) {
        clearTimeout(state._ty);
        state._ty = setTimeout(() => wsSend({ t: 'chat:typing', conversation_id: state.convId }), 500);
      }
    });
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        form.requestSubmit();
      }
    });
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const body = input.value.trim();
      if (!body) return;
      input.value = '';
      autoGrow();
      try {
        const d = await api(`/api/conversations/${state.convId}/messages`, { method: 'POST', body: { body, reply_to_id: state.replyTo } });
        state.replyTo = 0;
        appendMessage(d.message, true);
        wsSend({ t: 'chat:read', conversation_id: state.convId });
        $('#messages').scrollTop = $('#messages').scrollHeight;
      } catch (err) {
        toast(err.message, 'error');
      }
    });
    const attach = $('#attach-btn');
    if (attach) attach.addEventListener('click', () => openFilePicker('chat'));
    const emoji = $('#emoji-btn');
    if (emoji) emoji.addEventListener('click', (e) => { e.stopPropagation(); openEmojiPicker(emoji); });
    // mark read on view
    wsSend({ t: 'chat:read', conversation_id: state.convId });
  }

  function openEmojiPicker(anchor) {
    const EMOJIS = ['👍', '❤️', '😂', '😮', '😢', '🙏', '🎉', '🔥', '✅', '👀', '💯', '🤝'];
    const pop = document.createElement('div');
    pop.className = 'panel absolute bottom-12 right-0 z-50 grid grid-cols-6 gap-1 rounded-xl p-2 shadow-pop';
    pop.style.position = 'absolute';
    pop.innerHTML = EMOJIS.map((e) => `<button class="grid h-9 w-9 place-items-center rounded-lg text-lg hover:bg-[var(--color-panel2)]" data-em="${e}">${e}</button>`).join('');
    const wrap = anchor.parentElement;
    wrap.style.position = 'relative';
    wrap.appendChild(pop);
    pop.addEventListener('click', (ev) => {
      const b = ev.target.closest('[data-em]');
      if (!b) return;
      const em = b.dataset.em;
      pop.remove();
      const target = state.replyTo || lastOwnMessageId();
      if (target) {
        api(`/api/messages/${target}/reactions`, { method: 'POST', body: { emoji: em } }).catch((e) => toast(e.message, 'error'));
      }
    });
    setTimeout(() => document.addEventListener('click', () => pop.remove(), { once: true }), 10);
  }

  function lastOwnMessageId() {
    const own = $$('#messages [data-msg]').filter((n) => +n.dataset.sender === ME.id);
    return own.length ? +own[own.length - 1].dataset.msg : 0;
  }

  function initMessageScroll() {
    const box = $('#messages');
    if (!box) return;
    box.addEventListener('scroll', () => {
      if (box.scrollTop < 60 && state.convId) loadOlder();
    });
    let loading = false;
    async function loadOlder() {
      if (loading) return;
      const first = box.querySelector('[data-msg]');
      if (!first) return;
      loading = true;
      try {
        const d = await api(`/api/conversations/${state.convId}/messages?before=${first.dataset.msg}&limit=60`);
        if (d.messages && d.messages.length) {
          const prev = box.querySelector('[data-msg]');
          d.messages.reverse().forEach((m) => {
            const div = document.createElement('div');
            div.innerHTML = msgHtml(m, false);
            box.prepend(div.firstElementChild);
          });
          box.scrollTop = prev ? prev.offsetTop - 40 : 0;
        }
      } catch (e) {
      } finally {
        loading = false;
      }
    }
  }

  // Global search: filters the conversation list instantly and streams server
  // hits (messages, files, chats) into a results panel under the search box.
  function initGlobalSearch() {
    const box = $('#conv-search');
    if (!box) return;
    let panel = $('#search-panel');
    if (!panel) {
      panel = document.createElement('div');
      panel.id = 'search-panel';
      panel.className = 'hidden px-3 pb-2';
      box.closest('div').parentElement.appendChild(panel);
    }
    let timer = 0;
    box.addEventListener('input', () => {
      const q = box.value.trim();
      clearTimeout(timer);
      if (q.length < 2) {
        panel.classList.add('hidden');
        panel.innerHTML = '';
        return;
      }
      timer = setTimeout(async () => {
        try {
          const d = await api(`/api/search?q=${encodeURIComponent(q)}`);
          const rows = [];
          (d.conversations || []).slice(0, 4).forEach((c) => {
            rows.push(`<a class="navlink" href="/chat/${c.id}">${icon('chat', 'h-4 w-4')}<span class="min-w-0 flex-1 truncate">${esc(c.title)}</span><span class="text-[10.5px] ink3">chat</span></a>`);
          });
          (d.messages || []).slice(0, 5).forEach((m) => {
            rows.push(`<a class="navlink" href="/chat/${m.conversation_id}">${icon('chat', 'h-4 w-4')}<span class="min-w-0 flex-1 truncate">${esc(m.clean || m.body)}</span><span class="text-[10.5px] ink3">${esc(m.sender_name || '')}</span></a>`);
          });
          (d.files || []).slice(0, 4).forEach((f) => {
            rows.push(`<a class="navlink" href="/files?select=${f.id}">${icon('file', 'h-4 w-4')}<span class="min-w-0 flex-1 truncate">${esc(f.name)}</span><span class="text-[10.5px] ink3">file</span></a>`);
          });
          panel.innerHTML = rows.length
            ? `<div class="panel flex max-h-72 flex-col gap-0.5 overflow-y-auto rounded-xl p-1 shadow-pop">${rows.join('')}</div>`
            : `<div class="panel rounded-xl px-3 py-2 text-[12px] ink3 shadow-pop">No matches for “${esc(q)}”</div>`;
          panel.classList.remove('hidden');
        } catch (e) {
          panel.classList.add('hidden');
        }
      }, 220);
    });
    box.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        box.value = '';
        panel.classList.add('hidden');
        $$('#conv-list [data-conv]').forEach((a) => (a.style.display = ''));
      }
    });
    document.addEventListener('click', (e) => {
      if (!panel.contains(e.target) && e.target !== box) panel.classList.add('hidden');
    });
  }

  function initConvFilters() {
    const bar = $('#conv-filter');
    if (!bar) return;
    bar.addEventListener('click', (e) => {
      const btn = e.target.closest('[data-filter]');
      if (!btn) return;
      $$('#conv-filter [data-filter]').forEach((b) => {
        b.classList.toggle('chip-brand', b === btn);
      });
      const mode = btn.dataset.filter;
      $$('#conv-list [data-conv]').forEach((a) => {
        const kind = a.dataset.kind || 'group';
        const unread = a.dataset.unread === '1';
        let show = true;
        if (mode === 'people') show = kind === 'direct';
        else if (mode === 'groups') show = kind === 'group';
        else if (mode === 'unread') show = unread;
        a.style.display = show ? '' : 'none';
      });
    });
  }

  function initConvSearch() {
    const box = $('#conv-search');
    if (!box) return;
    box.addEventListener('input', () => {
      const q = box.value.toLowerCase().trim();
      $$('#conv-list [data-conv]').forEach((a) => {
        a.style.display = !q || a.textContent.toLowerCase().includes(q) ? '' : 'none';
      });
    });
  }

  /* ------------------------------------------------------------ uploads */

  /* Uploads. One hidden <input type=file> lives in the app shell and has
     exactly one change handler (initUploader), so every entry point — the
     Files page buttons, drag-and-drop, and the chat attach button — shares
     the same path. `pendingUploadTarget` remembers which context opened it. */
  let pendingUploadTarget = null;

  function openFilePicker(target) {
    const fi = $('#file-input');
    if (!fi) {
      toast('Upload is unavailable on this page', 'error');
      return;
    }
    pendingUploadTarget = target || (state.section === 'chat' ? 'chat' : 'files');
    fi.click();
  }

  function initUploader() {
    const fi = $('#file-input');
    if (!fi || fi.dataset.bound) return;
    fi.dataset.bound = '1';
    fi.addEventListener('change', () => {
      const target = pendingUploadTarget || (state.section === 'chat' ? 'chat' : 'files');
      pendingUploadTarget = null;
      if (fi.files.length) uploadFiles(fi.files, null, target);
      fi.value = '';
    });
  }

  function humanSize(n) {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1048576).toFixed(1)} MB`;
    return `${(n / 1073741824).toFixed(1)} GB`;
  }

  async function uploadFiles(files, parentId, target) {
    const mode = target || (state.section === 'chat' && state.convId ? 'chat' : 'files');
    const parent = parentId == null ? currentFolderId() : parentId;
    let done = 0;
    for (const file of files) {
      const name = file.name || 'file';
      try {
        if (file.size === 0) throw new Error('file is empty');
        // Say it started: a large file otherwise looks like nothing happened.
        toast(`Uploading ${name} (${humanSize(file.size)})…`, 'info');
        const d = await api(
          `/api/files/upload?parent=${parent}&name=${encodeURIComponent(name)}`,
          {
            method: 'POST',
            headers: { 'Content-Type': file.type || 'application/octet-stream' },
            body: await file.arrayBuffer(),
          }
        );
        // Confirm the server stored the bytes we sent, not a JSON stub.
        const saved = d && d.file ? d.file.size : -1;
        if (saved !== file.size) {
          throw new Error(`stored ${humanSize(saved < 0 ? 0 : saved)} of ${humanSize(file.size)}`);
        }
        done += 1;
        toast(`${name} uploaded`, 'ok');
        if (mode === 'chat' && state.convId) {
          // Attachments in a conversation also post a message so the other
          // side sees them, instead of landing silently in Drive.
          const link = `${location.origin}/api/files/${d.file.id}/download`;
          const body = `📎 ${name} (${humanSize(file.size)})\n${link}`;
          const m = await api(`/api/conversations/${state.convId}/messages`, {
            method: 'POST',
            body: { body },
          });
          appendMessage(m.message, true);
        }
      } catch (e) {
        toast(`${name}: ${e.message}`, 'error');
      }
    }
    if (done && mode !== 'chat' && state.section === 'files') {
      setTimeout(() => location.reload(), 250);
    }
  }

  function currentFolderId() {
    const params = new URLSearchParams(location.search);
    return +params.get('parent') || 0;
  }

  async function reloadFiles() {
    if (state.section !== 'files') return;
    location.reload();
  }

  function initDragDrop() {
    const main = $('#maincol');
    if (!main) return;
    let depth = 0;
    main.addEventListener('dragenter', (e) => { e.preventDefault(); depth++; main.classList.add('ring-2'); });
    main.addEventListener('dragleave', () => { depth--; if (depth <= 0) { depth = 0; main.classList.remove('ring-2'); } });
    main.addEventListener('dragover', (e) => e.preventDefault());
    main.addEventListener('drop', (e) => {
      e.preventDefault();
      depth = 0;
      main.classList.remove('ring-2');
      if (e.dataTransfer.files && e.dataTransfer.files.length) uploadFiles(e.dataTransfer.files);
    });
  }

  /* ------------------------------------------------------------ files ui */

  function initFiles() {
    if (state.section !== 'files') return;
    $$('#file-list [data-file], #file-grid [data-file]').forEach((row) => {
      row.addEventListener('click', (e) => {
        if (e.target.closest('[data-act]')) return;
        const id = row.dataset.file;
        if (row.dataset.kind === 'folder') {
          location.href = `/files?parent=${id}`;
        } else {
          location.href = `/files?select=${id}`;
        }
      });
      row.addEventListener('dblclick', () => {});
    });
    $$('[data-act="file-menu"]').forEach((b) => b.addEventListener('click', (e) => {
      e.stopPropagation();
      const id = b.closest('[data-file]').dataset.file;
      openFileMenu(b, id);
    }));
    const toggle = $('[data-act="view-toggle"]');
    if (toggle) {
      toggle.addEventListener('click', () => {
        state.fileView = state.fileView === 'grid' ? 'list' : 'grid';
        location.href = '/files' + (currentFolderId() ? `?parent=${currentFolderId()}` : '');
      });
    }
  }

  function openFileMenu(anchor, id) {
    const row = anchor.closest('[data-file]');
    const kind = row.dataset.kind;
    const name = row.querySelector('.truncate')?.textContent || 'item';
    const items = [
      { act: 'download', label: 'Download', ic: 'download' },
      { act: 'rename', label: 'Rename', ic: 'edit' },
      { act: 'move', label: 'Move to…', ic: 'folder' },
      kind === 'folder' ? { act: 'share', label: 'Share…', ic: 'share' } : null,
      { act: 'delete', label: 'Move to trash', ic: 'trash', danger: true },
    ].filter(Boolean);
    const pop = document.createElement('div');
    pop.className = 'panel fixed z-[70] w-44 rounded-xl p-1 shadow-pop';
    pop.innerHTML = items.map((i) => `<button class="navlink ${i.danger ? 'hover:!text-red-500' : ''}" data-menu="${i.act}">${icon(i.ic, 'h-4 w-4')}<span>${i.label}</span></button>`).join('');
    const r = anchor.getBoundingClientRect();
    pop.style.top = Math.min(r.bottom + 4, window.innerHeight - 240) + 'px';
    pop.style.left = Math.max(8, r.left - 120) + 'px';
    document.body.appendChild(pop);
    const close = () => { pop.remove(); document.removeEventListener('click', close); };
    setTimeout(() => document.addEventListener('click', close), 10);
    pop.addEventListener('click', async (e) => {
      const b = e.target.closest('[data-menu]');
      if (!b) return;
      close();
      const act = b.dataset.menu;
      if (act === 'download') {
        location.href = `/api/files/${id}/download`;
      } else if (act === 'rename') {
        promptDialog('Rename', name, 'name', 'Rename', async (v) => {
          await api(`/api/files/${id}`, { method: 'PATCH', body: { name: v } });
          location.reload();
        });
      } else if (act === 'delete') {
        confirmDialog('Move to trash', `Move <b>${esc(name)}</b> to the trash? You can restore it later.`, 'Move to trash', async () => {
          await api(`/api/files/${id}`, { method: 'DELETE', body: { on: true } });
          location.reload();
        }, true);
      } else if (act === 'move') {
        const d = await api('/api/files?parent=0');
        const folders = (d.items || []).filter((f) => f.kind === 'folder');
        modal('Move to folder', `<div class="flex flex-col gap-1">${folders.map((f) => `<button class="navlink" data-move="${f.id}">${icon('folder', 'h-4 w-4')}<span>${esc(f.name)}</span></button>`).join('') || '<div class="text-[12.5px] ink3">No folders yet.</div>'}</div>`, '');
        $$('#modal-root [data-move]').forEach((b) => b.addEventListener('click', async () => {
          await api(`/api/files/${id}`, { method: 'PATCH', body: { parent: +b.dataset.move } });
          closeModal();
          location.reload();
        }));
      } else if (act === 'share') {
        modal('Share folder', '<div class="text-[12.5px] ink3">Sharing is enforced locally by the host.</div>', '');
      }
    });
  }

  function promptDialog(title, current, field, action, fn) {
    const el = modal(title, `<label class="block"><span class="mb-1.5 block text-xs font-semibold ink2">${esc(field)}</span><input class="input" id="pd-input" value="${esc(current)}" maxlength="120"></label>`,
      `<button class="btn btn-ghost" data-close type="button">Cancel</button><button class="btn btn-brand" data-ok type="button">${esc(action)}</button>`);
    el.querySelector('[data-ok]').addEventListener('click', async () => {
      const v = el.querySelector('#pd-input').value;
      if (!v.trim()) return;
      await fn(v.trim());
      closeModal();
    });
  }

  /* ------------------------------------------------------------ calls */

  function initCalls() {
    if (state.section !== 'calls') return;
    $$('[data-call-conv]').forEach((b) => b.addEventListener('click', async () => {
      const kind = document.querySelector('.call-kind.btn-brand')?.dataset.kind || 'video';
      try {
        const d = await api('/api/calls', { method: 'POST', body: { conversation_id: +b.dataset.callConv, kind } });
        if (d.active) { toast('A call is already in progress', 'info'); return; }
        startCallUi(d.call_id, kind, true);
      } catch (e) { toast(e.message, 'error'); }
    }));
    $$('.call-kind').forEach((b) => b.addEventListener('click', () => {
      $$('.call-kind').forEach((x) => { x.classList.remove('btn-brand'); x.classList.add('btn-soft'); });
      b.classList.add('btn-brand'); b.classList.remove('btn-soft');
    }));
    const join = $('#join-call');
    if (join) join.addEventListener('click', async () => {
      const cid = +($('#call-conv')?.value || 0);
      if (!cid) { toast('Pick a conversation first', 'error'); return; }
      const kind = document.querySelector('.call-kind.btn-brand')?.dataset.kind || 'video';
      try {
        const d = await api('/api/calls', { method: 'POST', body: { conversation_id: cid, kind } });
        startCallUi(d.call_id, kind, true);
      } catch (e) { toast(e.message, 'error'); }
    });
    const leave = $('#ctl-leave');
    if (leave) leave.addEventListener('click', () => leaveCall());
    const mic = $('#ctl-mic');
    if (mic) mic.addEventListener('click', () => { state.micOn = !state.micOn; applyTracks(); paintCallControls(); });
    const cam = $('#ctl-cam');
    if (cam) cam.addEventListener('click', () => { state.camOn = !state.camOn; applyTracks(); paintCallControls(); });
    const share = $('#ctl-share');
    if (share) share.addEventListener('click', toggleScreenShare);
    const stage = $('#call-stage');
    if (stage) {
      state.call = { id: +stage.dataset.call || 0, kind: 'video' };
      const t = $('#call-timer');
      setInterval(() => {
        if (!t) return;
        const start = +t.dataset.start || Date.now();
        t.textContent = clock(Date.now() - start);
      }, 500);
    }
  }

  function paintCallControls() {
    const mic = $('#ctl-mic'), cam = $('#ctl-cam');
    if (mic) mic.innerHTML = icon(state.micOn ? 'mic' : 'mic-off', 'h-5 w-5');
    if (cam) cam.innerHTML = icon(state.camOn ? 'video' : 'video-off', 'h-5 w-5');
    mic?.classList.toggle('!text-red-500', !state.micOn);
    cam?.classList.toggle('!text-red-500', !state.camOn);
  }

  function applyTracks() {
    if (state.localStream) {
      state.localStream.getAudioTracks().forEach((t) => (t.enabled = state.micOn));
      state.localStream.getVideoTracks().forEach((t) => (t.enabled = state.camOn));
    }
    Object.values(state.pc).forEach((pc) => {
      pc.getSenders().forEach((s) => {
        if (s.track && (s.track.kind === 'audio' || s.track.kind === 'video')) s.track.enabled = s.track.kind === 'audio' ? state.micOn : state.camOn;
      });
    });
  }

  async function startCallUi(callId, kind, isStarter) {
    try {
      const d = await api(`/api/calls/${callId}/join`);
      state.call = { id: callId, kind };
      const tiles = (d.call.participants || []).map((uid) => {
        const u = userById(uid);
        return { user_id: uid, name: u.name, color: u.color, initials: u.initials, speaking: false, video: kind === 'video', muted: false, local: uid === ME.id };
      });
      const stage = `
        <div class="flex min-h-0 flex-1 flex-col" style="background:var(--color-panel2)">
          <div class="flex items-center justify-between px-4 py-2.5">
            <div class="flex items-center gap-2 text-[12px] ink2"><span class="relative flex h-2 w-2"><span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75"></span><span class="relative inline-flex h-2 w-2 rounded-full bg-green-500"></span></span><span class="font-semibold">Live</span>·<span id="call-timer" data-start="${Date.now()}">${clock(0)}</span></div>
            <div class="text-[11px] ink3">${tiles.length} participants</div>
          </div>
          <div id="call-stage" data-call="${callId}" class="grid grid-cols-2 min-h-0 flex-1 gap-2 overflow-y-auto p-2">${tiles.map(tileHtml).join('')}</div>
        </div>
        <div class="flex items-center justify-center gap-2 border-t line px-3 py-3">
          <button id="ctl-mic" class="btn btn-soft btn-icon !h-11 !w-11 !rounded-full">${icon('mic', 'h-5 w-5')}</button>
          <button id="ctl-cam" class="btn btn-soft btn-icon !h-11 !w-11 !rounded-full">${icon('video', 'h-5 w-5')}</button>
          <button id="ctl-share" class="btn btn-soft btn-icon !h-11 !w-11 !rounded-full">${icon('share', 'h-5 w-5')}</button>
          <button id="ctl-leave" class="btn btn-icon !h-11 !w-11 !rounded-full" style="background:#ef4444;color:#fff">${icon('phone-off', 'h-5 w-5')}</button>
        </div>`;
      $('#maincol').innerHTML = stage;
      bindCallControls();
      await setupMedia();
      connectToPeers(d.call.participants || [], isStarter);
      toast('Joined call', 'ok');
    } catch (e) {
      toast(e.message, 'error');
    }
  }

  function tileHtml(t) {
    const video = t.video ? `<video id="cam-${t.user_id}" class="h-full w-full rounded-2xl object-cover" autoplay playsinline ${t.local ? 'muted' : ''}></video>` : '';
    const badge = t.video
      ? ''
      : `<div class="absolute inset-0 grid place-items-center rounded-2xl" style="background:var(--color-panel3)">${avatarHtml({ id: t.user_id, name: t.name, initials: t.initials, color: t.color, presence: '' }, 'h-16 w-16 text-lg', false)}</div>`;
    return `<div class="relative min-h-[180px] overflow-hidden rounded-2xl border line" style="background:var(--color-panel)">
      ${video}${badge}
      <div class="absolute bottom-2 left-2 flex items-center gap-1.5 rounded-full px-2 py-1 text-[11px] font-semibold text-white" style="background:rgb(0 0 0 / .5)"><span>${esc(t.name)}</span></div>
    </div>`;
  }

  function bindCallControls() {
    $('#ctl-mic')?.addEventListener('click', () => { state.micOn = !state.micOn; applyTracks(); paintCallControls(); });
    $('#ctl-cam')?.addEventListener('click', () => { state.camOn = !state.camOn; applyTracks(); paintCallControls(); });
    $('#ctl-share')?.addEventListener('click', toggleScreenShare);
    $('#ctl-leave')?.addEventListener('click', leaveCall);
  }

  async function setupMedia() {
    try {
      state.localStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: state.call?.kind === 'video' });
      const lv = $('#cam-' + ME.id);
      if (lv) lv.srcObject = state.localStream;
    } catch (e) {
      toast('Camera/microphone unavailable: ' + e.message, 'error');
    }
  }

  const RTC_CONFIG = { iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] };

  function ensurePc(uid) {
    if (state.pc.has(uid)) return state.pc.get(uid);
    const pc = new RTCPeerConnection(RTC_CONFIG);
    state.pc.set(uid, pc);
    if (state.localStream) state.localStream.getTracks().forEach((t) => pc.addTrack(t, state.localStream));
    pc.onicecandidate = (e) => {
      if (e.candidate) wsSend({ t: 'call:signal', call_id: state.call.id, to: uid, data: { kind: 'ice', candidate: e.candidate } });
    };
    pc.ontrack = (e) => {
      const v = $('#cam-' + uid);
      if (v) v.srcObject = e.streams[0];
    };
    pc.onconnectionstatechange = () => {
      const tile = $('#cam-' + uid)?.closest('.relative');
      if (tile) tile.style.boxShadow = pc.connectionState === 'connected' ? '0 0 0 2px #10b981' : '';
    };
    return pc;
  }

  async function connectToPeers(participants, isStarter) {
    for (const uid of participants) {
      if (uid === ME.id) continue;
      const pc = ensurePc(uid);
      try {
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);
        wsSend({ t: 'call:signal', call_id: state.call.id, to: uid, data: { kind: 'offer', sdp: pc.localDescription } });
      } catch (e) {}
    }
  }

  async function handleSignal(msg) {
    if (!state.call) return;
    const pc = ensurePc(msg.from);
    const d = msg.data || {};
    try {
      if (d.kind === 'offer') {
        await pc.setRemoteDescription(new RTCSessionDescription(d.sdp));
        const answer = await pc.createAnswer();
        await pc.setLocalDescription(answer);
        wsSend({ t: 'call:signal', call_id: state.call.id, to: msg.from, data: { kind: 'answer', sdp: pc.localDescription } });
      } else if (d.kind === 'answer') {
        await pc.setRemoteDescription(new RTCSessionDescription(d.sdp));
      } else if (d.kind === 'ice') {
        await pc.addIceCandidate(new RTCIceCandidate(d.candidate));
      }
    } catch (e) {}
  }

  function callPeerEvent(msg) {
    if (msg.event === 'join') {
      connectToPeers([msg.user_id], true);
    }
  }

  async function toggleScreenShare() {
    try {
      if (state.screenStream) {
        state.screenStream.getTracks().forEach((t) => t.stop());
        state.screenStream = null;
        if (state.localStream) {
          const v = $('#cam-' + ME.id);
          if (v) v.srcObject = state.localStream;
        }
        toast('Screen sharing stopped', 'info');
        return;
      }
      state.screenStream = await navigator.mediaDevices.getDisplayMedia({ video: true });
      const v = $('#cam-' + ME.id);
      if (v) v.srcObject = state.screenStream;
      state.screenStream.getVideoTracks()[0].onended = () => {
        state.screenStream = null;
        if (state.localStream && v) v.srcObject = state.localStream;
      };
      toast('Screen sharing started', 'ok');
    } catch (e) {
      toast('Screen sharing failed: ' + e.message, 'error');
    }
  }

  async function leaveCall() {
    if (!state.call) { location.href = '/calls'; return; }
    const id = state.call.id;
    try {
      await api(`/api/calls/${id}/leave`, { method: 'POST' });
    } catch (e) {}
    wsSend({ t: 'call:leave', call_id: id });
    teardownCall();
    location.href = '/calls';
  }

  function endCallUi(reason) {
    toast(reason, 'info');
    teardownCall();
    setTimeout(() => location.reload(), 900);
  }

  function teardownCall() {
    Object.values(state.pc).forEach((pc) => pc.close());
    state.pc.clear();
    state.localStream?.getTracks().forEach((t) => t.stop());
    state.screenStream?.getTracks().forEach((t) => t.stop());
    state.localStream = null;
    state.screenStream = null;
    state.call = null;
  }

  function showIncomingCall(msg) {
    if (document.querySelector('[data-incoming]')) return;
    const el = document.createElement('div');
    el.className = 'toast pointer-events-auto fixed left-1/2 top-4 z-[95] -translate-x-1/2';
    el.setAttribute('data-incoming', '');
    el.innerHTML = `<div class="flex items-center gap-3 rounded-2xl border line panel px-4 py-3 shadow-pop">
      <span class="grid h-9 w-9 place-items-center rounded-full" style="background:color-mix(in srgb,#10b981 15%,transparent);color:#059669">${icon('phone', 'h-4 w-4')}</span>
      <div class="text-[13px]"><div class="font-bold">${esc(msg.from || 'Someone')} is calling</div><div class="text-[11px] ink3">${msg.kind === 'audio' ? 'Audio call' : 'Video call'}</div></div>
      <button class="btn btn-brand btn-xs" data-answer>Join</button>
      <button class="btn btn-ghost btn-xs" data-dismiss>Dismiss</button></div>`;
    document.body.appendChild(el);
    el.querySelector('[data-answer]').addEventListener('click', async () => {
      el.remove();
      try {
        const d = await api(`/api/calls/${msg.call_id}/join`);
        state.call = { id: msg.call_id, kind: msg.kind };
        history.pushState({}, '', '/calls');
        await startCallUi(msg.call_id, msg.kind, false);
      } catch (e) { toast(e.message, 'error'); }
    });
    el.querySelector('[data-dismiss]').addEventListener('click', () => el.remove());
    setTimeout(() => el.remove(), 20000);
  }

  /* ------------------------------------------------------------ admin */

  function reloadAdmin() {
    if (state.section !== 'admin') return;
    const tab = new URLSearchParams(location.search).get('tab') || 'overview';
    if (tab === 'requests' || tab === 'devices' || tab === 'overview') location.reload();
    else updatePendingBadge();
  }

  function initAdmin() {
    if (state.section !== 'admin') return;
    const openToggle = $('#open-toggle');
    if (openToggle) openToggle.addEventListener('click', async () => {
      const next = openToggle.dataset.open === '1';
      busy(openToggle, true);
      try {
        await api('/api/admin/config', { method: 'POST', body: { open: next ? '1' : '0' } });
        toast(next ? 'Server is open to the network' : 'Server now requires approval', 'ok');
        setTimeout(() => location.reload(), 600);
      } catch (e) { toast(e.message, 'error'); busy(openToggle, false); }
    });
    $$('[data-approve]').forEach((b) => b.addEventListener('click', async () => {
      busy(b, true);
      try { await api(`/api/admin/requests/${b.dataset.approve}/approve`, { method: 'POST' }); location.reload(); }
      catch (e) { toast(e.message, 'error'); busy(b, false); }
    }));
    $$('[data-reject]').forEach((b) => b.addEventListener('click', async () => {
      busy(b, true);
      try { await api(`/api/admin/requests/${b.dataset.reject}/reject`, { method: 'POST' }); location.reload(); }
      catch (e) { toast(e.message, 'error'); busy(b, false); }
    }));
    $$('[data-revoke]').forEach((b) => b.addEventListener('click', async () => {
      confirmDialog('Revoke device', 'Revoke this device credential? It will stop working immediately.', 'Revoke', async () => {
        await api(`/api/admin/devices/${b.dataset.revoke}/revoke`, { method: 'POST' });
        location.reload();
      }, true);
    }));
    $$('[data-user-suspend]').forEach((b) => b.addEventListener('click', async () => {
      await api(`/api/admin/users/${b.dataset.userSuspend}`, { method: 'PATCH', body: { state: 'suspended' } });
      location.reload();
    }));
    $$('[data-user-unsuspend]').forEach((b) => b.addEventListener('click', async () => {
      await api(`/api/admin/users/${b.dataset.userUnsuspend}`, { method: 'PATCH', body: { state: 'active' } });
      location.reload();
    }));
    $$('[data-user-role]').forEach((sel) => sel.addEventListener('change', async () => {
      await api(`/api/admin/users/${sel.dataset.userRole}`, { method: 'PATCH', body: { role: sel.value } });
      toast('Role updated', 'ok');
      setTimeout(() => location.reload(), 700);
    }));
    $$('[data-act="backup-now"]').forEach((b) => b.addEventListener('click', async () => {
      busy(b, true);
      try { const d = await api('/api/admin/backups', { method: 'POST' }); toast(`Backup created (${bytes(d.size)})`, 'ok'); location.reload(); }
      catch (e) { toast(e.message, 'error'); busy(b, false); }
    }));
    $$('[data-backup-delete]').forEach((b) => b.addEventListener('click', async () => {
      await api(`/api/admin/backups/${b.dataset.backupDelete}`, { method: 'DELETE' });
      location.reload();
    }));
    $$('[data-act="restart"]').forEach((b) => b.addEventListener('click', () => {
      confirmDialog('Restart server', 'Restart the ROOM server? Connected devices reconnect automatically.', 'Restart', async () => {
        await api('/api/admin/restart', { method: 'POST' });
      });
    }));
    $$('[data-act="shutdown"]').forEach((b) => b.addEventListener('click', () => {
      confirmDialog('Shut down', 'Stop the ROOM server? You will need to start it again from the host machine.', 'Shut down', async () => {
        await api('/api/admin/shutdown', { method: 'POST' });
      }, true);
    }));
    const form = $('#config-form');
    if (form) form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      try {
        await api('/api/admin/config', { method: 'POST', body: {
          server_name: fd.get('server_name'), network_name: fd.get('network_name'),
          max_upload_mb: fd.get('max_upload_mb'), theme: fd.get('theme'),
          open: fd.get('open') ?? undefined,
        }});
        toast('Configuration saved', 'ok');
        setTimeout(() => location.reload(), 700);
      } catch (err) { toast(err.message, 'error'); }
    });
    $$('[data-tab]').forEach((a) => a.addEventListener('click', (e) => {
      // allow normal navigation; nothing extra
    }));
  }

  /* ------------------------------------------------------------ account */

  function initAccount() {
    if (state.section !== 'account') return;
    const form = $('#profile-form');
    if (form) form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      try {
        const d = await api('/api/users/me', { method: 'PATCH', body: { name: fd.get('name'), bio: fd.get('bio') } });
        ME.name = d.me.name; ME.bio = d.me.bio; ME.initials = d.me.initials;
        toast('Profile saved', 'ok');
        setTimeout(() => location.reload(), 700);
      } catch (err) { toast(err.message, 'error'); }
    });
  }

  /* ------------------------------------------------------------ shell */

  function initShell() {
    applyTheme(SETTINGS.theme || 'light');
    const tb = $('#theme-btn');
    if (tb) tb.addEventListener('click', () => setTheme(currentTheme() === 'dark' ? 'light' : 'dark'));
    const back = $('#back-btn');
    if (back) back.addEventListener('click', () => {
      // full page loads land on the pane; fall back to the list route on phones
      if (document.body.dataset.view === 'pane' && history.length <= 1) {
        location.href = state.section === 'files' ? '/files' : '/';
        return;
      }
      if (document.body.dataset.view === 'pane') {
        history.back();
        setTimeout(() => {
          if (document.body.dataset.view === 'pane' && !document.hidden) {
            location.href = state.section === 'files' ? '/files' : '/';
          }
        }, 450);
        return;
      }
      if (history.length > 1) history.back();
      else location.href = '/';
    });
    $$('#rail-nav [data-nav], #mobnav [data-nav]').forEach((a) => {
      if (a.dataset.nav === state.section) a.classList.add('active');
    });
    document.body.classList.add('ws-off');
    connectWs();
    // mobile: default to the list view
    const isMobile = () => window.matchMedia('(max-width: 767px)').matches;
    const syncMobile = () => {
      const list = $('#listcol'), main = $('#maincol');
      if (!list || !main) return;
      if (isMobile()) {
        if (state.mobileView === 'main') { list.classList.add('hidden'); main.classList.remove('hidden'); }
        else { list.classList.remove('hidden'); main.classList.add('hidden'); }
      } else {
        list.classList.remove('hidden');
        main.classList.remove('hidden');
      }
    };
    state.mobileView = 'list';
    $$('a[href^="/chat/"]').forEach((a) => a.addEventListener('click', () => { state.mobileView = 'main'; syncMobile(); }));
    const backBtn = $('#back-btn');
    if (backBtn) backBtn.addEventListener('click', () => { state.mobileView = 'list'; setTimeout(syncMobile, 50); });
    window.addEventListener('resize', syncMobile);
    syncMobile();
    // idle presence
    let idleTimer = null;
    const active = () => {
      wsSend({ t: 'presence:active' });
      clearTimeout(idleTimer);
      idleTimer = setTimeout(() => wsSend({ t: 'presence:idle' }), 5 * 60 * 1000);
    };
    ['pointerdown', 'keydown', 'wheel'].forEach((ev) => document.addEventListener(ev, active, { passive: true }));
    active();
  }

  /* ------------------------------------------------------------ global actions */

  function initGlobalActions() {
    document.addEventListener('click', async (e) => {
      const newChat = e.target.closest('[data-new-chat]');
      if (newChat) { openNewChat(); return; }

      const act = e.target.closest('[data-act]');
      if (!act) return;
      const name = act.dataset.act;

      if (name === 'upload') { openFilePicker('files'); return; }
      if (name === 'new-folder') {
        promptDialog('New folder', '', 'Folder name', 'Create', async (v) => {
          await api('/api/files', { method: 'POST', body: { parent: currentFolderId(), name: v } });
          location.reload();
        });
        return;
      }
      if (name === 'mark-read' && state.convId) {
        await api(`/api/conversations/${state.convId}/read`, { method: 'POST' });
        toast('Marked as read', 'ok');
        return;
      }
      if (name === 'mute-conv' && state.convId) {
        const muted = act.querySelector('[data-muted]')?.dataset.muted === '1';
        await api(`/api/conversations/${state.convId}/mute`, { method: 'POST', body: { muted: !muted } });
        location.reload();
        return;
      }
      if (name === 'add-member') { openNewChat(true); return; }
      if (name === 'close-panel') {
        const p = $('#sidecol');
        if (p) p.classList.add('hidden');
        return;
      }
      if (name === 'reply') {
        const id = +act.closest('[data-msg]')?.dataset.msg || 0;
        if (!id) return;
        state.replyTo = id;
        toast('Replying — type your message', 'info');
        $('#composer-input')?.focus();
        return;
      }
      if (name === 'react') {
        const id = +act.closest('[data-msg]')?.dataset.msg || 0;
        openReactionPicker(act, id);
        return;
      }
      if (name === 'edit') {
        const id = +act.closest('[data-msg]')?.dataset.msg || 0;
        const el = act.closest('[data-msg]');
        const current = el ? el.querySelector('.msg-body')?.innerText || '' : '';
        promptDialog('Edit message', current, 'Message', 'Save', async (v) => {
          await api(`/api/messages/${id}`, { method: 'PATCH', body: { body: v } });
        });
        return;
      }
      if (name === 'delete') {
        const id = +act.closest('[data-msg]')?.dataset.msg || 0;
        confirmDialog('Delete message', 'Delete this message for everyone?', 'Delete', async () => {
          await api(`/api/messages/${id}`, { method: 'DELETE' });
        }, true);
        return;
      }
      if (name === 'new-call') { location.href = '/calls'; return; }
    });

    document.addEventListener('click', async (e) => {
      const r = e.target.closest('.reaction-btn');
      if (!r) return;
      const on = !r.classList.contains('brand');
      await api(`/api/messages/${r.dataset.message}/reactions`, { method: 'POST', body: { emoji: r.dataset.emoji, on } }).catch((x) => toast(x.message, 'error'));
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') closeModal();
    });
  }

  function openReactionPicker(anchor, id) {
    const EMOJIS = ['👍', '❤️', '😂', '😮', '😢', '🙏', '🎉', '🔥'];
    const pop = document.createElement('div');
    pop.className = 'panel fixed z-[80] flex gap-1 rounded-xl p-1.5 shadow-pop';
    pop.innerHTML = EMOJIS.map((em) => `<button class="grid h-8 w-8 place-items-center rounded-lg text-base hover:bg-[var(--color-panel2)]" data-em="${em}">${em}</button>`).join('');
    const r = anchor.getBoundingClientRect();
    pop.style.top = Math.min(r.bottom + 6, window.innerHeight - 60) + 'px';
    pop.style.left = Math.max(8, r.left) + 'px';
    document.body.appendChild(pop);
    const close = () => { pop.remove(); document.removeEventListener('click', close); };
    setTimeout(() => document.addEventListener('click', close), 10);
    pop.addEventListener('click', (e) => {
      const b = e.target.closest('[data-em]');
      if (!b) return;
      close();
      api(`/api/messages/${id}/reactions`, { method: 'POST', body: { emoji: b.dataset.em } }).catch((x) => toast(x.message, 'error'));
    });
  }

  function openNewChat(group) {
    const users = Array.from(state.users.values()).filter((u) => u.id !== ME.id);
    const el = modal(
      group ? 'New group' : 'New conversation',
      `<div class="space-y-3">
        ${group ? `<label class="block"><span class="mb-1.5 block text-xs font-semibold ink2">Group name</span><input class="input" id="nc-name" maxlength="60" placeholder="e.g. Design team"></label>` : ''}
        <div class="max-h-64 overflow-y-auto">
          ${users.map((u) => `<label class="navlink" style="cursor:pointer"><input type="${group ? 'checkbox' : 'radio'}" name="nc-pick" class="nc-check" value="${u.id}" style="accent-color:var(--color-brand-600)">${avatarHtml(u, 'h-8 w-8 text-[11px]', true)}<span class="min-w-0 flex-1 truncate">${esc(u.name)}</span><span class="text-[11px] ink3">${u.presence === 'online' ? 'online' : 'offline'}</span></label>`).join('') || '<div class="text-[12.5px] ink3">No other members yet.</div>'}
        </div>
      </div>`,
      `<button class="btn btn-ghost" data-close type="button">Cancel</button><button class="btn btn-brand" id="nc-go" type="button">Create</button>`
    );
    el.querySelector('#nc-go').addEventListener('click', async () => {
      const checks = $$('.nc-check', el);
      const checked = checks.filter((c) => c.checked).map((c) => +c.value);
      try {
        let d;
        if (group) {
          const name = el.querySelector('#nc-name').value.trim();
          if (!name) { toast('Group name is required', 'error'); return; }
          if (!checked.length) { toast('Pick at least one member', 'error'); return; }
          d = await api('/api/conversations', { method: 'POST', body: { kind: 'group', name, members: checked } });
        } else {
          const target = checked[0] || 0;
          if (!target) { toast('Pick a member', 'error'); return; }
          d = await api('/api/conversations', { method: 'POST', body: { kind: 'direct', user_id: target } });
        }
        closeModal();
        location.href = '/chat/' + d.conversation.id;
      } catch (e2) { toast(e2.message, 'error'); }
    });
  }

  /* ------------------------------------------------------------ boot */

  function boot() {
    initShell();
    initGlobalActions();
    initUploader();
    if (state.section === 'chat') {
      const m = location.pathname.match(/^\/chat\/(\d+)/);
      if (m) state.convId = +m[1];
      initComposer();
      initMessageScroll();
      initGlobalSearch();
    initConvFilters();
    initConvSearch();
      updatePendingBadge();
    } else if (state.section === 'files') {
      initFiles();
      initDragDrop();

    } else if (state.section === 'calls') {
      initCalls();
    } else if (state.section === 'admin') {
      initAdmin();
      updatePendingBadge();
    } else if (state.section === 'account') {
      initAccount();
    }
    // refresh relative timestamps
    setInterval(() => {
      $$('[data-rel]').forEach(() => {});
    }, 60000);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();
})();
