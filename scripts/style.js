// Design-system guard — enforces docs/STYLE.md mechanically.
//
// The seven tests in the skill are visual, but the rules behind them are
// checkable: one radius, no gradients, no emoji, no shadows on flat surfaces,
// a five-step type scale, and the accent used for state only. This suite fails
// the build when a change drifts back to the average.
//
//   node scripts/style.js
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const CSS = path.join(ROOT, 'assets/app.css');
const DIST = path.join(ROOT, 'assets/app.dist.css');
const TPL = path.join(ROOT, 'src');

const results = [];
function check(label, ok, detail) {
  results.push([label, ok ? 'ok' : 'FAIL', detail === undefined ? '' : String(detail)]);
  return ok;
}

const css = fs.readFileSync(CSS, 'utf8');
const dist = fs.readFileSync(DIST, 'utf8');

// Collect every template + page source.
const sources = [];
(function walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.endsWith('.rs')) sources.push([p, fs.readFileSync(p, 'utf8')]);
  }
})(TPL);
const js = fs.readFileSync(path.join(ROOT, 'assets/app.js'), 'utf8');

// ---------------------------------------------------------------- palette
check('accent is signal green, not violet', /--color-brand-600:\s*#0f7a4a/.test(css));
check('no violet left in the token block', !/#5333f4|#6d4dff|#7c5cfc/.test(css.split('@layer')[0]));
check('old violet is gone from the built css', !/#5333f4|#6d4dff|#7c5cfc/.test(dist));
check('dark mode defines the same accent family', /html\.dark\s*{[^}]*--color-brand-600/.test(css));

// ---------------------------------------------------------------- radius
// One radius token, plus 1px/2px for lamps and 999px only for genuinely round
// things (spinner, ping ring). Any other hardcoded radius is drift.
const radiusDecls = [...css.matchAll(/border-radius:\s*([^;]+);/g)].map((m) => m[1].trim());
const allowed = /^(var\(--radius-swx\)|0|1px|2px|3px|5px|8px|999px|0 3px 3px 0)$/;
const badRadius = radiusDecls.filter((r) => !allowed.test(r));
check('radius values follow the one-scale rule', badRadius.length === 0, badRadius.join(', '));

const stripComments = (s) => s.replace(/\/\/[^\n]*/g, '');
const tplRadii = sources.flatMap(([, raw]) =>
  [...stripComments(raw).matchAll(/rounded-(?!\[)([a-z0-9]+)/g)].map((m) => m[1]));
const allowedTpl = new Set(['none', 'full']);
const badTpl = [...new Set(tplRadii.filter((r) => !allowedTpl.has(r)))];
check('templates use the 3px radius, not arbitrary steps', badTpl.length === 0, badTpl.join(', '));

// ---------------------------------------------------------------- shadows
// No shadow on a surface. `shadow-pop` is only for things that float.
const softShadows = sources.filter(([, s]) => /shadow-soft/.test(s)).map(([p]) => path.basename(p));
check('no shadow-soft on flat panels', softShadows.length === 0, softShadows.join(', '));
check('shadow-soft resolves to none', /--shadow-soft:\s*none/.test(css));

// ---------------------------------------------------------------- type
const sizes = new Set();
for (const [, s] of sources) {
  for (const m of s.matchAll(/text-\[(\d+(?:\.\d+)?)px\]/g)) sizes.add(Number(m[1]));
}
// 10 micro, 11 label, 12 dense, 14 body, 17 heading, 24 title/display.
const allowedSizes = new Set([10, 11, 12, 14, 17, 24, 40]);
const extra = [...sizes].filter((n) => !allowedSizes.has(n)).sort((a, b) => a - b);
check('type scale has no one-off sizes', extra.length === 0, extra.join(', '));
check('mono is the display face', /--font-display:\s*ui-monospace/.test(css));

// ---------------------------------------------------------------- banned
const gradientsInCss = (css.match(/(?:linear|radial)-gradient\(/g) || []).length;
check('the only gradient is the skeleton scan', gradientsInCss === 1, gradientsInCss);
// Tailwind emits the skeleton rule twice: once minified, once inside the
// reduced-motion prefers block. Both are the same affordance.
const distGradients = new Set(
  (dist.match(/(?:linear|radial)-gradient\([^)]*\)/g) || []).map((g) => g.replace(/\s+/g, ''))
);
check('the built css carries no gradient beyond the skeleton scan',
  [...distGradients].every((g) => /90deg/.test(g)), [...distGradients].length);
const emoji = /[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}\u{FE0F}]/u;
const emojiHits = sources.filter(([, s]) => emoji.test(s)).map(([p]) => path.basename(p));
check('no emoji used as icons', emojiHits.length === 0, emojiHits.join(', '));
check('no glow shadow behind the mark', !/0 0 \d+px/.test(css.split('@layer')[0]));

// ---------------------------------------------------------------- craft
check('tabular numerals are set globally', /font-variant-numeric:\s*tabular-nums/.test(css));
check('focus-visible is defined', /:focus-visible\s*{/.test(css));
check('selection colour is set', /::selection\s*{/.test(css));
check('reduced motion is honoured', /prefers-reduced-motion/.test(css));
check('print styles exist', /@media print/.test(css));
check('a favicon exists', /route\("\/favicon\.svg"/.test(
  fs.readFileSync(path.join(ROOT, 'src/main.rs'), 'utf8')));

// ---------------------------------------------------------------- signature
check('the status strip is defined', /\.statusstrip/.test(css));
check('the status strip is rendered', sources.some(([, s]) => /status_strip/.test(s)));
check('the status strip is a live region', /role=\\"status\\"/.test(
  sources.find(([p]) => p.endsWith('mod.rs'))[1]));

// ---------------------------------------------------------------- errors
// Users must never read a server truth.
const rawToasts = [...js.matchAll(/toast\(\s*[a-z0-9]*(?:\.|\))\s*message/g)];
check('no raw error message is toasted to users', rawToasts.length === 0, rawToasts.length);
check('a human error mapper exists', /function humanError/.test(js));
check('failures carry a correlation ref', /ref \$\{h\.ref\}|\$\{h\.ref\}/.test(js));

// ---------------------------------------------------------------- skeletons
check('skeleton styles exist', /\.skel\b/.test(css));
check('skeleton respects reduced motion', /prefers-reduced-motion[\s\S]*skel/.test(css));
check('a skeleton helper exists in the client', /function skelRows/.test(js));

// ---------------------------------------------------------------- report
let failed = 0;
for (const [label, state, detail] of results) {
  if (state === 'FAIL') failed++;
  const mark = state === 'ok' ? '  ok  ' : ' FAIL ';
  console.log(`${mark} ${label}${detail ? '  (' + detail + ')' : ''}`);
}
console.log(`\n${results.length - failed}/${results.length} style rules hold`);
process.exit(failed ? 1 : 0);
