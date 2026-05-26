# Releasing

End-to-end steps to ship a new version with working auto-updates.

## One-time setup

> **Already done on this machine** as of 2026-05-26:
> - Signing key generated at `C:\Users\Jonas\.tauri\password-wallet.key` (no password).
> - Public key embedded in `src-tauri/tauri.conf.json`.
> - GitHub secret `TAURI_SIGNING_PRIVATE_KEY` populated on `jonax1337/Simple-Password-Manager`.
>
> The steps below are kept for reproducibility / disaster recovery.

### 1. Generate Tauri signing keys (only if regenerating)

These are the keys that sign updater artifacts. The private key stays on your machine and in GitHub Secrets; the public key lives in `tauri.conf.json`.

```powershell
npm run tauri -- signer generate -w $HOME\.tauri\password-wallet.key -p "" --ci
```

`-p ""` + `--ci` produces a key with **no password** — simpler for CI (no second secret needed). If you want a password-protected key, drop those flags and you'll be prompted.

Two files appear:

- `~/.tauri/password-wallet.key` — **private**, never commit, never share.
- `~/.tauri/password-wallet.key.pub` — public key, paste the full content into `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`.

⚠️ **Regenerating the key invalidates updates for all existing installs.** Only do it if the private key has actually been compromised.

### 2. Configure GitHub Secrets

In the repo settings → Secrets and variables → Actions:

- `TAURI_SIGNING_PRIVATE_KEY` — full contents of `~/.tauri/password-wallet.key`. (Already set.)
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — **only needed** if you generated a password-protected key. Currently unset, which is correct for the empty-password key in use.

Quick set via CLI:

```powershell
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo jonax1337/Simple-Password-Manager < "$HOME\.tauri\password-wallet.key"
```

### 3. Verify the updater endpoint

`src-tauri/tauri.conf.json` currently points to:

```
https://github.com/jonax1337/Simple-Password-Manager/releases/latest/download/latest.json
```

If the repo ever moves or gets renamed, update this URL.

## Cutting a release

1. Bump the version in three places (must match exactly):
   - `package.json`
   - `src-tauri/tauri.conf.json` → `version`
   - `src-tauri/Cargo.toml` → `[package].version`
2. Commit with message `chore: release v1.0.1` (or whatever the version is).
3. Tag and push:

   ```powershell
   git tag v1.0.1
   git push origin main --tags
   ```

4. The `Release` workflow runs:
   - Builds Windows artifacts with the signing key.
   - Creates a **draft** GitHub release with the installer.
   - A second job generates `latest.json` (the updater manifest) from the signature file and uploads it to the same release.
5. Open the draft release on GitHub, write release notes, then publish.
6. Existing installations check `latest.json` on next launch and will offer the update.

## Verifying the update flow

After publishing a higher-version release:

1. Install the **previous** version locally (or keep an old build around).
2. Launch it. Within ~3 seconds the toast "Update available" should appear.
3. Open Settings → Updates → "Install & restart".
4. Confirm the new version launches with `About → Version` matching the tag.

## Troubleshooting

- **"signature verification failed"** at install time: pubkey in `tauri.conf.json` does not match the private key used to sign. Re-check `plugins.updater.pubkey`.
- **"could not fetch update"**: endpoint URL wrong, `latest.json` not uploaded, or repo is private (GitHub blocks unauthenticated downloads from private repos).
- **No update offered, but a newer release exists**: SemVer in `latest.json` lower-or-equal to the installed version, or the `windows-x86_64` platform entry is missing from `latest.json`. Inspect the file from the GH release.
- **`createUpdaterArtifacts` warnings during local builds**: harmless if you're not testing updates locally. The `.sig` file is only generated during signed CI builds.

## Local installer testing (optional)

If you want to test the installer without publishing a release:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $HOME\.tauri\password-wallet.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "your-key-password"
npm run tauri:build
```

Installer + `.sig` end up under `src-tauri/target/release/bundle/nsis/`.
