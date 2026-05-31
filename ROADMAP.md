# Roadmap — From KeePass client to full password suite

Stand: 2026-05-31. Ziel war "von lokalem KeePass-Client zu vollwertiger Password-Suite (Desktop + Browser + Sync)" — der Großteil davon ist jetzt drin. Nächste Session: **Phase 6 (UI polish + Fixes)**, danach Folge-Stages für Production-Hardening.

## Status

- **Phase 1 (Quick Wins)** ✅ — Close-to-Tray, Autostart, Update-Workflow, signed Bundles.
- **v2.0 (Mai 2026)** ✅ — kompletter Frontend-Rewrite von Next.js/React → **Svelte 5 + Vite**.
- **Phase 2 (Performance)** ✅ — Profiling abgeschlossen.
- **Phase 3 (Browser Extension)** ✅ — KeePassXC-kompatibler Native-Messaging-Host, Bridge-HTTP-Server, Auto-Fill/Save-Prompt MVP.
- **Phase 4 (Cloud-Ordner-Sync)** ✅ — `notify`-File-Watcher, `<db>.kdbx.lock` mit Stale-Detection, Sync-Status-Pill in der Sidebar, Per-Eintrag-Konflikt-Dialog.
- **Phase 5 (Eigener Sync-Server)** ✅ — alle Stages A–E + Cloud-only-Mode + Hello-Integration. Siehe unten und [`server/README.md`](./server/README.md).
- **Phase 6 (UI polish + Fixes)** ⏭️ in Arbeit — siehe Punkte unten.

Die alten Phase-1-Notizen unten haben Dateipfade aus der React-Ära (`components/Settings.tsx`, `lib/storage.ts` etc.). Die Funktionen leben jetzt unter `src/lib/` und `src/lib/components/`.

---

## Phase 4 — Cloud-Ordner-Sync (✅ shipped)

Datei-basierter Sync ohne eigenen Server (KeePassXC-Modell). Wenn das KDBX in Dropbox / OneDrive / Nextcloud liegt:

- `src-tauri/src/commands/database.rs` — `notify`-Crate watcht den Parent-Ordner, debounced 250 ms. Emittiert `database-external-change` an das Frontend, das `checkDatabaseChanges` + ggf. `merge_database` ruft.
- `src-tauri/src/lockfile.rs` — KeePass-Style `<db>.kdbx.lock` mit `{pid, host, acquired_at}` JSON. 10-Minuten-Stale-Detection klaut fremde Locks von gecrashten Peers. Save-Pfad acquired / released um jedes Write.
- `src/lib/components/Sidebar.svelte` — Sync-Pill: `Saving…` / `Merging…` / `Syncing cloud…` / `Synced 5s ago` / `Sync needs attention`.
- `src/lib/components/ConflictResolutionDialog.svelte` — Per-Eintrag-Diff (lokal vs. remote), Keep-local / Keep-remote pro Eintrag, "Keep all local/remote"-Shortcuts.

Tests: 126 ✅ (inkl. lockfile-Suite mit acquire/release/stale/peek).

---

## Phase 5 — Eigener Sync-Server (✅ shipped, MVP)

End-to-end-verschlüsselter Sync mit eigenem Server. Server ist self-hostable, Multi-Vault, mit Sharing, Permission-Levels und Web-Dashboard.

### Stage A — Server Multi-Vault Schema & Endpoints

`server/src/storage.rs` — drei Tabellen:

```sql
users         (id, email, server_auth_hash, kdf_salt_b64,
               wrapped_master_key_b64, recovery_blob_b64,
               account_pubkey_b64, wrapped_account_privkey_b64, created_at)
vaults        (id, name, owner_user_id, ciphertext_b64, etag,
               created_at, updated_at)
vault_members (vault_id, user_id, role, wrapped_vault_key_b64,
               invited_at, accepted_at)
```

`server/src/main.rs` + `vault.rs` + `auth.rs` exponieren:

