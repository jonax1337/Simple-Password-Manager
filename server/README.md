# password-wallet-server

Self-hosted, end-to-end-encrypted vault sync server for [Simple Password Manager](../README.md).

> **Status: MVP, usable, not yet production-hardened.** The protocol is stable
> enough to round-trip vaults between any number of devices and to share them
> across accounts with proper E2E key wrap. What's NOT in yet: schema-migration
> guarantees, audit logging, 2FA on the cloud account itself, and a password-
> change-without-recovery flow. See the bottom of this file for the full
> "out of scope" list.

## What it does

- **Multi-vault per account** — Personal, Work, Family, … each with its own KDBX-style master password
- **End-to-end encryption** — server only sees opaque AES-256-GCM ciphertext; master password never leaves the device
- **Vault sharing** with Curve25519 sealed-box key wrap and Owner / Editor / Reader permission levels
- **Recovery codes** so a forgotten master password isn't a death sentence
- **Per-IP rate limiting** on `/auth/*` (5 burst, ~10/min steady) via `tower_governor`
- **ETag-based optimistic locking** — two devices pushing the same vault don't silently overwrite each other
- **Zero-config startup** — JWT secret + DB path auto-provisioned to OS data dir on first run
- **JWT auth** — 7-day bearer tokens, refresh-on-use via Remember-Me

## What it does NOT do

- See, decrypt, or recover your vault contents. **The server only holds
  ciphertext + Argon2-of-Argon2 auth proofs.** If you lose both your master
  password and your recovery code, the data is unrecoverable.
- Send push notifications when another device updates a vault. Clients
  re-sync when the user opens / switches a vault. (Future: WebSocket.)
- Track read access. Editors / owners can audit *write* events through
  `updated_at` on each vault; there's no per-read log yet.
- Federate across servers. Each instance is single-tenant of its own users.

## Threat model

```
master_password (typed)
    │ Argon2id (m=64 MiB, t=3, p=4, client-side)
    ▼
password_key (32B, used only to wrap master_key)
    │
    ▼
master_key (32B, random at signup, never derived) ──┐
    │                                                │
    ├─ SHA-256(··· || "auth") ──► client_auth_hash ──┴─► (sent on login)
    │                                                     │ Argon2 again
    │                                                     ▼
    │                                              stored in users.server_auth_hash
    │
    ├─ SHA-256(··· || "vault") ──► vault_key (random per vault, wrapped under master_key)
    │                                          │
    │                                          ▼
    │                                    AES-256-GCM(kdbx_bytes) ──► vault_blob
    │
    └─ X25519 keypair (privkey wrapped under master_key)
          │
          ▼
      account_pubkey stored server-side, used by other accounts to
      sealed-box wrap a vault_key for sharing with this account
```

A full database leak forces an attacker through:

1. Argon2id (server salt) on `server_auth_hash` — recover `client_auth_hash`
2. AES-256-GCM unwrap of `wrapped_master_key` using a `password_key` derived
   from a guessed master password (64 MiB Argon2id per guess)
3. With `master_key` in hand, AES-256-GCM unwrap each `wrapped_vault_key`,
   then each vault's ciphertext blob

The master password is the only low-entropy input. **Pick a strong one.**

The recovery code is a 20-character Crockford-base32 string (80 bits of
entropy). The server stores `master_key` wrapped under a key derived from
`(recovery_code, SHA-256("recovery:" || email))` so a recovery flow doesn't
require the user to remember anything except the printed code. Recovery
codes themselves are shown **once** at signup — the server holds the wrap,
not the code.

---

## Running locally

```bash
cd server
cargo run --release
# Listens on 0.0.0.0:8090
# JWT secret + DB land in your OS data dir on first run:
#   Windows: %APPDATA%\passwordwallet\password-wallet-server\data\
#   Linux:   ~/.local/share/passwordwallet/password-wallet-server/data/
#   macOS:   ~/Library/Application Support/passwordwallet/password-wallet-server/data/
```

No environment variables required for local dev. Point the desktop app at
`http://localhost:8090` in Settings → Cloud Sync → Create account.

---

## Production deploy (HTTPS via Docker Compose + Caddy)

Auto-TLS, single-command deploy:

```bash
cd server
cp .env.example .env
$EDITOR .env             # set DOMAIN + ACME_EMAIL
docker compose up -d
```

Caddy provisions a Let's Encrypt cert on first request and renews it
automatically. The vault server runs on the internal Docker network only;
Caddy is the only thing that listens on the public network.

### docker-compose.yml (already in this directory)

