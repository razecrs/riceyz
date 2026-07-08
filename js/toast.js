/* ===== Batcave Toast ===== */
const toast = document.getElementById('toast');

// Rust shows the window then calls this; hiding is handled by Rust (via the queue).
window.playToast = function(msg){
  document.getElementById('ttext').textContent = (msg || 'NOTIFICATION').toUpperCase();
  toast.classList.remove('show');
  void toast.offsetWidth;
  toast.classList.add('show');
};

// theme sync (shared bat://localhost localStorage)
(function(){
  const a = localStorage.getItem('bcAccent');
  if(a){
    document.documentElement.style.setProperty('--accent', a);
    const [r,g,b] = [1,3,5].map(i => parseInt(a.slice(i,i+2),16));
    document.documentElement.style.setProperty('--glow', `rgba(${r},${g},${b},.55)`);
  }
})();
