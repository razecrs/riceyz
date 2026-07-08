/* ===== Batcave Dashboard, live brain ===== */
const $ = s => document.querySelector(s);
const clamp = (n,a,b)=>Math.max(a,Math.min(b,n));
let liveData = false;   // flips true when Lively feeds real stats

/* ---- CLOCK · GREETING · DATE ---- */
function tick(){
  const d = new Date();
  let h = d.getHours();
  const m = String(d.getMinutes()).padStart(2,'0');
  const ampm = h>=12 ? 'PM' : 'AM';
  const h12 = h%12 || 12;
  $('#time').textContent = String(h12).padStart(2,'0')+':'+m;
  $('#ampm').textContent = ampm;

  const part = h<12 ? 'GOOD MORNING' : h<18 ? 'GOOD AFTERNOON' : 'GOOD EVENING';
  $('#greet').textContent = `${part}, MASTER WAYNE.`;

  const days = ['SUN','MON','TUE','WED','THU','FRI','SAT'];
  const mons = ['JAN','FEB','MAR','APR','MAY','JUN','JUL','AUG','SEP','OCT','NOV','DEC'];
  const ord  = n => { const s=['TH','ST','ND','RD'], v=n%100; return n+(s[(v-20)%10]||s[v]||s[0]); };
  $('#dnum').textContent = String(d.getDate()).padStart(2,'0');
  $('#dday').textContent = days[d.getDay()];
  $('#dmon').textContent = mons[d.getMonth()]+' '+ord(d.getDate());
}
setInterval(tick,1000); tick();

/* ---- SYSTEM STATS (Lively pushes these; demo animates if not) ---- */
function setStat(id,val){
  val = clamp(Math.round(val),0,100);
  const el = document.getElementById(id), bar = document.getElementById(id+'Bar');
  if(el)  el.textContent = val+'%';
  if(bar) bar.style.width = val+'%';
  if(id==='gpu'){ const pg=document.getElementById('pgpu'); if(pg) pg.textContent=val; }
}

/* ---- TOP PERF OVERLAY: render FPS + frametime (real, via rAF) ---- */
let _frames=0, _last=performance.now();
function perfLoop(now){
  _frames++;
  const dt = now - _last;
  if(dt >= 500){
    const fps = Math.round(_frames*1000/dt);
    const f = document.getElementById('fps'), mt = document.getElementById('ftime');
    if(f)  f.textContent  = fps;
    if(mt) mt.textContent = (1000/Math.max(fps,1)).toFixed(1);
    _frames=0; _last=now;
  }
  requestAnimationFrame(perfLoop);
}
requestAnimationFrame(perfLoop);
// GPU temp: placeholder until Electron backend (systeminformation) feeds it live
document.getElementById('ptemp').textContent = '--';

/* ---- ELECTRON: real live hardware (systeminformation via preload) ---- */
if (window.batcave && window.batcave.onStats) {
  window.batcave.onStats(d => {
    liveData = true;
    if (d.cpu != null) setStat('cpu', d.cpu);
    if (d.ram != null) { setStat('ram', d.ram); $('#ramBig').textContent = Math.round(d.ram); }
    if (d.gpu != null) setStat('gpu', d.gpu);
    const temp = d.gpuTemp ?? d.cpuTemp;
    if (temp != null) document.getElementById('ptemp').textContent = Math.round(temp);
    // live C: disk usage on the dial
    const c = (d.disks || []).find(x => /^C:/i.test(x.mount));
    if (c && c.use != null) $('#disk').textContent = Math.round(c.use);
  });
}
// Rust host (WebView2) pushes stats here via evaluate_script
window.pushStats = function(d){
  liveData = true;
  if (d.cpu != null) setStat('cpu', d.cpu);
  if (d.ram != null) { setStat('ram', d.ram); $('#ramBig').textContent = Math.round(d.ram); }
  if (d.gpu != null) setStat('gpu', d.gpu);
  const temp = d.gpuTemp ?? d.cpuTemp;
  if (temp != null) document.getElementById('ptemp').textContent = Math.round(temp);
};
// Lively hook: fires when "System Data" is enabled in Lively settings
window.livelySystemInformation = function(data){
  try{
    const d = typeof data==='string' ? JSON.parse(data) : data;
    liveData = true;
    setStat('cpu', d.CurrentCpu);
    setStat('ram', d.CurrentRam);
    setStat('gpu', d.CurrentGpu);
    $('#ramBig').textContent = Math.round(d.CurrentRam);
  }catch(e){}
};
// demo animation (browser preview / before Lively data arrives)
let t = 0;
setInterval(()=>{
  if(liveData) return;
  t += 0.05;
  const cpu = 18 + 20*Math.abs(Math.sin(t))       + Math.random()*5;
  const ram = 40 +  9*Math.sin(t*0.6);
  const gpu = 12 + 28*Math.abs(Math.sin(t*0.4));
  setStat('cpu',cpu); setStat('ram',ram); setStat('gpu',gpu);
  $('#ramBig').textContent = Math.round(ram);
},450);

