# password-wallet-server

Self-hosted, end-to-end-encrypted vault sync server for [Simple Password Manager](../README.md).

> **Status: experimental.** This is the Phase 5 MVP from [`ROADMAP.md`](../ROADMAP.md). The
> protocol is stable enough to round-trip a vault between two devices, but no
> migration guarantees, no rate limiting, no audit logging, and no automated
> account recovery. Use it knowing you'll wipe and re-link if the schema
> changes.

## What it does

- Stores an opaque ciphertext blob per user (the encrypted KDBX file)
- Authenticates clients with a JWT derived from a double-Argon2id proof
- Atomic compare-and-swap writes via ETag — concurrent device A and B
  cannot silently clobber each other

## What it does not do

- See, decrypt, or recover your vault. **If you forget your master password,
  the server cannot help you.** Keep a paper backup.
- Federate across servers, support multiple vaults per account, or offer a
  web UI.
- Send notifications when another device pushes. Clients poll on focus.

## Threat model

```
master_password (typed)
    │ Argon2id (m=64 MiB, t=3, p=4, client-side)
    ▼
master_key (32B, never leaves the device)
    ├─ SHA256(··· || "auth") ──► client_auth_hash ──► (sent to server)
    │                                                    │ Argon2 again
    │                                                    ▼
    │                                             stored hash in users.server_auth_hash
    │
    └─ SHA256(··· || "vault") ─► vault_key (kept in RAM)
                                    │
                                    ▼
                              AES-256-GCM encrypt KDBX bytes ──► vault_blob (sent to server)
```

A full database leak forces an attacker through:
1. Argon2id (server salt) → recover `client_auth_hash`
2. Argon2id (client salt, 64 MiB) → recover `master_key`
3. Brute-force the actual master password (the only low-entropy input)

If they recover `master_key`, they can decrypt the vault blob — so a strong
master password is the only thing standing between a leaked server DB and
your plaintext. There's no `secret_key`-style high-entropy second factor
in this MVP (that's a future enhancement).

## Running locally

```bash
cd server
JWT_SECRET=$(openssl rand -hex 32) cargo run --release
# → listening on 0.0.0.0:8090
```

Point the desktop client at `http://localhost:8090` in Settings → Cloud Sync.

## Docker

```bash
docker build -t password-wallet-server .
docker run -d \
  --name vault-server \
  -p 8090:8090 \
  -v vault-data:/data \
  -e JWT_SECRET=$(openssl rand -hex 32) \
  password-wallet-server
```

For production: terminate TLS in front (Caddy/Nginx), back up `/data`
nightly, and rotate `JWT_SECRET` only when you can also force every client
to re-login (existing tokens become invalid).

### docker-compose example

```yaml
services:
  vault:
    build: .
    restart: unless-stopped
    ports:
      - "127.0.0.1:8090:8090"   # bind localhost-only; Caddy proxies in
    volumes:
      - vault-data:/data
    environment:
      JWT_SECRET: ${JWT_SECRET}  # set via .env
volumes:
  vault-data:
```

## Environment variables

| Var          | Default               | Notes                                              |
|--------------|-----------------------|----------------------------------------------------|
| `BIND`       | `0.0.0.0:8090`        | `host:port` for the listener                       |
| `DB_PATH`    | `vault-server.db`     | SQLite file. Persist this volume.                  |
| `JWT_SECRET` | dev-only fallback     | **Required in release builds.** ≥ 32 hex bytes.    |
| `RUST_LOG`   | `info`                | Standard tracing filter syntax                     |

## REST surface

| Method | Path                | Auth   | Body / response |
|--------|---------------------|--------|------------------|
| GET    | `/health`           | none   | `"ok"` |
| POST   | `/auth/signup`      | none   | `{email, kdf_salt_b64, client_auth_hash, initial_vault_blob_b64}` → `{token, user_id}` |
| POST   | `/auth/login`       | none   | `{email, client_auth_hash}` → `{token, user_id, kdf_salt_b64}` |
| POST   | `/auth/kdf-params`  | none   | `{email, client_auth_hash: "x"}` → `{kdf_salt_b64}` (auth hash ignored) |
| GET    | `/vault`            | Bearer | → `{etag, ciphertext_b64, updated_at}`, header `ETag` |
| PUT    | `/vault`            | Bearer | `{ciphertext_b64, expected_etag?}` → `{etag, updated_at}` (409 on mismatch) |

## Tests

```bash
cd server
cargo test
```

The integration tests in `src/main.rs` exercise the full sign-up → login →
push → conflict-on-stale-etag flow against an in-memory SQLite, no network
required.

## Out of scope (yet)

- Mobile clients
- Federation / multi-tenant
- Rate limiting + IP throttling (use Caddy/Nginx in front for now)
- Account recovery (Phase 5 follow-up: encrypt a copy of `master_key` with
  a printable recovery code generated at signup)
- Push notifications (clients currently poll on focus)
