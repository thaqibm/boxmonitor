const $ = (selector) => document.querySelector(selector);
const terminal = $('#terminal');
const status = $('#status');
let demo;
let columns = 100;
const rows = 36;

// Keep the terminal cells mounted across ticks instead of replacing the entire
// display. This also preserves a stable layout while web fonts finish loading.
let grid = [];
function draw() {
  if (!demo) return;
  const lines = JSON.parse(demo.render(columns, rows));
  if (grid.length !== rows || grid[0]?.length !== columns) {
    const fragment = document.createDocumentFragment();
    grid = Array.from({length:rows}, () => {
      const line = document.createElement('div');
      line.className = 'line';
      const cells = Array.from({length:columns}, () => {
        const cell = document.createElement('span');
        cell.className = 'cell';
        line.append(cell);
        return cell;
      });
      fragment.append(line);
      return cells;
    });
    terminal.replaceChildren(fragment);
  }
  for (let y = 0; y < lines.length; y++) {
    let x = 0;
    for (const [text, fg, bg, bold] of lines[y]) {
      for (const symbol of text) {
        const cell = grid[y][x++];
        if (!cell) continue;
        if (cell.textContent !== symbol) cell.textContent = symbol;
        const background = bg === 'inherit' ? 'transparent' : bg;
        const weight = bold ? 'bold' : 'normal';
        if (cell.style.color !== fg) cell.style.color = fg;
        if (cell.style.backgroundColor !== background) cell.style.backgroundColor = background;
        if (cell.style.fontWeight !== weight) cell.style.fontWeight = weight;
      }
    }
  }
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
    $('.screen-scroll').scrollLeft = 0;
    terminal.focus({preventScroll:true});
  } catch (error) { message(typeof error === 'string' ? error : error.message, true); }
});
document.querySelectorAll('[data-key]').forEach(button => button.addEventListener('click', () => key(button.dataset.key)));
document.addEventListener('keydown', event => {
  if (event.target.closest('input, textarea, [contenteditable]') || event.ctrlKey || event.metaKey || event.altKey) return;
  const action = { ArrowRight:'next', l:'next', ArrowLeft:'previous', h:'previous', p:'plot' }[event.key];
  if (action) { event.preventDefault(); key(action); }
});
new ResizeObserver(resize).observe($('.screen-scroll'));
document.fonts.ready.then(resize);

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