| Method | Path                                  | Auth   | Notes |
|--------|---------------------------------------|--------|-------|
| POST   | `/auth/signup`                        | none   | Generates first vault inline |
| POST   | `/auth/login`                         | none   | Returns kdf_salt + wrapped_master_key + wrapped_account_privkey |
| POST   | `/auth/kdf-params`                    | none   | Public — anyone can fetch salt |
| POST   | `/auth/recovery-init`                 | none   | Returns recovery_blob for a given email |
| POST   | `/auth/reset`                         | none   | Swap credentials after recovery |
| POST   | `/users/lookup`                       | Bearer | Resolve email → user_id + account_pubkey |
| GET    | `/vaults`                             | Bearer | List caller's vaults |
| POST   | `/vaults`                             | Bearer | Create new vault |
| GET    | `/vaults/{id}`                        | Bearer | Single vault + caller's wrapped key |
| PUT    | `/vaults/{id}`                        | Bearer | CAS update via expected_etag |
| PATCH  | `/vaults/{id}`                        | Bearer | Rename (owner) |
| DELETE | `/vaults/{id}`                        | Bearer | Delete (owner) |
| POST   | `/vaults/{id}/share`                  | Bearer | Add/update member with sealed key |
| DELETE | `/vaults/{id}/share`                  | Bearer | Revoke member |
| GET    | `/vaults/{id}/members`                | Bearer | List members + roles |
| PATCH  | `/vaults/{id}/members/{user_id}`      | Bearer | Change a member's role (owner) |

Zero-config Boot: `JWT_SECRET` + `DB_PATH` auto-generieren beim ersten Start, persistieren im OS data dir. Per-IP-Rate-Limit auf `/auth/*` via `tower_governor` (5 burst, 1 token / 6s) mit `SmartIpKeyExtractor` für Caddy-Setup.

### Stage B — Client Multi-Vault + Cloud-Login auf Unlock-Screen

`src-tauri/src/cloud/` — neuer Modul-Baum:

```
cloud/
├── crypto.rs    Argon2id KDF, AES-GCM wrap/unwrap, sealed-box (NaCl-style)
├── client.rs    Typed reqwest wrapper für alle Endpoints
└── session.rs   CloudSession + ActiveVault state
```

Wrapped-master-key Architektur:

```
master_password
    │ Argon2id (m=64MiB, t=3, p=4, client-side)
    ▼
password_key (32B, used only to wrap/unwrap master_key)
    │
    ▼
master_key (random at signup, never derived) ──┐
    │                                          │
    ├─ SHA256(··· || "auth")  ──► server_auth_hash
    │
    ├─ SHA256(··· || "vault") ──► vault_key (random) ──► AES-GCM(kdbx_bytes)
    │
    └─ X25519 keypair ──► account_pubkey (server-side, shareable)
                          account_privkey (wrapped under master_key)
```

`src/routes/UnlockScreen.svelte` bekommt einen Tab-Switcher *Local file / Cloud account*. `CloudUnlockTab.svelte` lädt Vault-Picker nach Login, KDBX-PW pro Vault via Modal (kein `window.prompt`-Hijack mehr).

### Stage C — Vault Sharing mit E2E Key Wrap

Curve25519-Keypair pro Account, beim Signup generiert. Sealed-Box (hand-rolled über `crypto_box::ChaChaBox` weil 0.9 keine top-level `seal()` mehr exportiert) wrapt den `vault_key` für jeden Empfänger einzeln. Server speichert nur die opaken Blobs.

`POST /vaults/{id}/share` nimmt `{recipient_user_id, wrapped_vault_key_b64, role}` — server-side validiert dass Caller `Owner` ist. Beim Empfänger-Login wird das wrapped vault_key via `sealed_open` mit dem eigenen account-keypair entschlüsselt.

### Stage D — Permission Levels (Owner / Editor / Reader)

Server enforced:
- **Owner**: CRUD, share/unshare, role-change, rename, delete vault
- **Editor**: read + PUT vault
- **Reader**: nur read

Client gates:
- `appState.cloudVaultRole` + `isReadOnly` getter
- `Sidebar.svelte` + `VaultSwitcher.svelte` zeigen "RO"-Badge
- `MainApp.handleNewEntry` / `handleSave` early-return mit Toast wenn read-only
- `VaultMembersDialog.svelte` — Owner-only Role-Dropdown + Revoke + Invite

