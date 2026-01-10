<div align="center">
  <img src="./app-icon.png" alt="Simple Password Manager" width="128" height="128">
  
  # Simple Password Manager

  **A modern, secure, and easy-to-use password manager that works with KeePass databases**

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
  [![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app)
  [![Next.js](https://img.shields.io/badge/Next.js-16-000000?style=for-the-badge&logo=next.js&logoColor=white)](https://nextjs.org)
  [![Rust](https://img.shields.io/badge/Rust-stable-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://rust-lang.org)
  [![React](https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=black)](https://react.dev)

</div>

---

## What is Simple Password Manager?

Simple Password Manager helps you store all your passwords in one secure place. Instead of remembering dozens of passwords or using the same one everywhere (please don't!), you only need to remember one master password.

**Key benefits:**
- **Works offline** – Your passwords stay on your computer, never uploaded anywhere
- **No account needed** – Just download and use, no sign-up required
- **Compatible with KeePass** – Use your existing `.kdbx` files or create new ones
- **Works on Windows, macOS, and Linux**

---

## Screenshots

<div align="center">
  <i>Coming soon</i>
</div>

---

## Features at a Glance

### Core Features
- **Open or create KeePass databases** (KDBX 3 & 4 format)
- **Organize passwords into folders** (groups)
- **Search** across all your entries instantly
- **Generate strong passwords** with one click
- **Mark favorites** for quick access
- **Edit entries in separate windows** – work on multiple at once

### Security Features
- **AES-256 encryption** – industry-standard protection
- **Auto-lock** – automatically locks after inactivity
- **Clipboard auto-clear** – passwords are removed from clipboard after 30 seconds
- **Breach detection** – check if your passwords appeared in known data breaches (optional)
- **Quick Unlock** – fast re-authentication for recently opened databases

### Quality of Life
- **Light & Dark themes** – easy on the eyes
- **Drag & drop** – organize entries and folders intuitively  
- **69 built-in icons** – personalize your entries
- **Undo/Redo** – made a mistake? No problem
- **Password history** – see previous versions of each entry
- **Custom fields** – store additional information
- **Expiration dates** – get reminded when passwords need updating
- **Live updates** – automatically detect external changes to your database
- **System tray** – minimize to tray instead of closing

---

## Keyboard Shortcuts

| Shortcut | What it does |
|----------|--------------|
| `Ctrl+S` (Win/Linux) / `Cmd+S` (Mac) | Save your database |
| `Ctrl+N` / `Cmd+N` | Create a new database |
| `Ctrl+F` / `Cmd+F` | Open/close search |
| `Ctrl+Z` / `Cmd+Z` | Undo last action |
| `Ctrl+Y` / `Cmd+Shift+Z` | Redo |
| `Ctrl+Alt+W` / `Cmd+Alt+W` | Close the current database |
| `Escape` | Close search |
| `Enter` | Unlock database (on password screen) |

---

## Getting Started

### Download & Install

**Option 1: Download a Release** (Recommended)
1. Go to the [Releases](https://github.com/jonax1337/Password-Manager/releases) page
2. Download the installer for your operating system
3. Run the installer and follow the prompts

**Option 2: Build from Source** (For developers)

Prerequisites:
- [Node.js](https://nodejs.org/) v20 or newer
- [Rust](https://rustup.rs/) (latest stable)
- Platform-specific requirements: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
git clone https://github.com/jonax1337/Password-Manager.git
cd Password-Manager
npm install
npm run tauri:dev     # Development mode
npm run tauri:build   # Production build
```

### First Steps

1. **Launch the app** – You'll see a welcome screen
2. **Create a new database** or **open an existing one** (`.kdbx` file)
3. **Set a strong master password** – This is the only password you need to remember!
4. **Start adding your passwords** – Click the `+` button or right-click for options

### Tips for Daily Use

- **Double-click** an entry to edit it in a new window
- **Right-click** anywhere for quick actions
- **Drag and drop** entries between folders to organize them
- **Star** your most-used entries to find them quickly in the Favorites view
- **Use the password generator** (wand icon) when creating new entries

---

## Settings

Access settings via the gear icon in the title bar.

### Appearance
Choose between **Light**, **Dark**, or **System** theme.

### Security
- **Auto-Lock Timer** – Lock the database after X seconds of inactivity (set to 0 to disable)
- **Breach Detection** – Enable checking passwords against the "Have I Been Pwned" database. This uses a privacy-safe method (k-anonymity) where only partial hashes are sent.

### Database
- **Live Updates** – When enabled, the app automatically detects and merges changes if someone else edits the same database file.

### Application
- **Close to Tray** – When you click X, minimize to system tray instead of quitting.

---

## Dashboard

When you open a database, you'll see a **Dashboard** with:

- **Total entries and groups** in your database
- **Health Score** – An overall security rating
- **Average password strength** – How strong are your passwords on average?
- **Security issues** – Weak, reused, old, or expired passwords that need attention
- **Breached passwords** – Passwords found in known data breaches (if enabled)

---

## How It Keeps Your Data Safe

### Encryption (Two Layers)

Your database is protected by two security layers:

1. **Key Derivation (Argon2id)** – Your master password is transformed into an encryption key using Argon2id, a memory-hard algorithm that makes brute-force attacks extremely difficult. Settings: 64 MB memory, 2 iterations, 2 parallel threads.

2. **Database Encryption (AES-256)** – The actual data is encrypted with AES-256, the same standard used by governments and banks worldwide.

*Older KeePass databases may use AES-KDF instead of Argon2id. The app will warn you and offer to upgrade.*

### Other Security Measures

| What | How |
|------|-----|
| **Your passwords** | Never leave your computer – everything is stored locally |
| **Clipboard** | Automatically cleared 30 seconds after copying a password |
| **Memory** | Sensitive data is handled securely using Rust's `secrecy` library |
| **Network** | Zero internet connections (except optional breach checking) |

> ⚠️ **Important:** This software has not undergone a professional security audit. For mission-critical use, consider established solutions like [KeePass](https://keepass.info/) or [KeePassXC](https://keepassxc.org/).

---

## Frequently Asked Questions

**Q: What happens if I forget my master password?**  
A: There is no way to recover your data. Your master password is the only key. Write it down and store it somewhere safe!

**Q: Can I use this with my existing KeePass database?**  
A: Yes! This app fully supports KDBX 3 and KDBX 4 formats used by KeePass and KeePassXC.

**Q: Does this sync across devices?**  
A: Not directly. You can sync your `.kdbx` file using any cloud storage (Dropbox, Google Drive, etc.) and open it on multiple devices. Enable "Live Updates" in settings to auto-merge changes.

**Q: Is my data sent anywhere?**  
A: No. Everything stays on your computer. The only exception is the optional breach detection feature, which sends partial password hashes (not actual passwords) to check against known breaches.

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
- [Next.js](https://nextjs.org) & [React](https://react.dev) – For the user interface
- [Rust](https://rust-lang.org) – For the secure backend
- [keepass-rs](https://crates.io/crates/keepass) – For KeePass database handling

---

## License

MIT License – See [LICENSE](LICENSE) file for details.

*This project is not affiliated with or endorsed by the official KeePass project.*
