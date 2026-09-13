const $ = (selector) => document.querySelector(selector);
const terminal = $('#terminal');
const status = $('#status');
let demo, timer, paused = false, loading = false;
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
  $('#sample-count').textContent = `${demo.samples()} samples`;
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
function updateStatus() {
  status.textContent = paused ? 'Paused · Wasm' : 'Running · Wasm';
  $('#pause').textContent = paused ? 'Resume' : 'Pause';
}
function togglePause() { if (!demo) return; paused = !paused; updateStatus(); }
function key(action) { if (!demo) return; demo.key(action); draw(); }
async function start() {
  if (loading) return;
  loading = true;
  $('#start').disabled = true;
  status.textContent = 'Loading WebAssembly…';
  try {
    const { default: init, Demo } = await import('./pkg/boxmonitor.js');
    await init();
    demo?.free();
    demo = new Demo();
    paused = false;
    clearInterval(timer);
    timer = setInterval(() => { if (!paused) { demo.tick(); draw(); } }, 1000);
    document.querySelectorAll('[data-key], [data-scenario], #pause, #reset').forEach(button => button.disabled = false);
    document.querySelectorAll('[data-scenario]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.scenario === '0')));
    $('#start').textContent = 'Started';
    resize(); updateStatus(); terminal.focus({preventScroll:true});
  } catch (error) {
    status.textContent = 'Could not load demo. Select Retry.';
    terminal.textContent = `WebAssembly failed to load. Check your connection and try again.\n\n${error.message}`;
    $('#start').textContent = 'Retry';
    $('#start').disabled = false;
  } finally { loading = false; }
}
$('#start').addEventListener('click', start);
$('#reset').addEventListener('click', start);
$('#pause').addEventListener('click', togglePause);
document.querySelectorAll('[data-key]').forEach(button => button.addEventListener('click', () => key(button.dataset.key)));
document.querySelectorAll('[data-scenario]').forEach(button => button.addEventListener('click', () => {
  demo.set_scenario(Number(button.dataset.scenario));
  document.querySelectorAll('[data-scenario]').forEach(other => other.setAttribute('aria-pressed', String(other === button)));
  // Advance once immediately so the scenario has visible feedback, even when paused.
  demo.tick(); draw();
}));
terminal.addEventListener('keydown', event => {
  if (event.key === 'ArrowRight') { event.preventDefault(); key('next'); }
  else if (event.key === 'ArrowLeft') { event.preventDefault(); key('previous'); }
  else if (event.key.toLowerCase() === 'p') { event.preventDefault(); key('plot'); }
  else if (event.key === ' ') { event.preventDefault(); togglePause(); }
});
new ResizeObserver(resize).observe($('.screen-scroll'));