$('#disk').textContent = '48';   // static for now (Lively has no disk feed)

/* ---- WEATHER (keyless, CORS-friendly: ipapi + open-meteo) ---- */
const wcode = c => ({0:'CLEAR',1:'MAINLY CLEAR',2:'PARTLY CLOUDY',3:'OVERCAST',45:'FOG',48:'FOG',
  51:'DRIZZLE',53:'DRIZZLE',55:'DRIZZLE',61:'RAIN',63:'RAIN',65:'HEAVY RAIN',71:'SNOW',73:'SNOW',
  80:'SHOWERS',81:'SHOWERS',95:'STORM',96:'STORM'}[c] || '--');
async function weather(){
  try{
    const loc = await (await fetch('https://ipapi.co/json/')).json();
    $('#wloc').textContent = ((loc.city||'GOTHAM')+', '+(loc.country_code||'')).toUpperCase();
    const u = `https://api.open-meteo.com/v1/forecast?latitude=${loc.latitude}&longitude=${loc.longitude}&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code`;
    const c = (await (await fetch(u)).json()).current;
    $('#wtemp').textContent = Math.round(c.temperature_2m)+'°';
    $('#wdesc').textContent = wcode(c.weather_code);
    $('#wwind').textContent = Math.round(c.wind_speed_10m)+' KM/H';
    $('#whum').textContent  = c.relative_humidity_2m+'%';
  }catch(e){
    $('#wloc').textContent='GOTHAM CITY'; $('#wtemp').textContent='14°';
    $('#wdesc').textContent='CLEAR'; $('#wwind').textContent='6 KM/H'; $('#whum').textContent='71%';
  }
}
weather(); setInterval(weather,600000);

/* ---- HARDWARE SPECS (auto-detected by the host, written to specs.json) ---- */
function applySpecs(s) {
  const set = (id,v)=>{ const e=document.getElementById(id); if(e) e.textContent=v; };
  if (s.cpu) set('hwCpu', (s.cpu.match(/i\d-\S+/)?.[0] || s.cpu) + `  ${s.cores}C/${s.threads}T`);
  if (s.gpu) set('hwGpu', (s.gpu||'').replace(/NVIDIA GeForce |NVIDIA /,''));
  if (s.ramGB) set('hwRam', `${Math.round(s.ramGB)}GB · ${s.ramSpeed}MHz`);
  if (s.disks && s.disks.length > 0) {
    const ssd = s.disks.find(d=>/SN\d|NVMe|SSD/i.test(d.model||'')) || s.disks[s.disks.length-1];
    const hdd = s.disks.find(d=>d!==ssd) || s.disks[0];
    if(ssd) set('hwSsd', (ssd.model.match(/SN\d+/)?'WD '+ssd.model.match(/SN\d+/)[0] : (ssd.model || '').split(/[\s-]/)[0]) + ` ${Math.round(ssd.sizeGB)}GB`);
    if(hdd) set('hwHdd', (/ST\d/.test(hdd.model)?'Seagate' : (hdd.model || '').split(/[\s-]/)[0]) + ` ${Math.round(hdd.sizeGB)}GB`);
  }
}

