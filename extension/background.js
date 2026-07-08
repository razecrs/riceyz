// Batcave Tabs Bridge, pushes open tabs to the host, polls for "switch to tab" commands.
const HOST = 'http://127.0.0.1:37421';

async function pushTabs() {
  try {
    const tabs = await chrome.tabs.query({});
    const data = tabs.map(t => ({ title: t.title, url: t.url, id: t.id, windowId: t.windowId }));
    await fetch(HOST + '/tabs', { method: 'POST', body: JSON.stringify(data) });
  } catch (e) { /* host not running */ }
}

async function pollActivate() {
  try {
    const r = await fetch(HOST + '/activate');
    const c = await r.json();
    if (c && typeof c.tabId === 'number' && c.tabId >= 0) {
      await chrome.tabs.update(c.tabId, { active: true });
      if (typeof c.windowId === 'number' && c.windowId >= 0) {
        await chrome.windows.update(c.windowId, { focused: true });
      }
    }
  } catch (e) { /* host not running */ }
}

// Continuous fetch activity also keeps the MV3 service worker alive.
setInterval(pushTabs, 2000);
setInterval(pollActivate, 800);
pushTabs();

chrome.tabs.onUpdated.addListener(pushTabs);
chrome.tabs.onRemoved.addListener(pushTabs);
chrome.tabs.onActivated.addListener(pushTabs);
chrome.tabs.onCreated.addListener(pushTabs);