```yaml
services:
  vault:
    build: .
    restart: unless-stopped
    expose: ["8090"]
    volumes:
      - vault-data:/data
    environment:
      BIND: "0.0.0.0:8090"
      DATA_DIR: /data
      # JWT_SECRET intentionally NOT set: auto-provisioned on first run.
      # Override here only when running behind multiple replicas.
      RUST_LOG: ${RUST_LOG:-info}

  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports: ["80:80", "443:443", "443:443/udp"]
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - ../dashboard/dist:/srv/dashboard:ro   # built static SPA
      - caddy-data:/data
      - caddy-config:/config
    environment:
      DOMAIN: ${DOMAIN}
      ACME_EMAIL: ${ACME_EMAIL}
    depends_on: [vault]

volumes: { vault-data:, caddy-data:, caddy-config: }
```

### Caddyfile

```caddyfile
{
    email {$ACME_EMAIL}
}

{$DOMAIN} {
    encode zstd gzip
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Content-Type-Options nosniff
        Referrer-Policy strict-origin
    }

    handle_path /dashboard/* {
        root * /srv/dashboard
        try_files {path} /index.html
        file_server
    }
    handle {
        reverse_proxy vault:8090 {
            header_up X-Real-IP {remote_host}
            header_up X-Forwarded-For {remote_host}
        }
    }
}
```

### Building the dashboard for serving

The compose file bind-mounts `../dashboard/dist`. Build it once before
`docker compose up`:

```bash
cd ../dashboard
npm install
npm run build
```

After that the dashboard is reachable at `https://yourdomain/dashboard/`.

---

## Environment variables

| Var          | Default                          | Notes                                              |
|--------------|----------------------------------|----------------------------------------------------|
| `BIND`       | `0.0.0.0:8090`                   | `host:port` for the listener                       |
| `DATA_DIR`   | OS data dir                      | Where DB + JWT secret live. Override in containers. |
| `DB_PATH`    | `$DATA_DIR/vault.db`             | SQLite file. Persist this volume.                  |
| `JWT_SECRET` | **auto-generated** on first run  | 32 hex bytes. Override only for multi-replica setups. Deleting `$DATA_DIR/jwt_secret` invalidates all sessions. |
| `RUST_LOG`   | `info`                           | Standard tracing-subscriber filter syntax          |

---

## REST surface

All bodies + responses are JSON. Errors are `{error: string}` with the
appropriate status. Auth column: *none* = public, *Bearer* = `Authorization:
Bearer <jwt>`.

### Auth + account

| Method | Path                | Auth   | Body / response |
|--------|---------------------|--------|------------------|
| GET    | `/health`           | none   | `"ok"` plain text |
| POST   | `/auth/signup`      | none*  | `{email, kdf_salt_b64, client_auth_hash, wrapped_master_key_b64, recovery_blob_b64, account_pubkey_b64, wrapped_account_privkey_b64, initial_vault_name, initial_vault_blob_b64, initial_wrapped_vault_key_b64}` → `{token, user_id}` |
| POST   | `/auth/login`       | none*  | `{email, client_auth_hash}` → `{token, user_id, kdf_salt_b64, wrapped_master_key_b64, wrapped_account_privkey_b64, account_pubkey_b64}` |
| POST   | `/auth/kdf-params`  | none*  | `{email, client_auth_hash: "x"}` → `{kdf_salt_b64, wrapped_master_key_b64}` |
| POST   | `/auth/recovery-init` | none* | `{email}` → `{recovery_blob_b64}` |
| POST   | `/auth/reset`       | none*  | `{email, client_auth_hash, new_kdf_salt_b64, new_wrapped_master_key_b64}` → `{token}` |

*\* rate-limited per IP via `tower_governor` (5 burst, 1 token per 6s, falls back to `X-Real-IP` / `X-Forwarded-For` when behind Caddy)*

### Users (auth-gated lookup)

