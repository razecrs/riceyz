# 🦇 Batcave

A from-scratch Windows desktop shell, a **live, clickable wallpaper**, a **Flow-Launcher-class launcher**, and **real notifications rendered as a batarang flying across your screen**. One small Rust process drives it all; the UI is plain HTML/CSS/JS, so you can reskin any of it.

Built with [`wry`](https://github.com/tauri-apps/wry) (WebView2) + [`tao`](https://github.com/tauri-apps/tao) + the `windows` crate. No Electron.

## Heads up before you run this

- **It hides your taskbar.** While the host runs it keeps the Windows taskbar hidden (re-hides it every ~1.5s). That is on purpose, the rice *is* your desktop.
- **It's for keyboard gremlins.** No taskbar means you get around with the `Alt+Space` launcher and shortcuts. If you live on the taskbar with a mouse, this will feel wrong.
- **Getting your taskbar back:** kill `host.exe` and disable or delete the `BatcaveHost` scheduled task. It comes right back (a sign-out or explorer restart if it is being stubborn).

## What you get

**Wallpaper / dashboard**, reparented onto the desktop layer (WorkerW/Progman), clickable via a low-level mouse hook. Live CPU/RAM/GPU, clock, weather, a **real WASAPI-loopback audio visualizer** (actual FFT, not fake bars), an SMTC music player, theme swatches, a power menu, quick toggles, and a one-click UI reload.

**Launcher** (`Alt+Space`), apps (with real extracted icons), calculator, unit/currency conversion, **your own Everything-style file search** (reads the NTFS MFT directly, live-updated via the USN journal), Steam, GitHub, Notion, Spotify, browser bookmarks/history/open-tabs, clipboard history, a process killer, a colour tool, media/volume/brightness, shell commands, and a `theme` command. Extensible with **plugins in any language** (see [`PLUGINS.md`](PLUGINS.md)).

**Notifications**, hooks the real Windows notification stream (`UserNotificationListener`) and replays each one as a cinematic batarang on the wallpaper (or a compact toast over apps).

## Layout

```
host/            Rust engine (the whole backend + Win32 glue)
  src/main.rs    window setup, surfaces, the event loop, IPC routing
  src/*.rs       focused modules: mft, viz, audio, browser, github, plugins, …
index.html …     dashboard surface (index/dashboard.js/style.css)
launcher.html …  launcher surface
toast.html …     over-app toast surface
extension/       browser tabs bridge (load unpacked)
plugins/         drop-in launcher plugins
```

The engine ↔ skin contract (IPC verbs + `window.push*` callbacks) is documented in [`PROTOCOL.md`](PROTOCOL.md), swap the HTML/CSS/JS freely.

## Build & run

```sh
cd host
cargo run                # debug (has a console for logs)
cargo build --release    # optimized, no console window
```

The **file search reads the raw NTFS volume, so it needs the host to run elevated.** For a silent elevated auto-start on logon, register a Scheduled Task with *Run with highest privileges* pointing at the built exe.

## Config

`config.json` in the repo root:

```json
{ "file_search": false }
```

`file_search: false` = **lite mode**, skips the MFT index and drops host RAM from ~750 MB to ~70 MB (nice for 8 GB laptops). Toggle it live from the launcher with `fs`. Defaults to on.

## API keys

GitHub / Spotify / Notion features read `tokens.json` (git-ignored). Copy `tokens.example.json` → `tokens.json` and fill it in. GitHub can reuse `gh auth token`.

## Notes

- WebView2 renders transparency as solid white on some setups, so every surface is opaque by design.
- Alt+Tab replacement was attempted and dropped, Windows 11 won't let a low-level hook suppress the native switcher.
