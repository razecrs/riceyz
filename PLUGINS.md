# Batcave Launcher, Plugin API

Write a plugin in **any language**, the host talks to it over **line-delimited JSON on stdio**.
No build step, no linking against us.

## Install
Drop a folder in `plugins/`:
```
plugins/
  my-plugin/
    plugin.json
    <your code>
```
Restart the host. `plugins` in the launcher lists what's loaded.

## `plugin.json`
```json
{
  "name": "My Plugin",
  "trigger": "gh",          // prefix that activates it; "" = every query
  "command": "node index.js" // run from the plugin's folder
}
```
`command` can be anything on PATH: `node index.js`, `python main.py`, `./my-plugin.exe`, …

## Protocol
The host launches your process once and keeps it alive. Per query it writes **one JSON line** to
your **stdin**:
```json
{"query": "cats"}
```
(If your `trigger` is `"gh"` and the user typed `gh cats`, you receive `{"query":"cats"}`, the
trigger is stripped.)

Reply with **one JSON line** on **stdout**:
```json
{"results": [
  {"title": "Search for cats", "subtitle": "opens Google", "action": "open:https://google.com/search?q=cats"},
  {"title": "Copy 'cats'",     "subtitle": "to clipboard",  "action": "copy:cats"}
]}
```

## Result fields
| Field | Meaning |
|---|---|
| `title` | main line (required) |
| `subtitle` | secondary line |
| `action` | what runs when the user hits Enter, any launcher **IPC verb** |

## Action verbs (what you can trigger)
`open:<url-or-path>` · `launch:<path>` · `shell:<cmd\|ps\|cmdadmin\|psadmin>:<command>` ·
`sys:<raw cmd>` · `copy:<text>` · `vol:<0-100\|mute>` · `bright:<0-100>` · `media:<playpause\|next\|prev>` · `notif:<msg>`

## Example (Node)
See `plugins/sample/`, trigger `echo`, e.g. `echo hello world`.

## Notes
- Print exactly one JSON line per query; keep it fast (the host reads one line back, blocking).
- Log/debug to **stderr** (stdout is the protocol channel).
- Crashes are isolated, a broken plugin won't take down the launcher.
