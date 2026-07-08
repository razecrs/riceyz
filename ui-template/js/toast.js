// The host shows this window and calls playToast(msg) with the notification text.
window.playToast = msg => { document.getElementById('toast').textContent = msg; };
