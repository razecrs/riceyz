# Batcave Skin Template

A starter skin for the [Batcave](../) desktop shell. Fork it, restyle it, publish it. No server, no sign-up, GitHub is the whole marketplace.

## What a skin is

Three HTML surfaces plus their css/js. That is it. The Rust engine serves your files and talks to them over a small message contract (documented in [`PROTOCOL.md`](../PROTOCOL.md)):

| File | Surface |
|------|---------|
| `index.html` | the wallpaper / dashboard |
| `launcher.html` | the `Alt+Space` launcher |
| `toast.html` | the over-app notification |

The host **calls your `window.push*()` functions** with data (stats, now-playing, apps, notifications, audio bars) and **you send it commands** with `window.ipc.postMessage("verb:args")`. See `js/dashboard.js` and `js/launcher.js` here for the minimal version of each.

## Rules (read these, it is short)

1. **Surfaces must be opaque.** WebView2 renders transparency as solid white on some machines, so give every surface a real background.
2. **You must ship all three** files (`index.html`, `launcher.html`, `toast.html`), even if a surface is nearly empty.
3. Only define the `push*` callbacks you actually use. The host guards every call, so ignoring one is fine.
4. Full verb + callback list lives in [`PROTOCOL.md`](../PROTOCOL.md). Do not invent new ones, the host only knows those.

## Try it

Point the engine at your skin folder in `config.json`:

```json
{ "ui": "R:\\path\\to\\your-skin" }
```

Restart the host. That is it, your skin is live.

## Publish it

1. Push your skin to its own GitHub repo.
2. Add the topic **`batcave-ui`** to the repo (Settings, or the gear next to About).
3. Done. It now shows up at [github.com/topics/batcave-ui](https://github.com/topics/batcave-ui) alongside everyone else's.

That topic page **is** the marketplace. No central list to get added to, no server to run. Anyone browses the topic, clones a skin, points `config.json` at it.
