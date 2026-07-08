# Batcave, Engine ⟷ Skin Protocol

The Rust host is a **UI-agnostic engine**. It serves the HTML/CSS/JS in this folder over
the `bat://localhost/` custom protocol and talks to it through a small message contract.
**Swap the markup/styles/scripts freely**, the engine keeps working as long as the new
JS speaks this protocol.

## Surfaces (files the engine loads)

| Surface | File | Window |
|---|---|---|
| Dashboard (wallpaper) | `index.html` → `dashboard.js` + `style.css` | desktop layer, opaque |
| Launcher | `launcher.html` → `launcher.js` + `launcher.css` | top-center popup, opaque |
| Toast (over-app notif) | `toast.html` → `toast.js` + `toast.css` | small always-on-top, opaque |

> ⚠️ Opaque only, WebView2 transparency renders white on this machine.
> The served folder is the `root*` `PathBuf`s in `host/src/main.rs` (one place to change).

## Skin → Engine   (`window.ipc.postMessage("cmd:args")`)

Dashboard sends: `run:<chrome|terminal|files|steam|code>` · `open:<url>` · `media:<playpause|next|prev>` · `showtoast:<msg>` · `hidetoast`

Launcher sends:
| Command | Effect |
|---|---|
| `launch:<path>` | open an app / file |
| `open:<url-or-uri>` | shell-open a URL / `ms-settings:` / `steam://` / `obsidian://` |
| `launcher:hide` | hide the launcher |
| `resize:<px>` | size the launcher window to content height |
| `notif:<msg>` | fire a batarang notification |
| `sys:<cmd>` | run a raw system command (shutdown/lock/…) |
| `shell:<cmd\|ps\|cmdadmin\|psadmin>:<command>` | run a shell command (admin = UAC) |
| `media:<playpause\|next\|prev>` | SMTC transport |
| `vol:<0-100\|mute\|unmute>` | master volume |
| `appvol:<app>:<0-100>` | per-app volume |
| `bright:<0-100>` | laptop brightness |
| `clip:<index>` | restore a clipboard entry |
| `clipreset` | clear clipboard history |
| `sp:<query>` | Spotify search (async → `pushSpotify`) |
| `ns:<query>` | Notion search (async → `pushNotion`) |

## Engine → Skin   (engine calls these `window.*` functions; define them to receive data)

Dashboard: `pushStats({cpu,ram})` · `pushNowPlaying({title,artist,playing})` · `pushNotif(msg, "desktop"|"app")`

Launcher (all guarded with `&&`, so undefined = ignored):
| Function | Payload |
|---|---|
| `onLauncherShow()` | called each time it opens (reset + focus) |
| `pushApps([{name,path}])` | Start-Menu apps |
| `pushSteam([{appid,name}])` | Steam games |
| `pushObsidian([{name,path}])` | Obsidian notes |
| `pushBookmarks([{name,url}])` | browser bookmarks |
| `pushHistory([{name,url}])` | browser history |
| `pushClips([{t,s}])` | clipboard: text + source app |
| `pushRates({usd:1, eur:0.9, …})` | currency rates (USD base) |
| `pushRepos([{name,url}])` | GitHub repos |
| `pushNotifs([{name,url}])` | GitHub notifications |
| `pushSpotify(query, [{name,artist,url}])` | async Spotify results |
| `pushNotion(query, [{title,url}])` | async Notion results |

Toast: `playToast(msg)`

## Swapping the skin
1. Replace any surface's `*.html/*.css/*.js` with your own design.
2. In the new JS: call `window.ipc.postMessage(...)` with the commands above, and define
   the `window.push*` functions you want to consume.
3. Theme accent syncs via `localStorage['bcAccent']` / `['bcDim']` (shared across surfaces).
4. Restart the host. No Rust changes needed.
