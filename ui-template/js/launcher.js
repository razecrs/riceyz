// Launcher skin logic. Bare-bones app search to show the contract.
// The host pushes the installed-app list, you render + fire launch: on select.
function send(m) { if (window.ipc) window.ipc.postMessage(m); }

const input = document.getElementById('q');
const list = document.getElementById('results');
let apps = [], results = [], sel = 0;

// host -> you: the installed apps, pushed once (also pushSteam, pushBookmarks, etc. exist)
window.pushApps = a => { apps = a || []; };

// host -> you: called every time the launcher opens (Alt+Space). reset + focus here.
window.onLauncherShow = () => { input.value = ''; render(); input.focus(); };

function render() {
  const q = input.value.toLowerCase().trim();
  results = q ? apps.filter(a => a.name.toLowerCase().includes(q)).slice(0, 8) : [];
  sel = 0;
  list.innerHTML = results.map((a, i) => `<li class="${i === sel ? 'sel' : ''}">${a.name}</li>`).join('');
  [...list.children].forEach((li, i) => (li.onclick = () => run(i)));
  // tell the host our height so it can size the window to fit
  send('resize:' + document.getElementById('launcher').offsetHeight);
}

function run(i) { const a = results[i]; if (a) { send('launch:' + a.path); send('launcher:hide'); } }
function paint() { [...list.children].forEach((li, i) => li.classList.toggle('sel', i === sel)); }

input.addEventListener('input', render);
input.addEventListener('keydown', e => {
  if (e.key === 'Escape') send('launcher:hide');
  else if (e.key === 'Enter') run(sel);
  else if (e.key === 'ArrowDown') { sel = Math.min(sel + 1, results.length - 1); paint(); e.preventDefault(); }
  else if (e.key === 'ArrowUp') { sel = Math.max(sel - 1, 0); paint(); e.preventDefault(); }
});
