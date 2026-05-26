# Simple Password Manager — Browser Extension

Companion extension. Talks to the running desktop app over a loopback HTTP
bridge protected by a per-session bearer token. The token is discovered via
a tiny native-messaging host that the desktop app installs.

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full design.

## Development workflow

### Prerequisites

- Desktop app running (build with `npm run tauri:dev` from the repo root)
- Chrome, Edge or Firefox

### Build the extension

```bash
cd extension
npm install
npm run build
```

Output goes to `dist/`. That folder is what you load into the browser.

### Build the native messaging host

```bash
cd ../src-tauri
cargo build --bin browser-bridge-host
```

The binary appears at `src-tauri/target/debug/browser-bridge-host(.exe)`.

### Load the extension (unpacked)

1. Chrome / Edge: open `chrome://extensions` (or `edge://extensions`),
   enable Developer Mode, click "Load unpacked", select `extension/dist`.
2. Firefox: open `about:debugging#/runtime/this-firefox`, click "Load
   Temporary Add-on", select `extension/dist/manifest.json`.

Copy the **extension ID** that Chrome/Edge assigns (Firefox shows a UUID).

### Register the native messaging host

```bash
cd extension
node scripts/install-host.mjs --extension-id <id-from-browser>
```

This writes the native messaging manifest to the per-user directory each
browser scans, and on Windows also creates the matching registry key. It
re-runs cleanly (overwrites existing entries).

Restart any open browser windows after registration.

### Verify

1. With the desktop app running and a database unlocked, click the extension
   icon. You should see the database name in the popup header.
2. Open a page like `github.com` (or whichever domain you have entries for).
3. Entries matching the domain show up. Hit "Fill" — username + password
   are written into the form fields and the popup closes.

## Layout

```
extension/
  ARCHITECTURE.md             ← design decisions
  manifest.json               ← MV3 manifest
  popup.html
  postcss.config.js
  tailwind.config.js
  tsconfig.json
  vite.config.ts
  scripts/
    install-host.mjs          ← native host registration
  src/
    common/bridge.ts          ← shared types
    background/index.ts       ← service worker, owns the bridge token
    content/index.ts          ← injected on every page, performs the fill
    popup/
      index.tsx               ← React entry
      Popup.tsx               ← UI
      bridge-client.ts        ← popup → background → bridge wrapper
      useActiveDomain.ts      ← reads the active tab's hostname
      styles.css              ← Tailwind directives
```

## Security notes

- The token never leaves the background service worker (popup talks to it
  via `chrome.runtime.sendMessage`, not directly to the bridge).
- The bridge listens only on `127.0.0.1` and CORS is locked to
  `chrome-extension://*` and `moz-extension://*`.
- `bridge.json` is written with mode `0600` on Unix; on Windows it sits in
  per-user `%LOCALAPPDATA%` which already restricts access.
- A future revision will gate every entry behind an in-app "approve domain"
  prompt the first time a new domain asks.

## Known gaps (will land in 3.x follow-ups)

- No auto-detection of saves — if you sign up somewhere, the extension does
  not yet offer to save the credentials. Use the desktop app.
- No TOTP integration.
- No HTTP Basic Auth interception.
- No per-domain approval — currently any entry whose URL matches the active
  domain is returned to the extension on request.
