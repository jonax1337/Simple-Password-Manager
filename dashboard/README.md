# password-wallet dashboard

Browser admin for [`password-wallet-server`](../server/README.md). Sign in
with the same credentials as the desktop app, list your vaults, rename or
delete them.

> **The dashboard cannot view vault contents.** It can only inspect
> metadata (name, role, updated-at) and call administrative endpoints. To
> read a vault's passwords, use the desktop app. Argon2id runs in the
> browser (via `hash-wasm`) so your master password never leaves the
> browser tab.

## Run locally

```bash
cd dashboard
npm install
npm run dev
# → http://localhost:5173
```

The dev server points at the URL you type in the Server URL field. For
local development, run the desktop server on `http://localhost:8090`.

## Build for production

```bash
npm run build
# → dist/  (mountable static SPA)
```

`server/docker-compose.yml` already bind-mounts `../dashboard/dist` into
the Caddy service at `/srv/dashboard`. After building, the dashboard is
reachable at `https://yourdomain/dashboard/`.

## What's here vs. what isn't (Stage E status)

✅ implemented
- Login with full Argon2id-based auth proof (matches desktop crypto)
- List own vaults + role badges
- Rename / delete (owner-only)
- Local-storage token persistence

⏭️ next stages
- Stage C — share a vault with another account (E2E key wrap UI)
- Stage D — manage roles (reader/editor/owner) on shared vaults
- Account self-service: change password, view recovery state, delete account
- Audit log viewer

## Architecture notes

- Vanilla Svelte 5 + Vite. No SvelteKit — the dashboard is a single-page
  SPA, no SSR or filesystem routing needed.
- `src/api.ts` mirrors the REST surface from the desktop client. If those
  endpoints change, both consumers need to update.
- Argon2id parameters live in `api.ts:KDF_PARAMS` and MUST match
  `src-tauri/src/cloud/crypto.rs` exactly (m=64MiB, t=3, p=4, hash-len=32),
  otherwise the auth proof won't validate server-side.
- AES-GCM unwrap uses the platform Web Crypto API directly.
