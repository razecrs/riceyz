// Dashboard skin logic. The host (Rust) talks to you two ways:
//   1. it CALLS your window.push*() functions with data
//   2. you SEND it commands with window.ipc.postMessage("verb:args")
// Full contract is in PROTOCOL.md. Define only the callbacks you care about.

const $ = id => document.getElementById(id);

// ---- host -> you (define these to receive data) ----

// live CPU/RAM every ~1.5s
window.pushStats = ({ cpu, ram }) => {
  $('cpu').textContent = Math.round(cpu);
  $('ram').textContent = Math.round(ram);
};

// current media session (SMTC)
window.pushNowPlaying = ({ title, artist, playing }) => {
  $('now').textContent = title ? `${playing ? '▶' : '⏸'} ${title} - ${artist}` : '';
};

// 48 audio-spectrum bars (0..1), ~30fps. (This template just ignores them, use them if you want.)
window.pushViz = bars => { /* draw a visualizer here */ };

// a real Windows notification arrived. mode is "desktop" or "app".
let notifTimer;
window.pushNotif = (msg, mode) => {
  const n = $('notif');
  n.textContent = msg;
  n.classList.add('show');
  clearTimeout(notifTimer);
  notifTimer = setTimeout(() => n.classList.remove('show'), 5000);
};

// ---- you -> host ----
// e.g. bcIpc('open:https://github.com'), bcIpc('run:chrome'), bcIpc('power:lock'),
//      bcIpc('media:playpause'), bcIpc('reload')
function bcIpc(msg) { if (window.ipc) window.ipc.postMessage(msg); }

// clock (this part is just yours, no host involved)
setInterval(() => {
  const d = new Date();
  $('clock').textContent = `${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}`;
}, 1000);
