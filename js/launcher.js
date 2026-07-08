/* ===== Batcave Launcher, provider engine ===== */
const $ = s => document.querySelector(s);
const input = $('#lsearch');
let results = [], sel = 0, apps = [];

window.addEventListener('error', e => {
  const u = document.getElementById('lresults');
  if(u) u.innerHTML = '<li style="padding:12px;color:#ff8a8a;font-family:monospace;font-size:11px">JS ERROR: ' + (e.message || e) + '</li>';
});
function send(m){ if(window.ipc) window.ipc.postMessage(m); }
function hide(){ send('launcher:hide'); }
function fit(){ const l = document.getElementById('launcher'); if(l) send('resize:' + l.offsetHeight); }
async function copy(t){ try { await navigator.clipboard.writeText(String(t)); } catch(_){} }
function he(s){ return String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c])); }

/* ---- lucide-style icons ---- */
const IC = {
  search:'<circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.5" y2="16.5"/>',
  app:'<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
  calc:'<rect x="4" y="2" width="16" height="20" rx="2"/><line x1="8" y1="6" x2="16" y2="6"/><line x1="8" y1="14" x2="8" y2="14"/><line x1="12" y1="14" x2="12" y2="14"/><line x1="16" y1="14" x2="16" y2="18"/><line x1="8" y1="18" x2="12" y2="18"/>',
  globe:'<circle cx="12" cy="12" r="9"/><line x1="3" y1="12" x2="21" y2="12"/><path d="M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18"/>',
  ruler:'<path d="M3 8l5-5 13 13-5 5z"/><path d="M7 7l2 2M11 5l2 2M9 11l2 2M13 9l2 2"/>',
  clock:'<circle cx="12" cy="12" r="9"/><polyline points="12 7 12 12 16 14"/>',
  text:'<polyline points="4 7 4 4 20 4 20 7"/><line x1="9" y1="20" x2="15" y2="20"/><line x1="12" y1="4" x2="12" y2="20"/>',
  power:'<path d="M12 3v9"/><path d="M6.5 6.5a9 9 0 1 0 11 0"/>',
  gear:'<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"/>',
  bat:'<path d="M12 5c-1 2-3 2.5-4.5 1.5C8 9 6 9.5 4 8c1 2 .5 3.5-1 4 2 .5 3 2 3 3.5 1.5-1.5 4-1 5 1 1-2 3.5-2.5 5-1 0-1.5 1-3 3-3.5-1.5-.5-2-2-1-4-2 1.5-4 1-2.5-1.5C15 7.5 13 7 12 5z"/>',
  media:'<polygon points="6 4 20 12 6 20 6 4"/>',
  steam:'<circle cx="12" cy="12" r="9"/><circle cx="15.5" cy="8.5" r="2.4"/><circle cx="8" cy="15" r="1.9"/><line x1="13.5" y1="10.5" x2="9.6" y2="13.6"/>',
  audio:'<polygon points="4 9 4 15 8 15 13 19 13 5 8 9 4 9"/><path d="M17 8.5a4.5 4.5 0 0 1 0 7"/>',
  sun:'<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  clip:'<rect x="8" y="2" width="8" height="4" rx="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>',
  terminal:'<rect x="3" y="4" width="18" height="16" rx="2"/><polyline points="7 9 10 12 7 15"/><line x1="12" y1="15" x2="16" y2="15"/>',
  note:'<path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z"/><line x1="9" y1="13" x2="15" y2="13"/><line x1="9" y1="17" x2="13" y2="17"/>',
  bookmark:'<path d="M6 3h12a1 1 0 0 1 1 1v17l-7-4-7 4V4a1 1 0 0 1 1-1z"/>',
  history:'<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><polyline points="3 3 3 8 8 8"/><polyline points="12 7 12 12 15 14"/>',
  coin:'<circle cx="12" cy="12" r="9"/><path d="M14.5 9.3a2.6 2.6 0 0 0-2.5-1.3c-1.4 0-2.5.7-2.5 2s1.1 1.7 2.5 2 2.5.6 2.5 2-1.1 2-2.5 2a2.6 2.6 0 0 1-2.5-1.3"/><line x1="12" y1="6" x2="12" y2="8"/><line x1="12" y1="16" x2="12" y2="18"/>',
  github:'<path d="M9 19c-4.3 1.4-4.3-2.5-6-3m12 5v-3.5c0-1 .1-1.4-.5-2 2.8-.3 5.5-1.4 5.5-6a4.6 4.6 0 0 0-1.3-3.2 4.2 4.2 0 0 0-.1-3.2s-1.1-.3-3.5 1.3a12 12 0 0 0-6 0C6.5 2.8 5.4 3.1 5.4 3.1a4.2 4.2 0 0 0-.1 3.2A4.6 4.6 0 0 0 4 9.5c0 4.6 2.7 5.7 5.5 6-.6.6-.6 1.2-.5 2V21"/>',
  bell:'<path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/>',
  palette:'<circle cx="12" cy="12" r="9"/><circle cx="8.5" cy="10.5" r="1"/><circle cx="15.5" cy="10.5" r="1"/><circle cx="9.5" cy="15" r="1"/><path d="M12 3a9 9 0 0 0 0 18c1.5 0 2-1 2-2s-.5-2 1-2h2a4 4 0 0 0 4-4 9 9 0 0 0-9-9z"/>',
  file:'<path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/>',
  skull:'<path d="M12 2a8 8 0 0 0-8 8v3l-1 3h3v3h3v-2h4v2h3v-3h3l-1-3v-3a8 8 0 0 0-8-8z"/><circle cx="9" cy="11.5" r="1.4"/><circle cx="15" cy="11.5" r="1.4"/>',
  plug:'<path d="M9 2v6M15 2v6"/><path d="M7 8h10v3a5 5 0 0 1-10 0z"/><path d="M12 16v6"/>',
};
const svg = k => `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">${IC[k] || IC.search}</svg>`;

