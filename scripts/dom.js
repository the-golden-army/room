// ROOM client DOM suite — runs the real app.js against a live server inside jsdom.
//
//   npm i --no-save jsdom
//   node scripts/dom.js [base_url]
//
// With no argument the suite starts its own binary from dist/room on a throwaway
// data directory, so every run begins from a virgin database (the first device
// to register becomes the owner). Passing a URL reuses an existing server, which
// must also have a fresh ROOM_DATA_DIR.
//
// It prepares two approved devices plus a conversation over the HTTP API, boots
// every page with the real client script, and exercises the interactive paths a
// pure HTTP smoke test cannot reach (boot payload, bubbles, pickers, composer).
const { JSDOM, VirtualConsole } = require('jsdom');
const fs = require('fs');
const path = require('path');
const http = require('http');

const ROOT = path.resolve(__dirname, '..');
const EXTERNAL = process.argv[2];
const PORT = 8123;
let BASE = (EXTERNAL || `http://127.0.0.1:${PORT}`).replace(/\/$/, '');
let child = null;
const results = [];
const errors = [];

function check(label, ok, detail) {
  results.push([label, ok ? 'ok' : 'FAIL', detail === undefined ? '' : String(detail)]);
  return ok;
}

function request(method, p, body, cookie, contentType) {
  return new Promise((resolve, reject) => {
    const url = new URL(BASE + p);
    const headers = {};
    let payload;
    if (body !== undefined && body !== null) {
      payload = Buffer.from(typeof body === 'string' ? body : JSON.stringify(body));
      headers['Content-Type'] = contentType
        || (typeof body === 'string' ? 'application/x-www-form-urlencoded' : 'application/json');
      headers['Content-Length'] = payload.length;
    }
    if (cookie) headers.Cookie = cookie;
    const req = http.request({ hostname: url.hostname, port: url.port, path: url.pathname + url.search, method, headers }, (res) => {
      const chunks = [];
      res.on('data', (c) => chunks.push(c));
      res.on('end', () => {
        const text = Buffer.concat(chunks).toString();
        let json = null;
        try { json = JSON.parse(text); } catch (e) { /* html */ }
        resolve({ status: res.statusCode, text, json, headers: res.headers });
      });
    });
    req.on('error', reject);
    if (payload) req.write(payload);
    req.end();
  });
}

function cookieOf(res) {
  const raw = res.headers['set-cookie'] || [];
  for (const line of raw) {
    const m = /^room_device=([^;]+)/.exec(line);
    if (m) return `room_device=${m[1]}`;
  }
  return '';
}

async function prepare() {
  const alice = await request('POST', '/api/request-access', 'name=DomAlice&device=dom-a');
  const aCookie = cookieOf(alice);
  const bob = await request('POST', '/api/request-access', 'name=DomBob&device=dom-b');
  const bCookie = cookieOf(bob);
  const pending = await request('GET', '/api/admin/requests', null, aCookie);
  if (!pending.json || !pending.json.pending) {
    throw new Error(`no pending approvals (status ${pending.status}): ${pending.text.slice(0, 160)}`);
  }
  const rid = pending.json.pending[0].id;
  await request('POST', `/api/admin/requests/${rid}/approve`, null, aCookie);
  const users = await request('GET', '/api/users', null, aCookie);
  const ids = Object.fromEntries(users.json.users.map((u) => [u.name, u.id]));
  const conv = await request('POST', '/api/conversations',
    { kind: 'group', name: 'DomGroup', members: [ids.DomAlice, ids.DomBob] }, aCookie);
  const gid = conv.json.conversation.id;
  await request('POST', `/api/conversations/${gid}/messages`, { body: 'dom seed one' }, aCookie);
  await request('POST', `/api/conversations/${gid}/messages`, { body: 'dom seed two' }, bCookie);
  await request('POST', '/api/files/upload?name=dom-notes.txt', 'file from the DOM suite', aCookie, 'text/plain');
  return { aCookie, bCookie, gid };
}

function mkFetch(cookie) {
  return async (url, opts = {}) => {
    const headers = opts.headers || {};
    const ctype = headers['Content-Type'] || headers['content-type'];
    const res = await request(
      opts.method || 'GET',
      url.startsWith('http') ? url.replace(BASE, '') : url,
      opts.body,
      cookie,
      ctype
    );
    return {
      ok: res.status >= 200 && res.status < 300,
      status: res.status,
      headers: { get: (k) => res.headers[String(k).toLowerCase()] || null },
      json: async () => res.json,
      text: async () => res.text,
    };
  };
}

