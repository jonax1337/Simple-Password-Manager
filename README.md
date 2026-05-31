<div align="center">
  <img src="./app-icon.png" alt="Simple Password Manager" width="128" height="128">
  
  # Simple Password Manager

  **A modern, secure, end-to-end-encrypted password manager that bridges KeePass and 1Password — local files, cloud-only vaults, or both.**

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app)
  [![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=for-the-badge&logo=svelte&logoColor=white)](https://svelte.dev)
  [![Vite](https://img.shields.io/badge/Vite-6-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vitejs.dev)
  [![Rust](https://img.shields.io/badge/Rust-stable-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://rust-lang.org)

</div>

---

## What is Simple Password Manager?

Two products in one binary:

- **The KeePass client you already know** — open a `.kdbx` file, type your master password, you're in. Works fully offline, no account needed.
- **An optional E2E-encrypted cloud sync** — link an account on your self-hosted server and your vault lives encrypted in the cloud instead of on disk. Add more vaults, share them with other accounts (Owner / Editor / Reader), unlock with Windows Hello, manage everything from a browser dashboard.

Pick the model that fits, or mix them — local `.kdbx` files and cloud vaults sit side-by-side in the same unlock screen.

**Key benefits:**
- **Works offline** when you want it to — local databases never leave your machine.
- **Works as a cloud product** when you want it to — your own server, no third party.
- **Compatible with KeePass** — KDBX 3 & 4 format. Open existing files, write back with KeePassXC.
- **End-to-end encrypted** — the server only ever sees opaque ciphertext; your master password never leaves your device.
- **Works on Windows and Linux** (macOS builds paused while a Tauri upstream issue is resolved).

---

## Screenshots

<div align="center">
  <i>Coming soon</i>
</div>

---

## What's new

### 2.1 — Cloud, multi-vault, sharing, Hello (May 2026)

- **Self-hostable sync server** (`server/`). Axum + SQLite + JWT, auto-TLS via Caddy, one-command Docker Compose deploy. Zero env-vars needed for local dev (JWT secret + DB path auto-generated).
- **Multi-vault per account** — Personal, Work, Family… one cloud account holds as many vaults as you like.
- **Vault sharing with E2E key wrap** — invite another account, they get the vault wrapped with their own Curve25519 keypair. Server can't read it.
- **Permission levels** — Owner / Editor / Reader. Readers see a "RO" badge and write attempts are blocked client- and server-side.
- **Members dialog** — manage who has access, change roles, revoke. Reachable from Settings → Cloud Sync → Manage members.
- **Cloud-only vaults are RAM-only** — when you sign in via the cloud, the KDBX never lands on disk. App-close = bytes gone, server is the source of truth.
- **Vault switcher in the sidebar** — dropdown to swap vaults mid-session. Passwords cached per session so switching back is one click.
- **Windows Hello for cloud** — enroll once after a login, next launch is a single Hello prompt and you're in your default vault automatically.
- **Recovery codes** — printed once at signup. Forgotten master password? Type the code + a new password and the vault is back.
- **Web dashboard** (`dashboard/`) — same indigo design system as the desktop app. List, rename, delete vaults from any browser. Argon2id runs client-side via `hash-wasm` so the master password never leaves the tab.
- **Per-IP rate limiting** on `/auth/*` (5 burst, 1 token / 6s) via `tower_governor`.

### 2.0 — Svelte rewrite (May 2026)

The whole frontend was rebuilt from React/Next.js to **Svelte 5 + Vite** with a 1Password-inspired three-pane layout.

- **Three-pane layout** — sidebar / item list / detail pane, all resizable.
- **Inline detail view with read/edit toggle** — instead of popping out a new window.
- **Auto-save** — every change flushes to disk in milliseconds. No more Ctrl+S.
- **Command palette** (`Ctrl+K`) — jump to any folder, entry, or action.
- **Settings as a dialog** — modal with internal nav, no separate window.
- **Indigo-violet palette** keyed to the app icon, in light and dark mode.
- **Drag & drop restored** — move entries between folders, reorganise the tree.
- **Live updates everywhere** — moving an entry, changing an icon, or a remote merge all reflect instantly without refreshing.

---

## Features at a Glance

### Core (local-file mode)
- **Open or create KeePass databases** (KDBX 3 & 4 format)
- **Organize passwords into folders** (groups)
- **Search & jump** to anything with `Ctrl+K`
- **Generate strong passwords** with one click
- **Mark favorites** for quick access
- **Inline detail pane** — view, edit, and save without leaving the main window

### Cloud (optional, via your own server)
- **Sign in to your cloud account** from the unlock screen (alongside local file open)
- **Multi-vault per account** — Personal, Work, Family, …
- **Vault picker dropdown** in the sidebar, switch mid-session
- **Share vaults** with other accounts using sealed-box E2E key wrap
- **Owner / Editor / Reader** permissions, enforced both client- and server-side
- **Members dialog** — invite, promote, demote, revoke
- **Recovery codes** for forgotten master passwords
- **Auto-push after every change** — the cloud is always in sync
- **No local file** in cloud mode — vault lives only in RAM + on the server

### Security Features
- **AES-256 encryption** for the KDBX layer
- **AES-256-GCM** wrapping ciphertext for cloud transport
- **Argon2id key derivation** (64 MiB / 3 iter / 4 lanes) — memory-hard, slow brute-force
- **Curve25519 sealed-box** for vault sharing — only the recipient can unwrap
- **Auto-lock** after inactivity
- **Clipboard auto-clear** after 30 seconds
- **Breach detection** via HIBP (opt-in, k-anonymity, no plaintext leaves your machine)
- **Quick Unlock** — fast re-authentication for recently opened databases
- **Yubikey HMAC-SHA1 challenge-response** as a second factor (optional, local only)
- **Windows Hello** unlocks **both** local databases and cloud accounts (Windows only)
- **Per-IP rate limiting** on server `/auth/*` endpoints

### Quality of Life
- **Auto-save** — nothing is ever lost between session and disk
- **Auto-push** on cloud-linked vaults — every edit lands on the server
- **Remember-Me** — cloud session rehydrates at app launch (Windows Credential Manager + DPAPI)
- **Light & Dark themes** — easy on the eyes, follows system by default
- **Drag & drop** — move entries between folders intuitively
- **69 built-in icons** — personalize your entries
- **Undo/Redo** — made a mistake? `Ctrl+Z` / `Ctrl+Y`
- **Password history** — see previous versions of each entry, restore in one click
- **Custom fields** — store additional information
- **Expiration dates** — get reminded when passwords need updating
- **Silent remote merge** — when a shared file is edited elsewhere, changes are merged automatically
- **Per-entry conflict dialog** when both sides changed the same entry
- **`<db>.kdbx.lock`** file (KeePass-compatible) with 10-minute stale detection
- **System tray** — minimize to tray instead of closing
- **Browser extension hook** — connect a native messaging host for in-browser use

---

## Keyboard Shortcuts

| Shortcut | What it does |
|----------|--------------|
| `Ctrl+K` / `Cmd+K` | Open the command palette (search anything, run actions) |
| `Ctrl+,` / `Cmd+,` | Open Settings |
| `Ctrl+Z` / `Cmd+Z` | Undo last action |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Enter` | Unlock database (on password screen) |
| `Esc` | Close the command palette or current dialog |

Saving is automatic — no shortcut needed.

---

## Getting Started

### Download & Install

**Option 1: Download a Release** (Recommended)
1. Go to the [Releases](https://github.com/jonax1337/Simple-Password-Manager/releases) page
2. Download the installer for your operating system
3. Run the installer and follow the prompts

**Option 2: Build from Source** (For developers)

Prerequisites:
- [Node.js](https://nodejs.org/) v20 or newer
- [Rust](https://rustup.rs/) (latest stable)
- Platform-specific requirements: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/jonax1337/Simple-Password-Manager.git
cd Simple-Password-Manager
npm install
npm run tauri:dev     # Development mode
npm run tauri:build   # Production build
```

### First Steps

**Local-file mode** (no account needed):

1. **Launch the app** — You'll see the welcome screen with two tabs: *Local file* and *Cloud account*
2. **Stay on the Local file tab**
3. **Create a new database** or **open an existing one** (`.kdbx` file)
4. **Set a strong master password** — This is the only password you need to remember
5. **Start adding your passwords** — Click the `+` in the item list to add an entry

**Cloud mode** (requires a running server — see [`server/`](./server/README.md)):

1. **Launch the server** — `cd server && cargo run --release` (no env vars needed for dev)
2. **Open the desktop app** → Local file tab → create a temporary `.kdbx`
3. **Settings (`Ctrl+,`) → Cloud Sync → Create account** — point at `http://localhost:8090`, pick a username + master password
4. **Save the recovery code** the dialog shows you (only chance!)
5. **Lock** (`Ctrl+L`) → unlock screen → **Cloud account** tab → sign in
6. **Pick your vault** → enter the KDBX password → you're in
7. From now on the vault lives entirely in the cloud + RAM. No more local file.

### Tips for Daily Use

- **Hit `Ctrl+K`** from anywhere to search and jump to entries, folders, or actions
- **Click the vault header** in the sidebar to swap between cloud vaults (or create a new one)
- **Single-click** an entry to see its detail; click **Edit** to modify it
- **Drag and drop** entries onto folders to move them
- **Star** your most-used entries to find them in the Favorites view
- **Use the password generator** (wand icon in edit mode) when creating new entries
- **Enroll Windows Hello** after your first cloud login — the next launch is one tap

---

## Settings

Open Settings with the gear icon in the sidebar (or `Ctrl+,`). The dialog has its own internal nav:

### Appearance
Choose between **Light**, **Dark**, or **System** theme.

### Security
- **Auto-Lock Timer** — Lock the database after X seconds of inactivity (set to *Never* to disable)
- **Breach Detection (HIBP)** — Enable checking passwords against the "Have I Been Pwned" database. Uses k-anonymity — only partial hashes leave your machine.
- **Yubikey** — Enroll a Yubikey as a second factor for unlocking this database (local-file mode only)
- **Windows Hello** — Seal the master password behind Windows Hello for quick unlock (Windows only)

### Database
Saves and remote merges happen automatically. The path of the open database is shown for reference. Cloud-only vaults show their name instead.

### Cloud Sync
- **Link this vault to a server** — Create an account or sign in to one
- **Push / Pull** — Manual sync buttons (push runs automatically on every save when linked)
- **Manage members** — Owner-only dialog: invite, change roles, revoke access
- **Disconnect** — Drop the session (data on the server stays untouched)

### Application
- **Close to Tray** — When you close the window, minimize to system tray instead of quitting.
- **Autostart** — Launch the app when you sign in.
- **Updates** — Check for and install the next version.
- **Browser extension** — Register the native messaging host for a browser extension.

---

## Dashboard

The Home view in the desktop app shows:

- **Total entries and folders** in your database
- **Health Score** — Overall security rating
- **Average password strength** — How strong are your passwords on average?
- **Security issues** — Weak, reused, old, or expired passwords that need attention
- **Breached passwords** — Passwords found in known data breaches (if HIBP is enabled)

There's also a **web admin dashboard** at `https://yourdomain/dashboard/` (or `http://localhost:5173` in dev) when running the cloud server. It exposes vault metadata only — see [`dashboard/`](./dashboard/README.md).

---

## How It Keeps Your Data Safe

### Local-file mode (KeePass-compatible)

Your database is protected by two security layers:

1. **Key Derivation (Argon2id)** — Your master password is transformed into an encryption key using Argon2id, a memory-hard algorithm that makes brute-force attacks extremely difficult. Settings: 64 MB memory, 2 iterations, 2 parallel threads.

2. **Database Encryption (AES-256)** — The actual data is encrypted with AES-256, the same standard used by governments and banks worldwide.

*Older KeePass databases may use AES-KDF instead of Argon2id. The app will warn you on open and offer to upgrade.*

### Cloud mode (E2E-encrypted)

Add a third layer on top of the local KeePass crypto:

3. **Account key derivation** — A separate Argon2id pass on the cloud master password yields a `password_key`. The 32-byte `master_key` is **random** at signup; `password_key` only wraps it. Changing the cloud password re-wraps `master_key`, no vault re-encryption.

4. **AES-256-GCM wrap** of the KDBX bytes with a `vault_key` (also random, also wrapped under `master_key`). This is what hits the server. The server can't decrypt it.

5. **Vault sharing** uses Curve25519 sealed-box: each recipient's account_pubkey wraps a copy of `vault_key`. The sender never needs the recipient's password.

6. **Recovery code** is a 20-character Crockford base32 string shown once at signup. It wraps `master_key` server-side; entering it lets you set a new cloud password without losing the vault.

### Other Security Measures

| What | How |
|------|-----|
| **Your master password** | Never leaves your computer — Argon2id runs in the desktop or in the browser |
| **Clipboard** | Automatically cleared 30 seconds after copying a password |
| **Memory** | Sensitive data is handled securely using Rust's `secrecy` library + `zeroize` |
| **Network** | Zero connections in local-file mode (except optional breach checking and update checks). Cloud mode only talks to your server. |
| **Cloud-only vaults** | Never written to disk. App-close = bytes gone. |
| **Rate limiting** | Server throttles `/auth/*` per IP (5 burst, ~10/min steady) |
| **Lock files** | `<db>.kdbx.lock` (KeePass-style) for shared local files; ETag CAS for cloud writes |

> ⚠️ **Important:** This software has not undergone a professional security audit. For mission-critical use, consider established solutions like [KeePass](https://keepass.info/), [KeePassXC](https://keepassxc.org/), or [1Password](https://1password.com/).

---

## Frequently Asked Questions

**Q: What happens if I forget my master password?**
A: For **local databases**: there is no recovery — your master password is the only key. For **cloud accounts**: the recovery code shown once at signup lets you reset. If you lost both, the vault is unrecoverable.

**Q: Can I use this with my existing KeePass database?**
A: Yes! This app fully supports KDBX 3 and KDBX 4 formats used by KeePass and KeePassXC.

**Q: Does this sync across devices?**
A: Two ways:
- **File-level sync** (KeePassXC-style): Drop your `.kdbx` into Dropbox / OneDrive / Nextcloud / Syncthing. The app's `notify`-based file watcher reloads on external changes and merges automatically.
- **E2E cloud sync** (1Password-style): Run [`server/`](./server/README.md) on a VPS, sign in from the desktop. Multi-vault, sharing, web dashboard.

**Q: Is my data sent anywhere?**
A: In local-file mode: no. Optional exceptions: breach detection (sends partial password hashes — never actual passwords) and update check.
In cloud mode: only to **your own server**. The server only sees opaque AES-GCM ciphertext.

**Q: Is this open source?**
A: Yes! MIT licensed. Feel free to inspect the code, contribute, or fork it.

**Q: Can I host the cloud server for my family/team?**
A: Yes, that's the supported model. One account per user, share vaults across accounts. See [`server/README.md`](./server/README.md) for the Docker-Compose deploy walkthrough with auto-TLS.

**Q: What's missing / not production-grade yet?**
A: The cloud half is a working MVP but has known gaps — see [`ROADMAP.md`](./ROADMAP.md). Notably: no per-control disabled state for readers in every editor surface (top-level handlers block writes), no audit log viewer, no mobile clients, no 2FA on the cloud account itself.

---

## Contributing

Contributions are welcome! Whether it's bug reports, feature requests, or code contributions — feel free to open an issue or pull request.

For the code layout:

| Directory | What |
|-----------|------|
| `src/` | Svelte 5 frontend (unlock screens, main app, components, UI primitives) |
| `src-tauri/` | Rust backend (KDBX layer, cloud crypto, Tauri commands, browser-extension bridge) |
| `server/` | Self-hostable cloud-sync server (Axum + SQLite + JWT) |
| `dashboard/` | Web admin SPA (Svelte 5 + Tailwind v4) |
| `extension/` | Browser extension (Manifest V3) |

---

## Credits

**Created by Jonas Laux**

Built with:
- [Tauri](https://tauri.app) — For the native desktop experience
- [Svelte 5](https://svelte.dev) & [Vite](https://vitejs.dev) — For the user interface
- [Axum](https://docs.rs/axum) — For the cloud sync server
- [bits-ui](https://bits-ui.com) — For accessible UI primitives
- [Tailwind CSS](https://tailwindcss.com) — For styling
- [Rust](https://rust-lang.org) — For the secure backend
- [keepass-rs](https://crates.io/crates/keepass) — For KeePass database handling
- [crypto_box](https://crates.io/crates/crypto_box) — For Curve25519 sealed-box vault sharing
- [hash-wasm](https://github.com/Daninet/hash-wasm) — For browser-side Argon2id

---

## License

MIT License — See [LICENSE](LICENSE) file for details.

*This project is not affiliated with or endorsed by the official KeePass project.*
