#!/usr/bin/env node
// Register the native messaging host for the running browser(s).
//
// Usage:
//   node scripts/install-host.mjs --extension-id <id> [--host-path <path>]
//
// If --host-path is omitted, the script tries to locate the host binary at:
//   <repo>/src-tauri/target/release/browser-bridge-host(.exe)
//   <repo>/src-tauri/target/debug/browser-bridge-host(.exe)
//
// Effect on Windows:
//   - Writes manifest JSON to %LOCALAPPDATA%\digital.laux.passwordmanager\bridge\nm-host.<browser>.json
//   - Writes a registry value HKCU\Software\<vendor>\NativeMessagingHosts\<name>
//     pointing at the manifest file.
//
// Effect on macOS / Linux:
//   - Writes manifest JSON into the browser's per-user NativeMessagingHosts dir.
//
// Re-running the script is safe; it overwrites existing manifest/registry entries.

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { homedir, platform } from "node:os";
import { join, resolve, dirname } from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const HOST_NAME = "digital.laux.simple_password_manager";
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

const args = parseArgs(process.argv.slice(2));
if (!args["extension-id"]) {
  console.error("Required: --extension-id <id>");
  console.error("Find the ID at chrome://extensions (Developer Mode enabled).");
  process.exit(2);
}
const extensionId = args["extension-id"];
const hostPath = args["host-path"] ?? locateHostBinary();
if (!hostPath || !existsSync(hostPath)) {
  console.error(`Host binary not found at: ${hostPath}`);
  console.error(
    "Build it first with `cargo build --bin browser-bridge-host` " +
      "in src-tauri/, or pass --host-path explicitly.",
  );
  process.exit(2);
}

const os = platform();
const browserKinds =
  os === "win32"
    ? ["chrome", "edge", "firefox"]
    : os === "darwin"
      ? ["chrome", "edge", "firefox", "chromium"]
      : ["chrome", "edge", "firefox", "chromium"];

let installed = 0;
for (const kind of browserKinds) {
  try {
    install(kind, extensionId, hostPath);
    installed++;
  } catch (e) {
    console.warn(`  skipping ${kind}: ${e.message}`);
  }
}

if (installed === 0) {
  console.error("\nNo browsers registered.");
  process.exit(1);
}
console.log(`\nDone. Registered for ${installed} browser(s).`);
console.log(
  "Restart any open browser windows so the new native host is picked up.",
);

// ---------- helpers ----------

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next && !next.startsWith("--")) {
        out[key] = next;
        i++;
      } else {
        out[key] = true;
      }
    }
  }
  return out;
}

function locateHostBinary() {
  const exe = os === "win32" ? "browser-bridge-host.exe" : "browser-bridge-host";
  for (const profile of ["release", "debug"]) {
    const p = join(REPO_ROOT, "src-tauri", "target", profile, exe);
    if (existsSync(p)) return p;
  }
  return null;
}

function install(kind, extId, binPath) {
  // Build manifest content. Firefox uses `allowed_extensions`, Chrome family
  // uses `allowed_origins`.
  const manifest = {
    name: HOST_NAME,
    description: "Simple Password Manager native messaging host",
    path: binPath,
    type: "stdio",
  };
  if (kind === "firefox") {
    manifest.allowed_extensions = [`${extId}@simple-password-manager`];
  } else {
    manifest.allowed_origins = [`chrome-extension://${extId}/`];
  }

  const { manifestPath, vendor } = paths(kind);
  mkdirSync(dirname(manifestPath), { recursive: true });
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
  console.log(`  ${kind}: wrote ${manifestPath}`);

  if (os === "win32") {
    const reg = `HKCU\\Software\\${vendor}\\NativeMessagingHosts\\${HOST_NAME}`;
    execSync(
      `reg add "${reg}" /ve /t REG_SZ /d "${manifestPath.replace(/\\/g, "\\\\")}" /f`,
      { stdio: "ignore" },
    );
    console.log(`  ${kind}: registry ${reg}`);
  }
}

function paths(kind) {
  if (os === "win32") {
    const localAppData =
      process.env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local");
    const dir = join(
      localAppData,
      "digital.laux.passwordmanager",
      "bridge",
      "native-hosts",
    );
    const vendor = {
      chrome: "Google\\Chrome",
      chromium: "Chromium",
      edge: "Microsoft\\Edge",
      firefox: "Mozilla",
    }[kind];
    return {
      manifestPath: join(dir, `${HOST_NAME}.${kind}.json`),
      vendor,
    };
  }
  if (os === "darwin") {
    const support = join(homedir(), "Library", "Application Support");
    const dirs = {
      chrome: join(support, "Google", "Chrome", "NativeMessagingHosts"),
      chromium: join(support, "Chromium", "NativeMessagingHosts"),
      edge: join(support, "Microsoft Edge", "NativeMessagingHosts"),
      firefox: join(homedir(), "Library", "Application Support", "Mozilla", "NativeMessagingHosts"),
    };
    return { manifestPath: join(dirs[kind], `${HOST_NAME}.json`) };
  }
  // Linux
  const cfg = join(homedir(), ".config");
  const dirs = {
    chrome: join(cfg, "google-chrome", "NativeMessagingHosts"),
    chromium: join(cfg, "chromium", "NativeMessagingHosts"),
    edge: join(cfg, "microsoft-edge", "NativeMessagingHosts"),
    firefox: join(homedir(), ".mozilla", "native-messaging-hosts"),
  };
  return { manifestPath: join(dirs[kind], `${HOST_NAME}.json`) };
}
