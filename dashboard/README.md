# password-wallet dashboard

Browser admin for [`password-wallet-server`](../server/README.md). Same
indigo design system as the [desktop app](../README.md) (Tailwind v4 + the
same oklch palette tokens). Sign in with your cloud credentials, list your
vaults, rename or delete them.

> **The dashboard cannot view vault contents.** Only metadata (name, role,
> updated-at). To read or edit passwords use the desktop app. Argon2id runs
> in the browser via `hash-wasm`; AES-GCM via Web Crypto. The master
> password never leaves the browser tab.

## What's here

✅ implemented
- **Login** with full Argon2id-based auth proof (matches the desktop
  client's crypto exactly — `m=64MiB, t=3, p=4, hash-len=32`)
- **Vault listing** with role badges (owner / editor / reader)
- **Rename + delete** for owners
- **Local-storage token persistence** — refresh keeps you signed in
- **Smart server URL default** — uses `window.location.origin` in
  production (served by Caddy alongside the API), falls back to
  `http://localhost:8090` in Vite dev mode
- **Helpful error messages** — 404 on login distinguishes "no account with
  that username" from "can't reach server"

⏭️ next (Phase 6 — UI polish + features)
- Replace native `window.prompt` / `window.confirm` with real modals
- Members management UI (mirror of desktop's VaultMembersDialog)
- Account self-service: change cloud password, view recovery state,
  delete account
- Audit log viewer (once the server exposes events)
- Empty-state card when account has no vaults yet
- Light/dark theme toggle (currently follows OS only)

## Run locally

```bash
cd dashboard
npm install
npm run dev          # → http://localhost:5173
```

The dev server expects the vault server on `http://localhost:8090`. Adjust
the Server URL field in the login screen if yours runs elsewhere.

## Build for production

```bash
npm run build        # → dist/
```

Mountable as a static SPA. `server/docker-compose.yml` already
bind-mounts `../dashboard/dist` into the Caddy service at `/srv/dashboard`
— after building, the dashboard becomes reachable at
`https://yourdomain/dashboard/`.

Bundle size (post-Tailwind-v4):
- `dist/assets/index-*.css` — ~19 KB / ~4 KB gzip
- `dist/assets/index-*.js`  — ~77 KB / ~30 KB gzip

## Architecture notes

- **Vanilla Svelte 5 + Vite + Tailwind v4 SPA.** No SvelteKit (no SSR or
  filesystem routing needed for a single login + listing surface).
- **`src/api.ts`** mirrors the REST surface from the desktop client. If
  the server endpoints change, both consumers need to update.
- **Argon2id parameters** live in `api.ts:KDF_PARAMS` and MUST stay in
  sync with `src-tauri/src/cloud/crypto.rs` — otherwise the auth proof
  won't validate server-side. Change one, change the other.
- **AES-GCM unwrap** uses the platform Web Crypto API (`crypto.subtle`),
  not a polyfill — saves bundle size.
- **Design tokens** in `src/app.css` are a copy of the desktop app's
  palette (oklch indigo-violet). Keep them in sync when the palette
  evolves; consider extracting to a shared package once a third consumer
  needs them.

## Threat model (browser-side)

Same as the desktop app — see [`../server/README.md`](../server/README.md)
"Threat model" section. The dashboard is just one more client; it doesn't
add or remove anything from the server's perspective.

What's different in a browser:
- **No persistent vault_key** — the dashboard doesn't decrypt vault
  contents, so it never derives `vault_key`. Only `client_auth_hash` is
  computed and sent.
- **localStorage stores the JWT** — same trust boundary as any other web
  app. If your browser is compromised, the token is gone. Mitigate by
  signing out when you're done (top-right "Sign out" button).
- **No service worker, no offline cache** — the dashboard always needs
  the network to do anything useful.