### Stage E — Web Dashboard (`dashboard/`)

Vanilla Svelte 5 + Vite + Tailwind v4 SPA. Selbe oklch Indigo-Violett-Palette wie die Desktop-App. `hash-wasm` für browser-seitiges Argon2id; Web Crypto API für AES-GCM unwrap. Aktuell: Login, Vault-Listing, Rename/Delete für Owner. Servierbar als statische Files via Caddy (handle-path `/dashboard/*` im Caddyfile).

### Stage F — Cloud-only Mode (✅ shipped, Bonus)

Auf Wunsch nachgeschoben: Cloud-Vaults landen **nicht** mehr als Datei auf Disk.

- `Database.path` ist jetzt `Option<PathBuf>` — `None` = cloud-only
- `Database::open_from_bytes` + `save_to_bytes` + `create_in_memory` neue APIs ohne FS-Touch
- `cloud_open_vault(vault_id, kdbx_password)` lädt direkt in `state.database`
- Save-Pfad: in cloud-mode → `save_to_bytes` + `cloud_push`, kein `fs::write`
- File-Watcher + lockfile in cloud-mode aus (kein File da)
- App-Close = Bytes weg, Server ist Source of Truth

Plus Vault-Switcher: Dropdown im Sidebar-Header, KDBX-PWs werden pro Session gecached (`appState.vaultPasswords`) damit Switch-back nicht erneut promptet.

### Hello-Integration für Cloud

`hello.rs` erweitert um `hello_cloud_{store,retrieve,clear,is_enrolled}`. Bundle ist opaque JSON:

```json
{
  "server_url": "...",
  "email": "...",
  "cloud_password": "...",
  "default_vault_id": "...",
  "vault_passwords": { "vault_id": "kdbx_pw", ... }
}
```

Hello-Prompt → entschlüssele Bundle → `cloud_login` + alle Vault-PWs in Session-Cache laden + default-Vault öffnen. User landet mit **einem Tap** in seinem Vault. Bundle wird beim erstmaligen Login mit `Remember Me` angeboten, oder manuell im Vault-Picker via "Enable one-tap unlock with Windows Hello"-Button.

### Remember Me Auto-Rehydrate

`cloud_persist_session` schreibt Master-Key + Token in den Windows Credential Manager unter einem well-known `DEFAULT_TARGET`. `cloud_rehydrate_session` wird beim App-Start (`Shell.svelte init()`) gerufen — wenn Session da, springt Unlock-Screen direkt zum Cloud-Tab mit pre-fetched Vault-Liste und Highlight auf der zuletzt geöffneten Vault.

### Recovery Codes

20-Zeichen Crockford-Base32 (16 random bytes, ohne 0/O/I/L). Beim Signup einmal angezeigt. Argon2id-Salt = `SHA256("recovery:" || email)` damit der Code allein zur Recovery reicht. `cloud_recover(email, code, new_password)` wrappt `master_key` mit neuem `password_key` und überschreibt server-side via `/auth/reset`.

---

## Phase 6 — UI Polish + Fixes (⏭️ nächste Session)

Was funktioniert, was noch schöner werden muss:

### Bekannte Lücken aus Phase 5

1. **Per-control disable im Reader-Mode** — Top-level Handler (`handleNewEntry`, `handleSave`) blocken Writes, aber die Buttons in `EntryList`, `EntryEditor`, `GroupTree` sind noch klickbar und geben dann erst den Toast. Sauberer: alle Edit-Affordances visuell disabled when `appState.isReadOnly`.
2. **Recovery-Code-Anzeige nach Signup** — kommt aktuell nur als Toast/Console-Log durch. Braucht einen eigenen Modal mit "Copy" + "Download as .txt" + "Print" + großem Warning-Banner ("Save this NOW, you won't see it again").
3. **Conflict-Dialog Politur** — die Per-Entry-Diff-View funktioniert aber sieht funktional aus, könnte mit Field-Labels + Color-Coded-Diff polished werden.
4. **Sync-Pill Animationen** — momentan harte Zustandswechsel. Smoother Cross-Fade zwischen `Saving` → `Syncing cloud` → `Synced` wäre nett.
5. **Dashboard Native Dialogs** — Rename/Delete nutzen aktuell `window.prompt`/`window.confirm`. Sollten richtige Dialoge sein (analog zum Desktop).
6. **Members-Dialog** — Layout passt, aber "Invite" Form-Feedback ist minimal. Plus: Pending invitations (accepted_at == null) hervorheben.
7. **Settings → Cloud Sync** Layout — könnte aufgeräumter werden mit echten Sections + Icons.
8. **VaultSwitcher** — beim öffnen ist die Liste momentan einen Tick spät da (race zwischen onMount-loadList und open-event). Funktioniert aber leichter Flicker.
9. **CloudUnlockTab Phase Picker** — wenn user den Back-Button drückt, wird die Form geleert; sollte den Username im Field behalten.
10. **Empty-state des Dashboards** — wenn 0 Vaults, sieht's leer aus. Empty-State-Card mit Call-to-Action ("Create your first vault in the desktop app").