| Method | Path             | Auth   | Body / response |
|--------|------------------|--------|------------------|
| POST   | `/users/lookup`  | Bearer | `{email, client_auth_hash: "x"}` → `{user_id, account_pubkey_b64}` (404 if unknown OR if user hasn't enrolled for sharing) |

### Vaults

| Method | Path                    | Auth   | Permission   | Body / response |
|--------|-------------------------|--------|--------------|------------------|
| GET    | `/vaults`               | Bearer | any member   | `{vaults: [VaultSummary]}` |
| POST   | `/vaults`               | Bearer | any user     | `{name, ciphertext_b64, wrapped_vault_key_b64}` → `{id, etag, updated_at}` |
| GET    | `/vaults/{id}`          | Bearer | any member   | → `{id, name, role, owner_user_id, etag, created_at, updated_at, wrapped_vault_key_b64, ciphertext_b64}`, header `ETag` |
| PUT    | `/vaults/{id}`          | Bearer | owner+editor | `{ciphertext_b64, expected_etag?}` → `{etag, updated_at}` (409 on mismatch) |
| PATCH  | `/vaults/{id}`          | Bearer | owner only   | `{name}` — rename |
| DELETE | `/vaults/{id}`          | Bearer | owner only   | wipes vault + cascade-removes member rows |
| POST   | `/vaults/{id}/share`    | Bearer | owner only   | `{recipient_user_id, wrapped_vault_key_b64, role}` (upsert — re-share replaces) |
| DELETE | `/vaults/{id}/share`    | Bearer | owner or self | `{user_id}` — revoke; refuses to remove the last owner |
| GET    | `/vaults/{id}/members`  | Bearer | any member   | `{members: [{user_id, email, role, invited_at, accepted_at}]}` |
| PATCH  | `/vaults/{id}/members/{user_id}` | Bearer | owner only | `{role}` — refuses to demote the last owner |

`VaultSummary` (returned by list/create): `{id, name, role, owner_user_id, etag, created_at, updated_at, wrapped_vault_key_b64}`.

`role` is always `"owner" | "editor" | "reader"`.

---

## Database schema

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    server_auth_hash TEXT NOT NULL,              -- Argon2 of client_auth_hash
    kdf_salt_b64 TEXT NOT NULL,                  -- client-side Argon2 salt (public)
    wrapped_master_key_b64 TEXT NOT NULL,        -- master_key wrapped under password_key
    recovery_blob_b64 TEXT NOT NULL,             -- master_key wrapped under recovery_key
    account_pubkey_b64 TEXT NOT NULL DEFAULT '', -- X25519 public key, public
    wrapped_account_privkey_b64 TEXT NOT NULL DEFAULT '',  -- privkey wrapped under master_key
    created_at INTEGER NOT NULL
);

CREATE TABLE vaults (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    owner_user_id TEXT NOT NULL,
    ciphertext_b64 TEXT NOT NULL,                -- AES-GCM(vault_key, kdbx_bytes), base64
    etag TEXT NOT NULL,                          -- changes on every ciphertext update
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE vault_members (
    vault_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,                          -- 'owner' | 'editor' | 'reader'
    wrapped_vault_key_b64 TEXT NOT NULL,         -- vault_key wrapped for THIS user
    invited_at INTEGER NOT NULL,
    accepted_at INTEGER,                         -- null = pending (auto-accept for now)
    PRIMARY KEY (vault_id, user_id),
    FOREIGN KEY (vault_id) REFERENCES vaults(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_vault_members_user ON vault_members(user_id);
CREATE INDEX idx_vaults_owner ON vaults(owner_user_id);
```

Owners always have a `vault_members` row (with `role='owner'`) on their own
vaults — that keeps listing + permission code uniform across the three
roles.

---

## Tests

```bash
cd server
cargo test
```

Currently 14 tests covering: schema init, user uniqueness, vault create +
list + update + CAS, role-based write rejection, owner-only rename/delete,
signup → login + recovery flow, sharing roundtrip.

Integration tests use an in-memory SQLite + axum's `oneshot` — no network,
no fixture files. Rate-limit middleware is skipped under tests (would
otherwise 500 because `oneshot()` has no socket peer for `SmartIpKeyExtractor`).

---

## Out of scope (next iterations)

These are deliberate gaps in the current MVP. None are fundamental — each
has a clear path forward, see [`ROADMAP.md`](../ROADMAP.md) Phase 6.

- **Audit log viewer** (schema + endpoint + dashboard UI)
- **Account self-service**: change password with old password as proof,
  delete account, regenerate recovery code
- **2FA / WebAuthn on the cloud account itself** (separate from the
  Yubikey-on-KDBX flow)
- **Mobile clients** (iOS / Android via Tauri 2)
- **Push notifications** for vault changes (WebSocket)
- **Schema migrations** with proper versioning
- **Linux Hello equivalent** (libsecret / Keychain on macOS)
- **Read-access audit** (only writes are logged via `updated_at`)
- **CI** (Docker image build + e2e API tests on push)

---

## Versioning

This server speaks one wire protocol version. There is no version handshake.
If the protocol changes incompatibly, expect a `v0 → v1` bump that the
desktop app will negotiate explicitly. For now: keep desktop + server +
dashboard versions in lockstep.