/* ---- fuzzy score ---- */
function fscore(q, name){
  name = name.toLowerCase(); q = q.toLowerCase();
  if(!q) return 1;
  const i = name.indexOf(q);
  if(i === 0) return 1000; if(i > 0) return 500 - i;
  let qi = 0; for(const c of name){ if(c === q[qi]) qi++; if(qi === q.length) return 100; }
  return -1;
}

/* ═══ PROVIDERS: (query) -> [{icon|emoji, title, subtitle, score, action}] ═══ */
const providers = [];
const provider = fn => providers.push(fn);

/* theme engine:  theme red / theme #ff00aa  (recolors launcher + dashboard) */
const THEMES = { blue:['#1b9dff','#0c5f96'], red:['#ff3131','#a01f1f'], yellow:['#ffcf1a','#b8940f'],
  white:['#e6e9ee','#8a95a5'], green:['#3ddc84','#1f8f56'], purple:['#c6a0f6','#7d5bbe'], orange:['#ff8c42','#b85c1f'] };
function applyTheme(accent, dim){
  localStorage.setItem('bcAccent', accent); localStorage.setItem('bcDim', dim);
  const R = document.documentElement.style;
  R.setProperty('--accent', accent); R.setProperty('--accent-dim', dim);
  const [r,g,b] = [1,3,5].map(i => parseInt(accent.slice(i,i+2),16));
  R.setProperty('--glow', `rgba(${r},${g},${b},.55)`);
}
provider(q => {
  const m = q.match(/^theme\s+(#?[0-9a-z]+)$/i); if(!m) return [];
  const key = m[1].toLowerCase();
  let accent, dim;
  if(THEMES[key]){ [accent, dim] = THEMES[key]; }
  else if(/^#?[0-9a-f]{6}$/i.test(key)){ accent = key[0] === '#' ? key : '#' + key; dim = accent; }
  else return [];
  return [{icon:'palette', title:`Theme → ${key}`, subtitle:'recolor launcher + dashboard', score:5000,
    action:() => { applyTheme(accent, dim); hide(); }}];
});

/* apps, show all (sorted) on empty query, fuzzy-ranked otherwise */
provider(q => {
  const ranked = q
    ? apps.map(a => ({a, s: fscore(q, a.name)})).filter(x => x.s >= 0).sort((x,y) => y.s - x.s)
    : apps.map(a => ({a, s: 1})).sort((x,y) => x.a.name.localeCompare(y.a.name));
  return ranked.slice(0, 100).map(x => ({icon:'app', img: appIcons[x.a.path], title:x.a.name, subtitle:x.a.path,
    score:x.s + 200, action:() => send('launch:' + x.a.path)}));
});
let appIcons = {};
window.pushAppIcons = m => { appIcons = m || {}; search(); };

/* calculator:  = expr   or bare math */
provider(q => {
  const expr = q.replace(/^=\s*/, '');
  if(!/\d/.test(expr) || q.length < 2) return [];
  const v = calc(expr);
  if(v == null) return [];
  return [{icon:'calc', title:String(v), subtitle:`= ${expr}   ·   Enter to copy`, score:6000,
    action:() => { copy(v); hide(); }}];
});

/* web search: keyword triggers */
const ENGINES = {
  g:['Google','https://www.google.com/search?q='], yt:['YouTube','https://www.youtube.com/results?search_query='],
  wiki:['Wikipedia','https://en.wikipedia.org/w/index.php?search='], gh:['GitHub','https://github.com/search?q='],
  ddg:['DuckDuckGo','https://duckduckgo.com/?q='], npm:['npm','https://www.npmjs.com/search?q='],
  so:['Stack Overflow','https://stackoverflow.com/search?q='], mdn:['MDN','https://developer.mozilla.org/en-US/search?q='],
};
provider(q => {
  const m = q.match(/^(\w+)\s+(.+)/);
  if(m && ENGINES[m[1].toLowerCase()]){
    const [name, url] = ENGINES[m[1].toLowerCase()];
    return [{icon:'globe', title:`Search ${name}`, subtitle:m[2], score:5500,
      action:() => { send('open:' + url + encodeURIComponent(m[2])); hide(); }}];
  }
  return [];
});

/* currency converter (live rates from Rust):  "100 usd to eur" */
let rates = {};
window.pushRates = r => { rates = r || {}; };
const CUR_ALIAS = { yen:'jpy', dollar:'usd', dollars:'usd', buck:'usd', bucks:'usd', euro:'eur', euros:'eur',
  pound:'gbp', pounds:'gbp', sterling:'gbp', quid:'gbp', rupee:'inr', rupees:'inr', yuan:'cny', rmb:'cny',
  won:'krw', ruble:'rub', rouble:'rub', real:'brl', reais:'brl', peso:'mxn', franc:'chf', rand:'zar',
  lira:'try', dirham:'aed', riyal:'sar', shekel:'ils', zloty:'pln', baht:'thb', ringgit:'myr', rupiah:'idr',
  dong:'vnd', naira:'ngn', krona:'sek', krone:'nok', forint:'huf', koruna:'czk', hryvnia:'uah', taka:'bdt' };
const curCode = x => { x = x.toLowerCase(); return CUR_ALIAS[x] || x; };
provider(q => {
  const m = q.match(/^([\d.]+)\s*([a-z]{3,})\s*(?:to|in)\s*([a-z]{3,})$/i);
  if(!m) return [];
  const amt = parseFloat(m[1]), from = curCode(m[2]), to = curCode(m[3]);
  if(rates[from] == null || rates[to] == null) return [];
  const out = +(amt * rates[to] / rates[from]).toFixed(4);
  return [{icon:'coin', title:`${out} ${to.toUpperCase()}`, subtitle:`${amt} ${from.toUpperCase()}  ·  live rate  ·  Enter to copy`, score:5600,
    action:() => { copy(out); hide(); }}];
});

/* unit + temperature converter:  "10 kg to lb" */
provider(q => { const r = convert(q); return r ? [{icon:'ruler', title:r.out, subtitle:`${r.desc}   ·   Enter to copy`, score:5500, action:() => { copy(r.val); hide(); }}] : []; });

/* time & date */
provider(q => {
  const s = q.trim().toLowerCase(); const out = [];
  if(['time','now','date','today'].includes(s)){
    const d = new Date();
    out.push({icon:'clock', title:d.toLocaleTimeString(), subtitle:d.toLocaleDateString(undefined,{weekday:'long',year:'numeric',month:'long',day:'numeric'}), score:4000, action:() => { copy(d.toLocaleString()); hide(); }});
  }
  const um = s.match(/^(\d{9,13})$/);
  if(um){ const ms = um[1].length > 10 ? +um[1] : +um[1]*1000; const d = new Date(ms);
    out.push({icon:'clock', title:d.toLocaleString(), subtitle:`unix ${um[1]}   ·   Enter to copy`, score:4200, action:() => { copy(d.toString()); hide(); }}); }
  return out;
});

/* text tools:  b64e / b64d / urle / urld <text> */
provider(q => {
  const m = q.match(/^(b64e|b64d|urle|urld)\s+([\s\S]+)/i); if(!m) return [];
  const op = m[1].toLowerCase(); let r;
  try {
    if(op === 'b64e') r = btoa(unescape(encodeURIComponent(m[2])));
    else if(op === 'b64d') r = decodeURIComponent(escape(atob(m[2])));
    else if(op === 'urle') r = encodeURIComponent(m[2]);
    else r = decodeURIComponent(m[2]);
  } catch(_){ return [{icon:'text', title:'(invalid input)', subtitle:op, score:4500, action:() => {}}]; }
  return [{icon:'text', title:r, subtitle:`${op}   ·   Enter to copy`, score:4800, action:() => { copy(r); hide(); }}];
});

/* emoji:  emoji <name>  or  :<name> */
const EMOJI = [['fire','🔥'],['heart','❤️'],['skull','💀'],['rocket','🚀'],['star','⭐'],['check','✅'],['cross','❌'],['eyes','👀'],['bat','🦇'],['thumbsup','👍'],['laugh','😂'],['cry','😭'],['cool','😎'],['think','🤔'],['party','🎉'],['100','💯'],['clap','👏'],['pray','🙏'],['brain','🧠'],['moon','🌙'],['bolt','⚡'],['ghost','👻'],['robot','🤖'],['money','💰'],['gem','💎'],['crown','👑'],['sparkles','✨'],['warning','⚠️'],['bug','🐛'],['gear','⚙️'],['lock','🔒'],['key','🔑'],['trophy','🏆'],['target','🎯']];
provider(q => {
  const m = q.match(/^(?:emoji|:)\s*(.+)/i); if(!m) return [];
  const t = m[1].toLowerCase();
  return EMOJI.filter(([n]) => n.includes(t)).slice(0, 8)
    .map(([n,e]) => ({emoji:e, title:`${e}   ${n}`, subtitle:'emoji   ·   Enter to copy', score:4600, action:() => { copy(e); hide(); }}));
});

/* system commands */
const SYS = [['shutdown','shutdown /s /t 0'],['restart','shutdown /r /t 0'],['sleep','rundll32.exe powrprof.dll,SetSuspendState 0,1,0'],['lock','rundll32.exe user32.dll,LockWorkStation'],['sign out','shutdown /l'],['logoff','shutdown /l']];
provider(q => {
  if(q.length < 2) return [];
  return SYS.filter(([n]) => n.includes(q.toLowerCase()))
    .map(([n,cmd]) => ({icon:'power', title:n.toUpperCase(), subtitle:'system command', score:4300, action:() => { send('sys:' + cmd); hide(); }}));
});

/* windows settings */
const SETTINGS = [['Display','ms-settings:display'],['Sound','ms-settings:sound'],['Bluetooth','ms-settings:bluetooth'],['Wi-Fi','ms-settings:network-wifi'],['Apps','ms-settings:appsfeatures'],['Windows Update','ms-settings:windowsupdate'],['Power &amp; Sleep','ms-settings:powersleep'],['Storage','ms-settings:storagesense'],['About','ms-settings:about'],['Mouse','ms-settings:mousetouchpad'],['Keyboard','ms-settings:keyboard'],['Themes','ms-settings:themes'],['Notifications','ms-settings:notifications']];
provider(q => {
  if(q.length < 3) return [];
  return SETTINGS.filter(([n]) => n.toLowerCase().includes(q.toLowerCase())).slice(0, 5)
    .map(([n,uri]) => ({icon:'gear', title:n, subtitle:uri, score:4100, action:() => { send('open:' + uri); hide(); }}));
});

/* notification test (batarang) */
provider(q => {
  const m = q.match(/^notif\s*(.*)/i); if(!m) return [];
  const msg = m[1].trim() || 'BATARANG INBOUND, MASTER WAYNE';
  return [{icon:'bat', title:'Send notification', subtitle:'» ' + msg, score:5000, action:() => { send('notif:' + msg); hide(); }}];
});

/* media control (SMTC via Rust) */
provider(q => {
  if(q.length < 3) return [];
  const items = [['play media','playpause'],['pause media','playpause'],['next track','next'],['previous track','prev']];
  return items.filter(([n]) => n.includes(q.toLowerCase()))
    .map(([n,cmd]) => ({icon:'media', title:n.toUpperCase(), subtitle:'media control', score:4400,
      action:() => { send('media:' + cmd); hide(); }}));
});

/* volume (Core Audio via Rust):  vol 50 / vol chrome 30 / mute / unmute */
provider(q => {
  const app = q.match(/^(?:vol|volume)\s+([a-z][\w.]*)\s+(\d{1,3})$/i);
  if(app){ const n = Math.min(100, +app[2]); return [{icon:'audio', title:`${app[1]} → ${n}%`, subtitle:'app volume', score:5300, action:() => { send('appvol:' + app[1] + ':' + n); hide(); }}]; }
  const m = q.match(/^(?:vol|volume)\s*(\d{1,3})$/i);
  if(m){ const n = Math.min(100, +m[1]); return [{icon:'audio', title:`Set volume ${n}%`, subtitle:'master volume', score:5200, action:() => { send('vol:' + n); hide(); }}]; }
  const s = q.trim().toLowerCase();
  if(s === 'mute') return [{icon:'audio', title:'MUTE', subtitle:'master volume', score:5000, action:() => { send('vol:mute'); hide(); }}];
  if(s === 'unmute') return [{icon:'audio', title:'UNMUTE', subtitle:'master volume', score:5000, action:() => { send('vol:unmute'); hide(); }}];
  return [];
});

/* external plugins (JSON stdio via Rust), supplement any query */
let pluginCache = {}, pluginTimer;
window.pushPlugins = (q, results) => { pluginCache[q] = results || []; if(input.value.trim() === q) search(); };
provider(q => {
  const term = q.trim(); if(term.length < 2) return [];
  if(pluginCache[term]) return pluginCache[term].map(r => ({icon:'plug', title:r.title, subtitle:r.subtitle, score:4900,
    action:() => { if(r.action) send(r.action); hide(); }}));
  clearTimeout(pluginTimer); pluginTimer = setTimeout(() => send('plugin:' + term), 220);
  return [];
});

/* lite-mode toggle:  fs  (turn the RAM-heavy file index on/off) */
let fileSearchOn = true;
window.pushFsState = b => { fileSearchOn = b; };
provider(q => {
  if(!/^(fs|filesearch|litemode)$/i.test(q.trim())) return [];
  return [{icon:'file', title:`File Search (Everything mode): ${fileSearchOn ? 'ON' : 'OFF'}`,
    subtitle: fileSearchOn ? 'Enter → turn OFF  ·  frees ~600 MB  ·  host restarts' : 'Enter → turn ON  ·  host restarts',
    score:5200, action:() => { send('togglefs'); hide(); }}];
});

/* plugin manager:  plugins */
let pluginList = [];
window.pushPluginList = list => { pluginList = list || []; };
provider(q => {
  if(!/^plugins?$/i.test(q.trim())) return [];
  if(!pluginList.length) return [{icon:'plug', title:'No plugins loaded', subtitle:'drop a folder in batcave-dashboard\\plugins', score:5000, action:() => {}}];
  return pluginList.map(p => ({icon:'plug', title:p.name, subtitle:'trigger: ' + (p.trigger || '(all queries)'), score:5000, action:() => {}}));
});

/* browser open tabs (via the Batcave extension bridge):  tab <query> */
let tabCache = {}, tabTimer;
window.pushTabs = (q, results) => { tabCache[q] = results || []; const cur = (input.value.match(/^tab\s+(.+)/i) || [])[1]; if(cur && cur.trim() === q) search(); };
provider(q => {
  const m = q.match(/^tab\s+(.+)/i); if(!m) return [];
  const term = m[1].trim(); if(term.length < 1) return [];
  if(tabCache[term]){
    const r = tabCache[term];
    if(!r.length) return [{icon:'globe', title:'No matching tabs', subtitle:'load the Batcave extension in your browser', score:5100, action:() => {}}];
    return r.map(t => ({icon:'globe', title:t.title || t.url, subtitle:t.url, score:5100,
      action:() => { send('tabactivate:' + t.id + ':' + t.win); hide(); }}));
  }
  clearTimeout(tabTimer); tabTimer = setTimeout(() => send('tabsearch:' + term), 150);
  return [{icon:'globe', title:'Searching tabs…', subtitle:term, score:5100, action:() => {}}];
});

/* process killer (async via Rust):  kill <name> */
let procCache = {}, procTimer;
const fmtMB = b => (b / 1048576).toFixed(0) + ' MB';
window.pushProcs = (q, results) => { procCache[q] = results || []; const cur = (input.value.match(/^kill\s+(.+)/i) || [])[1]; if(cur && cur.trim() === q) search(); };
provider(q => {
  const m = q.match(/^kill\s+(.+)/i); if(!m) return [];
  const term = m[1].trim(); if(term.length < 2) return [];
  if(procCache[term]){
    const r = procCache[term];
    if(!r.length) return [{icon:'skull', title:'No matching process', subtitle:term, score:5200, action:() => {}}];
    return r.map(p => ({icon:'skull', title:`Kill  ${p.name}`, subtitle:`PID ${p.pid}  ·  ${fmtMB(p.mem)}  ·  Enter to terminate`, score:5200,
      action:() => { send('killpid:' + p.pid); hide(); }}));
  }
  clearTimeout(procTimer); procTimer = setTimeout(() => send('proc:' + term), 200);
  return [{icon:'skull', title:'Finding processes…', subtitle:term, score:5200, action:() => {}}];
});

/* color tool:  #ff0000  or  rgb(255,0,0) */
const swatch = hex => 'data:image/svg+xml,' + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" rx="5" fill="${hex}"/></svg>`);
function rgbToHsl(r,g,b){
  r/=255; g/=255; b/=255;
  const mx = Math.max(r,g,b), mn = Math.min(r,g,b); let h, s, l = (mx+mn)/2;
  if(mx === mn){ h = s = 0; }
  else { const d = mx-mn; s = l > .5 ? d/(2-mx-mn) : d/(mx+mn);
    h = mx===r ? (g-b)/d+(g<b?6:0) : mx===g ? (b-r)/d+2 : (r-g)/d+4; h /= 6; }
  return `hsl(${Math.round(h*360)}, ${Math.round(s*100)}%, ${Math.round(l*100)}%)`;
}
provider(q => {
  const s = q.trim();
  let r, g, b;
  const mh = s.match(/^#?([0-9a-f]{6})$/i);
  const mr = s.match(/^rgb\(?\s*(\d{1,3})[,\s]+(\d{1,3})[,\s]+(\d{1,3})\s*\)?$/i);
  if(mh){ const n = parseInt(mh[1],16); r=(n>>16)&255; g=(n>>8)&255; b=n&255; }
  else if(mr){ r=+mr[1]; g=+mr[2]; b=+mr[3]; if(r>255||g>255||b>255) return []; }
  else return [];
  const hex = '#' + [r,g,b].map(v => v.toString(16).padStart(2,'0')).join('').toUpperCase();
  const rgb = `rgb(${r}, ${g}, ${b})`, hsl = rgbToHsl(r,g,b);
  const img = swatch(hex);
  return [
    {icon:'palette', img, title:hex, subtitle:'HEX  ·  Enter to copy', score:5600, action:() => { copy(hex); hide(); }},
    {icon:'palette', img, title:rgb, subtitle:'RGB  ·  Enter to copy', score:5590, action:() => { copy(rgb); hide(); }},
    {icon:'palette', img, title:hsl, subtitle:'HSL  ·  Enter to copy', score:5580, action:() => { copy(hsl); hide(); }},
  ];
});

/* instant file search, NTFS MFT index (async via Rust):  f <query> */
let fileCache = {}, fileTimer;
window.pushFiles = (q, results) => { fileCache[q] = results || []; const cur = (input.value.match(/^f\s+(.+)/i) || [])[1]; if(cur && cur.trim() === q) search(); };
provider(q => {
  const m = q.match(/^f\s+(.+)/i); if(!m) return [];
  const term = m[1].trim();
  if(term.length < 2) return [];
  if(fileCache[term]){
    const r = fileCache[term];
    if(!r.length) return [{icon:'file', title:'No files found', subtitle:'run host as admin to enable MFT search', score:5300, action:() => {}}];
    return r.map(f => ({icon:'file', title:f.name, subtitle:f.path, score:5300, action:() => { send('launch:' + f.path); hide(); }}));
  }
  clearTimeout(fileTimer); fileTimer = setTimeout(() => send('file:' + term), 200);
  return [{icon:'file', title:'Searching files…', subtitle:term, score:5300, action:() => {}}];
});

/* Spotify search (async via Rust):  sp <query> */
let spCache = {}, spTimer;
window.pushSpotify = (q, results) => { spCache[q] = results || []; const cur = (input.value.match(/^sp\s+(.+)/i) || [])[1]; if(cur && cur.trim() === q) search(); };
provider(q => {
  const m = q.match(/^sp\s+(.+)/i); if(!m) return [];
  const term = m[1].trim();
  if(spCache[term]) return spCache[term].map(t => ({icon:'media', title:t.name, subtitle:'♪ ' + t.artist, score:5500,
    action:() => { send('open:' + t.url); hide(); }}));
  clearTimeout(spTimer); spTimer = setTimeout(() => send('sp:' + term), 250);
  return [{icon:'media', title:'Searching Spotify…', subtitle:term, score:5500, action:() => {}}];
});

/* Notion search (async via Rust):  ns <query> */
let nsCache = {}, nsTimer;
window.pushNotion = (q, results) => { nsCache[q] = results || []; const cur = (input.value.match(/^(?:ns|notion)\s+(.+)/i) || [])[1]; if(cur && cur.trim() === q) search(); };
provider(q => {
  const m = q.match(/^(?:ns|notion)\s+(.+)/i); if(!m) return [];
  const term = m[1].trim();
  if(nsCache[term]) return nsCache[term].map(n => ({icon:'note', title:n.title, subtitle:'Notion page', score:5400,
    action:() => { send('open:' + n.url); hide(); }}));
  clearTimeout(nsTimer); nsTimer = setTimeout(() => send('ns:' + term), 250);
  return [{icon:'note', title:'Searching Notion…', subtitle:term, score:5400, action:() => {}}];
});

/* GitHub repos (from Rust):  repo <search> */
let repos = [];
window.pushRepos = list => { repos = list || []; };
provider(q => {
  const m = q.match(/^repo\s+(.+)/i); if(!m || !repos.length) return [];
  const term = m[1].trim();
  return repos.map(r => ({r, s: fscore(term, r.name)})).filter(x => x.s >= 0)
    .sort((x,y) => y.s - x.s).slice(0, 6)
    .map(x => ({icon:'github', title:x.r.name, subtitle:'GitHub repo', score:x.s + 100,
      action:() => { send('open:' + x.r.url); hide(); }}));
});

/* GitHub notifications (from Rust):  ghn */
let ghNotifs = [];
window.pushNotifs = list => { ghNotifs = list || []; };
provider(q => {
  if(!/^ghn$/i.test(q.trim())) return [];
  if(!ghNotifs.length) return [{icon:'bell', title:'No GitHub notifications', subtitle:'all caught up', score:5000, action:() => {}}];
  return ghNotifs.slice(0, 8).map(n => ({icon:'bell', title:n.name, subtitle:n.url.replace('https://github.com/', ''),
    score:5000, action:() => { send('open:' + n.url); hide(); }}));
});

/* browser bookmarks (from Rust):  bm <search> */
let bookmarks = [];
window.pushBookmarks = list => { bookmarks = list || []; };
provider(q => {
  const m = q.match(/^(?:bm|bookmark)\s+(.+)/i); if(!m || !bookmarks.length) return [];
  const term = m[1].trim();
  return bookmarks.map(b => ({b, s: fscore(term, b.name)})).filter(x => x.s >= 0)
    .sort((x,y) => y.s - x.s).slice(0, 6)
    .map(x => ({icon:'bookmark', title:x.b.name, subtitle:x.b.url, score:x.s + 100,
      action:() => { send('open:' + x.b.url); hide(); }}));
});

/* browser history (from Rust):  hist <search> */
let history = [];
window.pushHistory = list => { history = list || []; };
provider(q => {
  const m = q.match(/^(?:hist|his)\s+(.+)/i); if(!m || !history.length) return [];
  const term = m[1].trim();
  return history.map(h => ({h, s: fscore(term, h.name)})).filter(x => x.s >= 0)
    .sort((x,y) => y.s - x.s).slice(0, 6)
    .map(x => ({icon:'history', title:x.h.name, subtitle:x.h.url, score:x.s + 90,
      action:() => { send('open:' + x.h.url); hide(); }}));
});

/* Obsidian notes (from Rust):  obs <search> */
let obsNotes = [];
window.pushObsidian = list => { obsNotes = list || []; };
provider(q => {
  const m = q.match(/^(?:obs|obsidian)\s+(.+)/i); if(!m || !obsNotes.length) return [];
  const term = m[1].trim();
  return obsNotes.map(n => ({n, s: fscore(term, n.name)})).filter(x => x.s >= 0)
    .sort((x,y) => y.s - x.s).slice(0, 6)
    .map(x => ({icon:'note', title:x.n.name, subtitle:'Obsidian note', score:x.s + 100,
      action:() => { send('open:obsidian://open?path=' + encodeURIComponent(x.n.path)); hide(); }}));
});

/* shell runner:  >dir  (CMD)   $Get-Process  (PowerShell)  , with admin variants */
provider(q => {
  let mode, cmd;
  if(q.startsWith('>')){ mode = 'cmd'; cmd = q.slice(1).trim(); }
  else if(q.startsWith('$')){ mode = 'ps'; cmd = q.slice(1).trim(); }
  else return [];
  if(!cmd) return [];
  const label = mode === 'ps' ? 'PowerShell' : 'Command Prompt';
  return [
    {icon:'terminal', title:`Run: ${cmd}`, subtitle:label, score:5400, action:() => { send('shell:' + mode + ':' + cmd); hide(); }},
    {icon:'terminal', title:`Run as Admin: ${cmd}`, subtitle:label + '  ·  UAC', score:5395, action:() => { send('shell:' + mode + 'admin:' + cmd); hide(); }},
  ];
});

/* clipboard history (from Rust):  clip   or   clip <search> */
let clips = [];
window.pushClips = list => { clips = list || []; };
provider(q => {
  const m = q.match(/^(?:clip|cb)\s*(.*)/i); if(!m) return [];
  const arg = m[1].trim();
  if(/^(reset|clear)$/i.test(arg)) return [{icon:'clip', title:'Clear clipboard history', subtitle:'reset', score:5100, action:() => { send('clipreset'); hide(); }}];
  const term = arg.toLowerCase();
  return clips.map((c, i) => ({c, i})).filter(x => !term || (x.c.t || '').toLowerCase().includes(term)).slice(0, 8)
    .map(x => ({icon:'clip', title:(x.c.t || '').replace(/\s+/g, ' ').slice(0, 60) || '(blank)',
      subtitle:'from ' + (x.c.s || 'clipboard') + '  ·  Enter to restore', score:4700 - x.i,
      action:() => { send('clip:' + x.i); hide(); }}));
});

/* screen brightness (laptop panel via Rust):  bright 60 */
provider(q => {
  const m = q.match(/^(?:bright(?:ness)?)\s*(\d{1,3})$/i);
  if(!m) return [];
  const n = Math.min(100, +m[1]);
  return [{icon:'sun', title:`Brightness ${n}%`, subtitle:'laptop panel', score:5100, action:() => { send('bright:' + n); hide(); }}];
});

/* Steam games (list from Rust) */
let steamGames = [];
window.pushSteam = list => { steamGames = list || []; };
provider(q => {
  if(q.length < 2 || !steamGames.length) return [];
  return steamGames.map(g => ({g, s: fscore(q, g.name)})).filter(x => x.s >= 0)
    .sort((x,y) => y.s - x.s).slice(0, 5)
    .map(x => ({icon:'steam', title:x.g.name, subtitle:'Steam game', score:x.s + 150,
      action:() => { send('open:steam://rungameid/' + x.g.appid); hide(); }}));
});

/* ═══ render / input ═══ */
function search(){
  const q = input.value;
  results = [];
  for(const p of providers){ try { results.push(...p(q)); } catch(_){} }
  results.sort((a,b) => b.score - a.score);
  results = results.slice(0, 200);
  sel = 0; render();
}
function render(){
  const ul = $('#lresults'); ul.innerHTML = '';
  results.forEach((r,i) => {
    const li = document.createElement('li');
    if(i === sel) li.className = 'sel';
    const ic = r.img ? `<img class="licon-img" src="${r.img}">` : r.emoji ? `<span class="lemoji">${r.emoji}</span>` : svg(r.icon);
    li.innerHTML = `<span class="licon">${ic}</span><span class="ltext"><span class="ltitle">${he(r.title)}</span><span class="lsub">${he(r.subtitle||'')}</span></span>`;
    li.addEventListener('click', () => run(i));
    li.addEventListener('mousemove', () => { if(sel !== i){ sel = i; paint(); } });
    ul.appendChild(li);
  });
  fit();
}
function paint(){
  document.querySelectorAll('#lresults li').forEach((li,i) => li.classList.toggle('sel', i === sel));
  const el = document.querySelectorAll('#lresults li')[sel];
  if(el) el.scrollIntoView({block:'nearest'});
}
function run(i){ const r = results[i]; if(r && r.action) r.action(); }

input.addEventListener('input', search);
document.addEventListener('keydown', e => {
  if(e.key === 'Escape'){ hide(); e.preventDefault(); }
  else if(e.key === 'ArrowDown'){ sel = Math.min(sel+1, results.length-1); paint(); e.preventDefault(); }
  else if(e.key === 'ArrowUp'){ sel = Math.max(sel-1, 0); paint(); e.preventDefault(); }
  else if(e.key === 'Enter'){ run(sel); e.preventDefault(); }
});
document.addEventListener('mousedown', e => { if(!e.target.closest('#launcher')) hide(); });

/* ═══ helpers: calc + convert ═══ */
function calc(expr){
  const c = expr.trim(); if(!/\d/.test(c)) return null;
  const leftover = c.replace(/\b(sqrt|sin|cos|tan|asin|acos|atan|exp|abs|round|floor|ceil|min|max|pow|log|ln|pi|e|mod)\b/gi,'').replace(/[0-9+\-*/%^(). ,]/g,'');
  if(leftover.trim() !== '') return null;
  let e = c.replace(/\^/g,'**').replace(/\bmod\b/gi,'%')
    .replace(/\bpi\b/gi,'Math.PI').replace(/\bln\b/gi,'Math.log').replace(/\blog\b/gi,'Math.log10')
    .replace(/\b(sqrt|sin|cos|tan|asin|acos|atan|exp|abs|round|floor|ceil|min|max|pow)\b/gi, m => 'Math.' + m.toLowerCase())
    .replace(/\be\b/gi,'Math.E');
  try {
    const v = Function('"use strict";return (' + e + ')')();
    if(typeof v !== 'number' || !isFinite(v)) return null;
    return +parseFloat(v.toPrecision(12));
  } catch(_){ return null; }
}
const UNITS = { m:[1,'len'],km:[1000,'len'],cm:[0.01,'len'],mm:[0.001,'len'],mi:[1609.344,'len'],ft:[0.3048,'len'],inch:[0.0254,'len'],yd:[0.9144,'len'],
  kg:[1,'mass'],g:[0.001,'mass'],mg:[1e-6,'mass'],lb:[0.453592,'mass'],oz:[0.0283495,'mass'],
  b:[1,'data'],kb:[1024,'data'],mb:[1048576,'data'],gb:[1073741824,'data'],tb:[1099511627776,'data'],
  s:[1,'time'],min:[60,'time'],hr:[3600,'time'],day:[86400,'time'] };
function convert(q){
  const m = q.match(/^([\d.]+)\s*([a-z]+)\s*(?:to|in)\s*([a-z]+)$/i); if(!m) return null;
  const n = parseFloat(m[1]); let from = m[2].toLowerCase(), to = m[3].toLowerCase();
  const alias = { in:'inch', meters:'m', kilometers:'km', pounds:'lb', kilograms:'kg', grams:'g', minutes:'min', hours:'hr', seconds:'s', days:'day' };
  from = alias[from] || from; to = alias[to] || to;
  if(UNITS[from] && UNITS[to] && UNITS[from][1] === UNITS[to][1]){
    const out = +parseFloat((n * UNITS[from][0] / UNITS[to][0]).toPrecision(8));
    return { out:`${out} ${to}`, desc:`${n} ${from} = ${out} ${to}`, val:out };
  }
  const t = tempConv(n, from, to);
  if(t != null){ const out = +parseFloat(t.toPrecision(6)); return { out:`${out} °${to[0].toUpperCase()}`, desc:`${n}°${from[0].toUpperCase()} = ${out}°${to[0].toUpperCase()}`, val:out }; }
  return null;
}
function tempConv(n,f,t){ const M={c:'c',f:'f',k:'k',celsius:'c',fahrenheit:'f',kelvin:'k'}; f=M[f];t=M[t]; if(!f||!t) return null;
  const c = f==='c'?n : f==='f'?(n-32)*5/9 : n-273.15;
  return t==='c'?c : t==='f'?c*9/5+32 : c+273.15;
}

/* theme sync, re-read the shared accent (dashboard swatches write it) on every open */
function syncAccent(){
  const a = localStorage.getItem('bcAccent');
  if(a){ const R = document.documentElement.style; R.setProperty('--accent', a);
    R.setProperty('--accent-dim', localStorage.getItem('bcDim') || a);
    const [r,g,b] = [1,3,5].map(i => parseInt(a.slice(i,i+2),16));
    R.setProperty('--glow', `rgba(${r},${g},${b},.55)`); }
}
syncAccent();

/* from Rust */
window.pushApps = list => { apps = list || []; search(); };
window.onLauncherShow = function(){
  syncAccent();
  const l = document.getElementById('launcher');
  l.classList.remove('show'); void l.offsetWidth; l.classList.add('show');
  input.value = ''; search(); input.focus(); fit();
};
input.focus();