// the host regenerates specs.json on boot, so read it now and again once it lands
function loadSpecs(){ fetch('specs.json?_=' + Date.now()).then(r=>r.json()).then(applySpecs).catch(()=>{}); }
loadSpecs();
setTimeout(loadSpecs, 3000);

const eye = document.getElementById('eye');
eye.addEventListener('click', ()=>{
  document.getElementById('hw').classList.toggle('incognito');
  eye.classList.toggle('off');
});

/* ---- INTERACTIVE LAUNCHERS & LINKS (Rust IPC) ---- */
function bcIpc(msg){ if(window.ipc) window.ipc.postMessage(msg); }

/* quick dock: toggles (open:) + power (power:) + expandable power menu */
document.querySelectorAll('.qbtn[data-open]').forEach(b => b.onclick = () => bcIpc('open:' + b.dataset.open));
{ const qr = document.getElementById('qreload'); if(qr) qr.onclick = () => bcIpc('reload'); }
document.querySelectorAll('[data-power]').forEach(b => b.onclick = () => bcIpc('power:' + b.dataset.power));
(function(){
  const qp = document.getElementById('qpower'), qm = document.getElementById('qmenu');
  if(qp && qm){
    qp.onclick = e => { e.stopPropagation(); qm.classList.toggle('show'); };
    document.addEventListener('click', () => qm.classList.remove('show'));
  }
})();

document.querySelectorAll('.launchers .tab').forEach(tab => {
  tab.addEventListener('click', () => {
    bcIpc('run:' + tab.textContent.trim().toLowerCase());   // CODE -> our editor; rest -> apps
  });
});

const linkUrls = {
  GMAIL:'https://mail.google.com', YOUTUBE:'https://youtube.com',
  GITHUB:'https://github.com', REDDIT:'https://reddit.com', DISCORD:'https://discord.com'
};
document.querySelectorAll('.links li').forEach(li => {
  li.addEventListener('click', () => {
    const url = linkUrls[li.textContent.trim()];
    if(url) bcIpc('open:' + url);
  });
});

/* ---- THEME SWITCHER (accent: blue / yellow / red / white) ---- */
const _root = document.documentElement;
function _hexRGB(hex){ const h=hex.replace('#',''); const n=h.length===3?h.split('').map(x=>x+x).join(''):h;
  return [parseInt(n.slice(0,2),16),parseInt(n.slice(2,4),16),parseInt(n.slice(4,6),16)]; }
function applyAccent(accent, dim){
  const [r,g,b] = _hexRGB(accent);
  _root.style.setProperty('--accent', accent);
  _root.style.setProperty('--accent-dim', dim);
  _root.style.setProperty('--glow', `rgba(${r},${g},${b},.55)`);
  window._accentRGB = [r,g,b];
}
document.querySelectorAll('.swatch').forEach(s=>{
  s.addEventListener('click', ()=>{
    applyAccent(s.dataset.accent, s.dataset.dim);
    document.querySelectorAll('.swatch').forEach(x=>x.classList.remove('active'));
    s.classList.add('active');
    localStorage.setItem('bcAccent', s.dataset.accent);
    localStorage.setItem('bcDim', s.dataset.dim);
  });
});
(function restoreTheme(){
  const a=localStorage.getItem('bcAccent'), d=localStorage.getItem('bcDim');
  if(a && d){ applyAccent(a,d); document.querySelector(`.swatch[data-accent="${a}"]`)?.classList.add('active'); }
  else { window._accentRGB=[27,157,255]; document.querySelector('.swatch')?.classList.add('active'); }
})();

/* ---- BATARANG NOTIFICATION (fired from launcher via Rust IPC) ---- */
const NOTIF_FULL_MS = 5700;      // full cinematic length
const NOTIF_FAST_MS = 1450;      // quick flyby length
const NOTIF_TOAST_MS = 2400;     // over-app toast length
const NOTIF_COOLDOWN = 60000;    // full cinematic only once per minute
let notifQueue = [], notifBusy = false, lastFullAt = 0;

