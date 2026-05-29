<div align="center">
  <img src="./app-icon.png" alt="Simple Password Manager" width="128" height="128">
  
  # Simple Password Manager

  **A modern, secure, and easy-to-use password manager that works with KeePass databases**

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app)
  [![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?style=for-the-badge&logo=svelte&logoColor=white)](https://svelte.dev)
  [![Vite](https://img.shields.io/badge/Vite-6-646CFF?style=for-the-badge&logo=vite&logoColor=white)](https://vitejs.dev)
  [![Rust](https://img.shields.io/badge/Rust-stable-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://rust-lang.org)

</div>

---

## What is Simple Password Manager?

Simple Password Manager helps you store all your passwords in one secure place. Instead of remembering dozens of passwords or using the same one everywhere (please don't!), you only need to remember one master password.

**Key benefits:**
- **Works offline** – Your passwords stay on your computer, never uploaded anywhere
- **No account needed** – Just download and use, no sign-up required
- **Compatible with KeePass** – Use your existing `.kdbx` files or create new ones
- **Works on Windows and Linux** (macOS builds paused while a Tauri upstream issue is resolved)

---

## Screenshots

<div align="center">
  <i>Coming soon</i>
</div>

---

## What's new in 2.0

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

### Core Features
- **Open or create KeePass databases** (KDBX 3 & 4 format)
- **Organize passwords into folders** (groups)
- **Search & jump** to anything with `Ctrl+K`
- **Generate strong passwords** with one click
- **Mark favorites** for quick access
- **Inline detail pane** — view, edit, and save without leaving the main window

### Security Features
- **AES-256 encryption** – industry-standard protection
- **Argon2id key derivation** – memory-hard, slows down brute-force attacks
- **Auto-lock** – automatically locks after inactivity
- **Clipboard auto-clear** – passwords are removed from clipboard after 30 seconds
- **Breach detection** – check if your passwords appeared in known data breaches (optional, opt-in)
- **Quick Unlock** – fast re-authentication for recently opened databases
- **Yubikey HMAC-SHA1 challenge-response** as a second factor (optional)
- **Windows Hello** quick unlock layered on top of the master password (Windows only)

### Quality of Life
- **Auto-save** – nothing is ever lost between session and disk
- **Light & Dark themes** – easy on the eyes, follows system by default
- **Drag & drop** – move entries between folders intuitively
- **69 built-in icons** – personalize your entries
- **Undo/Redo** – made a mistake? `Ctrl+Z` / `Ctrl+Y`
- **Password history** – see previous versions of each entry, restore in one click
- **Custom fields** – store additional information
- **Expiration dates** – get reminded when passwords need updating
- **Silent remote merge** – when the file is edited elsewhere, changes are merged automatically
- **System tray** – minimize to tray instead of closing
- **Browser extension hook** – connect a native messaging host for in-browser use

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

1. **Launch the app** – You'll see a welcome screen
2. **Create a new database** or **open an existing one** (`.kdbx` file)
3. **Set a strong master password** – This is the only password you need to remember!
4. **Start adding your passwords** – Click the `+` in the item list to add an entry

### Tips for Daily Use

- **Hit `Ctrl+K`** from anywhere to search and jump to entries, folders, or actions
- **Single-click** an entry to see its detail; click **Edit** to modify it
- **Drag and drop** entries onto folders to move them
- **Star** your most-used entries to find them in the Favorites view
- **Use the password generator** (wand icon in edit mode) when creating new entries

---

## Settings

Open Settings with the gear icon in the sidebar (or `Ctrl+,`). The dialog has its own internal nav:

### Appearance
Choose between **Light**, **Dark**, or **System** theme.

### Security
- **Auto-Lock Timer** – Lock the database after X seconds of inactivity (set to *Never* to disable)
- **Breach Detection (HIBP)** – Enable checking passwords against the "Have I Been Pwned" database. Uses k-anonymity — only partial hashes leave your machine.
- **Yubikey** – Enroll a Yubikey as a second factor for unlocking this database
- **Windows Hello** – Seal the master password behind Windows Hello for quick unlock (Windows only)

### Database
Saves and remote merges happen automatically. The path of the open database is shown for reference.

### Application
- **Close to Tray** – When you close the window, minimize to system tray instead of quitting.
- **Autostart** – Launch the app when you sign in.
- **Updates** – Check for and install the next version.
- **Browser extension** – Register the native messaging host for a browser extension.

---

## Dashboard

The Home view shows:

- **Total entries and folders** in your database
- **Health Score** – Overall security rating
- **Average password strength** – How strong are your passwords on average?
- **Security issues** – Weak, reused, old, or expired passwords that need attention
- **Breached passwords** – Passwords found in known data breaches (if HIBP is enabled)

---

## How It Keeps Your Data Safe

### Encryption (Two Layers)

Your database is protected by two security layers:

1. **Key Derivation (Argon2id)** – Your master password is transformed into an encryption key using Argon2id, a memory-hard algorithm that makes brute-force attacks extremely difficult. Settings: 64 MB memory, 2 iterations, 2 parallel threads.

2. **Database Encryption (AES-256)** – The actual data is encrypted with AES-256, the same standard used by governments and banks worldwide.

*Older KeePass databases may use AES-KDF instead of Argon2id. The app will warn you on open and offer to upgrade.*

### Other Security Measures

| What | How |
|------|-----|
| **Your passwords** | Never leave your computer – everything is stored locally |
| **Clipboard** | Automatically cleared 30 seconds after copying a password |
| **Memory** | Sensitive data is handled securely using Rust's `secrecy` library |
| **Network** | Zero internet connections (except optional breach checking and update checks) |

> ⚠️ **Important:** This software has not undergone a professional security audit. For mission-critical use, consider established solutions like [KeePass](https://keepass.info/) or [KeePassXC](https://keepassxc.org/).

---

## Frequently Asked Questions

**Q: What happens if I forget my master password?**  
A: There is no way to recover your data. Your master password is the only key. Write it down and store it somewhere safe!

**Q: Can I use this with my existing KeePass database?**  
A: Yes! This app fully supports KDBX 3 and KDBX 4 formats used by KeePass and KeePassXC.

**Q: Does this sync across devices?**  
A: Not directly. You can sync your `.kdbx` file using any cloud storage (Dropbox, Google Drive, etc.) and open it on multiple devices. The app detects external changes and merges them silently — no action needed on your end.

**Q: Is my data sent anywhere?**  
A: No. Everything stays on your computer. The only exceptions are the optional breach detection feature (sends partial password hashes — never actual passwords — to check against known breaches) and the optional update check.

**Q: Is this open source?**  
A: Yes! MIT licensed. Feel free to inspect the code, contribute, or fork it.

---

## Contributing

Contributions are welcome! Whether it's bug reports, feature requests, or code contributions – feel free to open an issue or pull request.

---

## Credits

**Created by Jonas Laux**

Built with:
- [Tauri](https://tauri.app) – For the native desktop experience
- [Svelte 5](https://svelte.dev) & [Vite](https://vitejs.dev) – For the user interface
- [bits-ui](https://bits-ui.com) – For accessible UI primitives
- [Tailwind CSS](https://tailwindcss.com) – For styling
- [Rust](https://rust-lang.org) – For the secure backend
- [keepass-rs](https://crates.io/crates/keepass) – For KeePass database handling

---

## License

MIT License – See [LICENSE](LICENSE) file for details.

*This project is not affiliated with or endorsed by the official KeePass project.*