### Production-Hardening (für später)

- **Recovery-Code regenerieren** — UI um alten Code zu invalidieren und neuen zu drucken
- **Cloud-Account löschen** — `DELETE /auth/account` plus Confirmation-Dialog
- **Cloud-Account-PW ändern** mit altem PW als Proof (statt nur via Recovery)
- **Audit-Log** — `audit_log` Tabelle, `GET /audit?since=...` Endpoint, Viewer im Dashboard
- **2FA am Cloud-Account selbst** — TOTP-Setup, separat vom KDBX-Yubikey
- **Account-PW-Strength-Meter** auf der Signup-Seite
- **Push-Notifications für Vault-Änderungen** — WebSocket statt poll-on-focus
- **CI** für `server/` + `dashboard/` (Docker-Build, e2e API-Tests)
- **Linux Hello-Äquivalent** (libsecret / Keychain auf macOS)

### Mobile

Tauri 2.0 unterstützt iOS und Android. Mit dem Cloud-Server in Place und Multi-Vault macht eine Read-Mostly-Mobile-App jetzt Sinn. Eigene Session, danach.

---

## Code-Layout-Karte

| Pfad | Was |
|------|-----|
| `src/lib/components/Sidebar.svelte` | Vault-Header (mit RO-Badge), GroupTree, Nav |
| `src/lib/components/VaultSwitcher.svelte` | Dropdown im Header für Cloud-Vault-Switch + Create |
| `src/lib/components/VaultMembersDialog.svelte` | Member-Liste mit Rollen-Dropdown, Invite, Revoke |
| `src/lib/components/CloudUnlockTab.svelte` | Login / Picker / Hello-Unlock / Hello-Enroll |
| `src/lib/components/ConflictResolutionDialog.svelte` | Per-Eintrag Sync-Konflikt-UI |
| `src/lib/components/DatabaseConflictDialog.svelte` | Fallback Sync/Overwrite/Cancel |
| `src/lib/app-state.svelte.ts` | Globaler reaktiver State (Phase, Cloud-Session, Vault-PW-Cache, isReadOnly) |
| `src-tauri/src/cloud/` | Cloud crypto + HTTP client + Session-State |
| `src-tauri/src/commands/cloud.rs` | Tauri-Commands für signup / login / push / pull / switch / share / members |
| `src-tauri/src/commands/cloud_persistence.rs` | Windows Credential Manager Persistierung der Cloud-Session |
| `src-tauri/src/commands/hello.rs` | Hello für lokale kdbx + neu für Cloud-Bundle |
| `src-tauri/src/commands/database.rs` | open/save/merge/conflict + notify-Watcher install |
| `src-tauri/src/kdbx/database.rs` | KDBX-Layer mit Option<path> für cloud-only |
| `src-tauri/src/lockfile.rs` | `<db>.kdbx.lock` mit Stale-Detection |
| `server/src/storage.rs` | SQLite users + vaults + vault_members |
| `server/src/auth.rs` | Signup / Login / Recovery / Reset / Lookup |
| `server/src/vault.rs` | Vault CRUD + Share + Members |
| `dashboard/src/App.svelte` | Web-Admin-SPA (Login + Vault-Listing) |
