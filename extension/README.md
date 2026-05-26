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

Two equivalent paths:

**A. From inside the desktop app (recommended).** Click the extension icon
once — the popup will show your extension ID with a Copy button. Paste it
into the app's **Settings → Application → Browser Extension** card and
click *Register for all detected browsers*. The app probes every
Chromium-based browser plus Firefox on your machine and registers the
native host for each one in a single shot.

**B. From the command line.**

```bash
cd extension
node scripts/install-host.mjs --extension-id <id-from-browser>
# To remove later:
node scripts/install-host.mjs --uninstall
```

The script auto-detects Chrome, Edge, Brave, Vivaldi, Opera, Chromium,
Arc, and Firefox. On Windows it writes the registry key as well as the
JSON manifest. Re-running is idempotent.

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

## Save-on-submit flow

When you submit a login form on any page, the content script captures the
filled-in username + password, the background worker stashes them in
`chrome.storage.session`, and the extension icon gets a `+` badge. Open the
popup and a "Save this login?" banner sits at the top — pre-filled with
domain, username, and the captured password. Click *Save to vault* and the
desktop app writes a new entry to the root group and persists the database
to disk.

Heuristics: capture fires on real `<form>` submissions, on Enter inside a
password field, and on clicks of buttons whose label matches "log in / sign
in / sign up / submit / anmelden / einloggen / registrieren". Adjust in
`src/content/index.ts` if a site you use doesn't trigger it.

## TOTP

If an entry has a KeePassXC-style `otp` custom field — either an
`otpauth://totp/...` URI or a raw Base32 secret — the popup shows the
current 6-digit code in the entry's expanded view, with a countdown ring
that auto-refreshes when the code rolls over. Only HMAC-SHA1 is supported
(matches KeePassXC and every real-world site).

To create one in the desktop app: open the entry, add a custom field named
`otp` and paste the `otpauth://` URI you got from the site's "set up 2FA"
flow. The popup picks it up on next load.

## Sign-up vs sign-in awareness

The content script scores each capture as `login` or `signup` based on
several heuristics — number of password fields (two means "confirm
password" = signup), nearby button text, URL path, field-name hints like
`firstname`. The inline banner then says either "Save this login?" or
"Save this new account?" so the wording matches the user's intent.

If a guess is wrong it doesn't break anything — the save itself is
identical for both flavours.

## Inline page banner

The save flow is two-pronged. Right after a form submit the content script
injects a small banner directly into the host page (via Shadow DOM so the
host's CSS can't deface it). The user can save, dismiss, or pick *Never on
this site* — which adds the domain to a blocklist (`chrome.storage.local`)
that the content script consults on subsequent captures.

If the form submission causes a full page navigation and the inline banner
gets torn down before the user reacts, the BG-stashed capture surfaces on
the next page load: when a fresh content script boots, it checks
`chrome.storage.session` and resurrects the banner on the destination page
(only if origin matches and the capture is < 60 s old). The popup banner
is the final fallback in case the next page is somehow off-origin.

## Known gaps (will land in 3.x follow-ups)

- No HTTP Basic Auth interception.
- No per-domain approval — currently any entry whose URL matches the active
  domain is returned to the extension on request.
- Saved entries always land in the root group; choosing a destination group
  from the popup is not yet implemented (move it later in the desktop app).