async function bootPage(section, cookie) {
  const res = await request('GET', section, null, cookie);
  const vc = new VirtualConsole();
  vc.on('jsdomError', (e) => errors.push(`jsdomError(${section}): ${e.message}`));
  vc.on('error', (...a) => errors.push(`console.error(${section}): ${a.join(' ')}`));
  const dom = new JSDOM(res.text, {
    url: BASE + section,
    runScripts: 'outside-only',
    virtualConsole: vc,
    pretendToBeVisual: true,
  });
  const w = dom.window;
  w.fetch = mkFetch(cookie);
  w.matchMedia = w.matchMedia || (() => ({ matches: false, addEventListener() {}, removeEventListener() {} }));
  w.WebSocket = class {
    constructor() { this.readyState = 3; setTimeout(() => this.onclose && this.onclose(), 0); }
    send() {} close() {}
  };
  // Inline <script> tags are not executed with runScripts:'outside-only'.
  for (const sc of Array.from(w.document.querySelectorAll('script'))) {
    if (!sc.src) w.eval(sc.textContent);
  }
  w.eval(fs.readFileSync(path.join(ROOT, 'assets', 'app.js'), 'utf8'));
  await new Promise((r) => setTimeout(r, 450));
  return w;
}

const tick = (ms = 500) => new Promise((r) => setTimeout(r, ms));

function startServer() {
  const os = require('os');
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'room-dom-'));
  child = require('child_process').spawn(path.join(ROOT, 'dist', 'room'), [], {
    env: {
      ...process.env,
      ROOM_BIND: `127.0.0.1:${PORT}`,
      ROOM_DATA_DIR: path.join(dataDir, 'data'),
      ROOM_FILES_DIR: path.join(dataDir, 'files'),
      ROOM_NAME: 'ROOM DOM',
    },
    stdio: 'ignore',
  });
  return waitForServer();
}

