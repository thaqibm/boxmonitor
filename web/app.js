const $ = (selector) => document.querySelector(selector);
const terminal = $('#terminal');
const status = $('#status');
let demo;
let columns = 100;
const rows = 36;

function draw() {
  if (!demo) return;
  const fragment = document.createDocumentFragment();
  for (const runs of JSON.parse(demo.render(columns, rows))) {
    const line = document.createElement('div');
    line.className = 'line';
    for (const [text, fg, bg, bold] of runs) {
      const span = document.createElement('span');
      // Browser fallback fonts can give braille a different advance than ASCII.
      // Give every Ratatui cell exactly one monospace column.
      for (const symbol of text) {
        const cell = document.createElement('i');
        cell.className = 'cell';
        cell.textContent = symbol;
        span.append(cell);
      }
      span.style.color = fg;
      if (bg !== 'inherit') span.style.backgroundColor = bg;
      if (bold) span.style.fontWeight = 'bold';
      line.append(span);
    }
    fragment.append(line);
  }
  terminal.replaceChildren(fragment);
}
function resize() {
  // Measure the actual terminal font instead of assuming a character width.
  const probe = document.createElement('span');
  probe.textContent = '0000000000';
  terminal.append(probe);
  const width = probe.getBoundingClientRect().width / 10;
  probe.remove();
  columns = Math.max(80, Math.min(180, Math.floor(($('.screen-scroll').clientWidth - 24) / width)));
  draw();
}

function message(text, error = false) {
  status.textContent = text;
  status.classList.toggle('error', error);
}
function key(action) { if (demo) { demo.key(action); draw(); } }

function parseHost(value) {
  const separator = value.indexOf('=');
  // An equals sign in a URL query is not a name separator.
  const named = separator >= 0 && !/^https?:\/\//i.test(value);
  const raw = (named ? value.slice(separator + 1) : value).trim();
  let name = named ? value.slice(0, separator).trim() : '';
  if (!raw || /\s/.test(raw)) throw new Error('Enter a host or HTTP(S) URL, optionally preceded by Name =');
  const url = new URL(raw.includes('://') ? raw : `https://${raw}`);
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) throw new Error('Use a host or HTTP(S) URL without credentials.');
  const host = url.hostname.replace(/\.$/, '').toLowerCase();
  if (!host || (!host.startsWith('[') && !host.split('.').every(label => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/i.test(label)))) throw new Error('Enter a valid hostname or IP address.');
  if (!name) name = host.slice(0, 40);
  if (!/^[\x20-\x7e]{1,40}$/.test(name)) throw new Error('Use a name of 1–40 printable ASCII characters.');
  return { host, name };
}

$('#add-host').addEventListener('submit', event => {
  event.preventDefault();
  if (!demo) return;
  try {
    const {host, name} = parseHost($('#host').value.trim());
    demo.add_target(host, name);
    $('#host').value = '';
    message(`Added ${name} (${host}) · simulated`);
    draw();
  } catch (error) { message(typeof error === 'string' ? error : error.message, true); }
});
document.querySelectorAll('[data-key]').forEach(button => button.addEventListener('click', () => key(button.dataset.key)));
document.addEventListener('keydown', event => {
  if (event.target.closest('input, textarea, [contenteditable]') || event.ctrlKey || event.metaKey || event.altKey) return;
  const action = { ArrowRight:'next', l:'next', ArrowLeft:'previous', h:'previous', p:'plot' }[event.key];
  if (action) { event.preventDefault(); key(action); }
});
new ResizeObserver(resize).observe($('.screen-scroll'));

try {
  const { default:init, Demo } = await import('./pkg/boxmonitor.js');
  await init();
  demo = new Demo();
  document.querySelectorAll('button, input').forEach(control => control.disabled = false);
  resize();
  message('');
  setInterval(() => { demo.tick(); draw(); }, 1000);
} catch {
  message('Could not load the demo. Reload the page to retry.', true);
  terminal.textContent = 'WebAssembly could not load.';
}