// Rust routes here with mode 'desktop' (cinematic) or 'app' (toast).
window.pushNotif = function(msg, mode){
  notifQueue.push({msg: msg || 'TEST NOTIFICATION', mode: mode || 'desktop'});
  pumpNotif();
};
window.playNotif = function(msg){ window.pushNotif(msg, 'desktop'); }; // legacy alias

function pumpNotif(){
  if(notifBusy || notifQueue.length === 0) return;
  notifBusy = true;
  const {msg, mode} = notifQueue.shift();

  // --- over an app: fire the opaque toast window (Rust owns it) ---
  if(mode === 'app'){
    if(window.ipc) window.ipc.postMessage('showtoast:' + msg);
    setTimeout(() => {
      notifBusy = false;
      if(notifQueue.length === 0){ if(window.ipc) window.ipc.postMessage('hidetoast'); }
      else setTimeout(pumpNotif, 150);
    }, NOTIF_TOAST_MS);
    return;
  }

  // --- on the desktop: wallpaper cinematic (full 1st/min, fast after) ---
  const bn = document.getElementById('batnotif');
  const app = document.getElementById('app');
  if(!bn){ notifBusy = false; return; }
  const now = Date.now();
  const full = (now - lastFullAt) > NOTIF_COOLDOWN;
  if(full) lastFullAt = now;

  document.getElementById('bntext').textContent = msg.toUpperCase();
  bn.classList.remove('play', 'play-fast'); void bn.offsetWidth;
  bn.classList.add(full ? 'play' : 'play-fast');
  if(full && app) app.classList.add('bnfocus');

  setTimeout(() => {
    bn.classList.remove('play', 'play-fast');
    if(app) app.classList.remove('bnfocus');
    notifBusy = false;
    setTimeout(pumpNotif, 150);
  }, full ? NOTIF_FULL_MS : NOTIF_FAST_MS);
}

/* ---- MUSIC PLAYER (SMTC now-playing + controls via Rust IPC) ---- */
window.pushNowPlaying = function(d){
  const t=document.getElementById('mtitle'), a=document.getElementById('martist'), pl=document.getElementById('mplay');
  if(!t) return;
  t.textContent = d.title ? d.title.toUpperCase() : 'NOTHING PLAYING';
  a.textContent = d.artist ? d.artist.toUpperCase() : '';
  if(pl) pl.textContent = d.playing ? '⏸' : '▶';
};
document.querySelectorAll('.mctl button').forEach(b=>{
  b.addEventListener('click', ()=>{
    if(window.ipc && window.ipc.postMessage) window.ipc.postMessage('media:'+b.dataset.m);
  });
});

/* ---- AUDIO VISUALIZER (Lively audio spectrum; demo waves otherwise) ---- */
const cv = $('#viz'), cx = cv.getContext('2d');
let spectrum = new Array(128).fill(0);
window.livelyAudioListener = arr => { spectrum = arr; liveData = liveData || true; };
// Real WASAPI-loopback FFT bars from the Rust host (48 log-spaced bands, 0..1).
window.pushViz = arr => { spectrum = arr; };
function drawViz(){
  cx.clearRect(0,0,cv.width,cv.height);
  const n = 48, w = cv.width/n;
  const [ar,ag,ab] = window._accentRGB || [27,157,255];
  for(let i=0;i<n;i++){
    let v = spectrum[i] || 0;
    if(!v){ v = Math.abs(Math.sin(Date.now()/220 + i*0.5)) * 0.45 * (0.4+Math.random()*0.6); }
    const h = clamp(v*cv.height, 1, cv.height);
    cx.fillStyle = `rgba(${ar},${ag},${ab},${0.35 + v})`;
    cx.fillRect(i*w, cv.height-h, w-2, h);
  }
  requestAnimationFrame(drawViz);
}
drawViz();