async function waitForServer() {
  for (let i = 0; i < 100; i += 1) {
    try {
      const res = await request('GET', '/api/health');
      if (res.status === 200) return;
    } catch (e) { /* not up yet */ }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('server did not become ready');
}

function shutdown(code) {
  if (child) {
    try { child.kill('SIGKILL'); } catch (e) { /* already gone */ }
  }
  process.exit(code);
}

(async () => {
  if (!EXTERNAL) await startServer();
  const { aCookie, gid } = await prepare();

  for (const [p, label] of [
    ['/', 'chat'],
    ['/files', 'files'],
    ['/calls', 'calls'],
    ['/admin', 'admin'],
    ['/account', 'account'],
    [`/chat/${gid}`, 'conversation'],
  ]) {
    const w = await bootPage(p, aCookie);
    const bootType = w.eval('typeof window.__BOOT__');
    const boot = bootType === 'object' ? w.eval('window.__BOOT__') : {};
    check(`${label}: boot payload is an object`, bootType === 'object', bootType);
    check(`${label}: boot lists users`, Array.isArray(boot.users) && boot.users.length >= 2,
      JSON.stringify(boot.users && boot.users.length));
    check(`${label}: boot has me`, !!boot.me && boot.me.id > 0, boot.me && boot.me.name);
    check(`${label}: renders nodes`, w.document.querySelectorAll('*').length > 80);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    w.document.querySelector('[data-new-chat]').dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(250);
    const radios = w.document.querySelectorAll('input[type=radio].nc-check');
    check('new conversation dialog offers members', radios.length >= 1, radios.length);
    check('new conversation dialog has a create button', !!w.document.querySelector('#nc-go'));
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    w.document.querySelector('[data-act="add-member"]').dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(250);
    const boxes = w.document.querySelectorAll('input[type=checkbox].nc-check');
    check('new group dialog offers checkboxes', boxes.length >= 1, boxes.length);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    check('messages render as bubbles', w.document.querySelectorAll('#messages .bubble').length >= 2,
      w.document.querySelectorAll('#messages .bubble').length);
    check('own messages align right', w.document.querySelectorAll('#messages .bubble-row.own').length >= 1);
    check('day divider is shown', w.document.querySelectorAll('#messages .day-divider').length >= 1);
    check('bubbles carry timestamps', w.document.querySelectorAll('#messages .bubble-meta').length >= 2);
    check('conversation filters render', w.document.querySelectorAll('[data-filter]').length === 4,
      w.document.querySelectorAll('[data-filter]').length);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const before = w.document.querySelectorAll('#messages .msg-body').length;
    const form = w.document.querySelector('#composer');
    const input = w.document.querySelector('#composer-input');
    input.value = 'sent from the DOM suite';
    form.dispatchEvent(new w.Event('submit', { bubbles: true, cancelable: true }));
    await tick(900);
    const after = w.document.querySelectorAll('#messages .msg-body').length;
    check('composer posts and appends a message', after > before, `${before} -> ${after}`);
    check('composer message is rendered', w.document.querySelector('#messages').textContent.includes('sent from the DOM suite'));
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const before = w.document.documentElement.classList.contains('dark');
    w.document.querySelector('#theme-btn').dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(350);
    const after = w.document.documentElement.classList.contains('dark');
    check('theme toggle flips dark mode', before !== after, `${before} -> ${after}`);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const people = w.document.querySelector('[data-filter="people"]');
    people.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(150);
    const hiddenGroups = Array.from(w.document.querySelectorAll('#conv-list [data-conv][data-kind="group"]'))
      .every((a) => a.style.display === 'none');
    check('People filter hides group chats', hiddenGroups);
    const all = w.document.querySelector('[data-filter="all"]');
    all.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(150);
    const restored = Array.from(w.document.querySelectorAll('#conv-list [data-conv]'))
      .every((a) => a.style.display !== 'none');
    check('All filter restores every chat', restored);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const search = w.document.querySelector('#conv-search');
    search.value = 'zzz-no-such-conversation';
    search.dispatchEvent(new w.Event('input', { bubbles: true }));
    const shown = Array.from(w.document.querySelectorAll('#conv-list [data-conv]')).map((a) => a.style.display);
    check('conversation search filters the list', shown.every((d) => d === 'none'), JSON.stringify(shown));
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const react = w.document.querySelector('[data-act="react"]');
    react?.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
    await tick(200);
    const picker = w.document.querySelectorAll('[data-em]');
    check('reaction picker opens', picker.length >= 6, picker.length);
    w.close();
  }

  {
    const w = await bootPage(`/chat/${gid}`, aCookie);
    const box = w.document.querySelector('#conv-search');
    box.value = 'dom seed';
    box.dispatchEvent(new w.Event('input', { bubbles: true }));
    await tick(1200);
    const panel = w.document.querySelector('#search-panel');
    const links = panel ? panel.querySelectorAll('a') : [];
    check('global search returns hits', links.length >= 1, links.length);
    check('search panel lists messages', panel && panel.textContent.includes('dom seed'));
    w.close();
  }

  {
    const w = await bootPage('/files', aCookie);
    const menu = w.document.querySelector('[data-act="file-menu"]');
    if (menu) {
      menu.dispatchEvent(new w.MouseEvent('click', { bubbles: true }));
      await tick(200);
      const items = Array.from(w.document.querySelectorAll('[data-menu]')).map((b) => b.dataset.menu);
      check('file row menu opens', items.length >= 3, items.join(','));
      check('file row menu offers download', items.includes('download'), items.join(','));
      check('file row menu offers trash', items.includes('delete'), items.join(','));
    } else {
      check('file row menu opens', false, 'no file rows rendered');
    }
    w.close();
  }

  const failed = results.filter((r) => r[1] === 'FAIL');
  for (const r of results) {
    console.log(`${r[1] === 'ok' ? '  ok  ' : ' FAIL '} ${r[0]}${r[2] ? '  ' + r[2] : ''}`);
  }
  if (errors.length) {
    console.log('\npage errors:');
    for (const e of errors.slice(0, 15)) console.log('  ' + e);
  }
  console.log(`\n${results.length - failed.length}/${results.length} checks passed, ${errors.length} page errors`);
  shutdown(failed.length || errors.length ? 1 : 0);
})().catch((e) => {
  console.error('DOM suite crashed:', e.message);
  shutdown(1);
});
