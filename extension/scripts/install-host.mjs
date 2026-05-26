#!/usr/bin/env node
// Register the native messaging host for every Chromium-based browser
// (and Firefox) currently installed on this machine.
//
// Usage:
//   node scripts/install-host.mjs --extension-id <id> [--host-path <path>]
//   node scripts/install-host.mjs --uninstall
//
// What it does:
//   * Detects installed browsers via well-known per-OS paths/registry keys.
//   * For each detected browser, writes a native messaging host manifest
//     to the per-user directory the browser scans on startup.
//   * On Windows, also writes the HKCU registry value that points the
//     browser at that manifest.
//
// Re-running is safe — manifests/registry are overwritten.
//
// Important limitation: this script only sets up the **bridge** between
// the desktop app and an *already-installed* extension. The browser still
// requires that the extension itself be loaded (unpacked in dev, or from
// the Chrome Web Store / AMO once published). Browser vendors deliberately
// do not allow silent extension installation from outside their stores.

import { existsSync, mkdirSync, readdirSync, writeFileSync, rmSync } from "node:fs";
import { homedir, platform } from "node:os";
import { dirname, join, resolve } from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const HOST_NAME = "digital.laux.simple_password_manager";
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const OS = platform();

const args = parseArgs(process.argv.slice(2));

if (args.uninstall) {
  uninstall();
  process.exit(0);
}

if (!args["extension-id"]) {
  console.error("Usage: install-host.mjs --extension-id <id> [--host-path <path>]");
  console.error("       install-host.mjs --uninstall");
  console.error("");
  console.error("Find the extension ID at chrome://extensions (Developer Mode enabled).");
  process.exit(2);
}

const extensionId = args["extension-id"];
const hostPath = args["host-path"] ?? locateHostBinary();
if (!hostPath || !existsSync(hostPath)) {
  console.error(`Host binary not found at: ${hostPath ?? "(not located)"}`);
  console.error(
    "Build it first with `cargo build --bin browser-bridge-host` in src-tauri/, " +
      "or pass --host-path explicitly.",
  );
  process.exit(2);
}

const browsers = detectBrowsers();
if (browsers.length === 0) {
  console.error("No supported browsers detected on this machine.");
  process.exit(1);
}

console.log(`Detected ${browsers.length} browser(s):`);
for (const b of browsers) console.log(`  - ${b.label}`);
console.log("");

let ok = 0;
let fail = 0;
for (const b of browsers) {
  try {
    install(b, extensionId, hostPath);
    ok++;
  } catch (e) {
    console.warn(`  ${b.label}: SKIPPED (${e.message})`);
    fail++;
  }
}

console.log("");
console.log(
  `Done. ${ok} browser(s) registered${fail > 0 ? `, ${fail} skipped` : ""}.`,
);
console.log("Restart any open browser windows so the new native host is picked up.");

// ---------------- detection ----------------

// Returns the list of browsers we should attempt to register for. Each entry
// has { id, label, vendor (Win registry path), kind (chromium|firefox),
// configDir (per-OS native-host manifest dir) }.
function detectBrowsers() {
  if (OS === "win32") return detectBrowsersWindows();
  if (OS === "darwin") return detectBrowsersMac();
  return detectBrowsersLinux();
}

function detectBrowsersWindows() {
  const lad = process.env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local");
  const rd = process.env.APPDATA ?? join(homedir(), "AppData", "Roaming");
  const pf = process.env["ProgramFiles"] ?? "C:\\Program Files";
  const pf86 = process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)";

  // The native-host manifest itself can live anywhere on disk — we put it
  // in our app's data dir so uninstall has a single place to clean.
  const manifestDir = join(
    lad,
    "digital.laux.passwordmanager",
    "bridge",
    "native-hosts",
  );

  const candidates = [
    {
      id: "chrome",
      label: "Google Chrome",
      kind: "chromium",
      vendor: "Google\\Chrome",
      installProbe: [
        join(pf, "Google", "Chrome", "Application", "chrome.exe"),
        join(pf86, "Google", "Chrome", "Application", "chrome.exe"),
        join(lad, "Google", "Chrome", "Application", "chrome.exe"),
      ],
    },
    {
      id: "edge",
      label: "Microsoft Edge",
      kind: "chromium",
      vendor: "Microsoft\\Edge",
      installProbe: [
        join(pf, "Microsoft", "Edge", "Application", "msedge.exe"),
        join(pf86, "Microsoft", "Edge", "Application", "msedge.exe"),
      ],
    },
    {
      id: "brave",
      label: "Brave",
      kind: "chromium",
      vendor: "BraveSoftware\\Brave-Browser",
      installProbe: [
        join(pf, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
        join(pf86, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
        join(lad, "BraveSoftware", "Brave-Browser", "Application", "brave.exe"),
      ],
    },
    {
      id: "vivaldi",
      label: "Vivaldi",
      kind: "chromium",
      vendor: "Vivaldi",
      installProbe: [
        join(lad, "Vivaldi", "Application", "vivaldi.exe"),
        join(pf, "Vivaldi", "Application", "vivaldi.exe"),
      ],
    },
    {
      id: "opera",
      label: "Opera",
      kind: "chromium",
      vendor: "Opera Software\\Opera Stable",
      installProbe: [
        join(lad, "Programs", "Opera", "launcher.exe"),
        join(pf, "Opera", "launcher.exe"),
      ],
    },
    {
      id: "chromium",
      label: "Chromium",
      kind: "chromium",
      vendor: "Chromium",
      installProbe: [join(lad, "Chromium", "Application", "chrome.exe")],
    },
    {
      id: "arc",
      label: "Arc",
      kind: "chromium",
      vendor: "TheBrowserCompany\\Arc",
      installProbe: [
        join(lad, "Arc", "Application", "Arc.exe"),
        join(rd, "Arc", "Application", "Arc.exe"),
      ],
    },
    {
      id: "firefox",
      label: "Firefox",
      kind: "firefox",
      vendor: "Mozilla",
      installProbe: [
        join(pf, "Mozilla Firefox", "firefox.exe"),
        join(pf86, "Mozilla Firefox", "firefox.exe"),
      ],
    },
  ];

  return candidates
    .filter((c) => c.installProbe.some((p) => existsSync(p)))
    .map((c) => ({
      ...c,
      manifestPath: join(manifestDir, `${HOST_NAME}.${c.id}.json`),
    }));
}

function detectBrowsersMac() {
  const apps = "/Applications";
  const userApps = join(homedir(), "Applications");
  const support = join(homedir(), "Library", "Application Support");

  const candidates = [
    {
      id: "chrome",
      label: "Google Chrome",
      kind: "chromium",
      installProbe: [
        join(apps, "Google Chrome.app"),
        join(userApps, "Google Chrome.app"),
      ],
      manifestPath: join(
        support,
        "Google",
        "Chrome",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "edge",
      label: "Microsoft Edge",
      kind: "chromium",
      installProbe: [join(apps, "Microsoft Edge.app")],
      manifestPath: join(
        support,
        "Microsoft Edge",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "brave",
      label: "Brave",
      kind: "chromium",
      installProbe: [join(apps, "Brave Browser.app")],
      manifestPath: join(
        support,
        "BraveSoftware",
        "Brave-Browser",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "vivaldi",
      label: "Vivaldi",
      kind: "chromium",
      installProbe: [join(apps, "Vivaldi.app")],
      manifestPath: join(
        support,
        "Vivaldi",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "arc",
      label: "Arc",
      kind: "chromium",
      installProbe: [join(apps, "Arc.app")],
      manifestPath: join(
        support,
        "Arc",
        "User Data",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "chromium",
      label: "Chromium",
      kind: "chromium",
      installProbe: [join(apps, "Chromium.app")],
      manifestPath: join(
        support,
        "Chromium",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "firefox",
      label: "Firefox",
      kind: "firefox",
      installProbe: [join(apps, "Firefox.app")],
      manifestPath: join(
        support,
        "Mozilla",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
  ];

  return candidates.filter((c) => c.installProbe.some((p) => existsSync(p)));
}

function detectBrowsersLinux() {
  const cfg = join(homedir(), ".config");
  const candidates = [
    {
      id: "chrome",
      label: "Google Chrome",
      kind: "chromium",
      installProbe: ["/usr/bin/google-chrome", "/usr/bin/google-chrome-stable"],
      manifestPath: join(
        cfg,
        "google-chrome",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "edge",
      label: "Microsoft Edge",
      kind: "chromium",
      installProbe: ["/usr/bin/microsoft-edge", "/usr/bin/microsoft-edge-stable"],
      manifestPath: join(
        cfg,
        "microsoft-edge",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "brave",
      label: "Brave",
      kind: "chromium",
      installProbe: ["/usr/bin/brave-browser", "/usr/bin/brave"],
      manifestPath: join(
        cfg,
        "BraveSoftware",
        "Brave-Browser",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "vivaldi",
      label: "Vivaldi",
      kind: "chromium",
      installProbe: ["/usr/bin/vivaldi", "/usr/bin/vivaldi-stable"],
      manifestPath: join(
        cfg,
        "vivaldi",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "opera",
      label: "Opera",
      kind: "chromium",
      installProbe: ["/usr/bin/opera"],
      manifestPath: join(
        cfg,
        "opera",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "chromium",
      label: "Chromium",
      kind: "chromium",
      installProbe: ["/usr/bin/chromium", "/usr/bin/chromium-browser"],
      manifestPath: join(
        cfg,
        "chromium",
        "NativeMessagingHosts",
        `${HOST_NAME}.json`,
      ),
    },
    {
      id: "firefox",
      label: "Firefox",
      kind: "firefox",
      installProbe: ["/usr/bin/firefox", "/usr/bin/firefox-esr"],
      manifestPath: join(
        homedir(),
        ".mozilla",
        "native-messaging-hosts",
        `${HOST_NAME}.json`,
      ),
    },
  ];

  return candidates.filter((c) => c.installProbe.some((p) => existsSync(p)));
}

// ---------------- install / uninstall ----------------

function install(browser, extId, binPath) {
  const manifest = {
    name: HOST_NAME,
    description: "Simple Password Manager native messaging host",
    path: binPath,
    type: "stdio",
  };
  if (browser.kind === "firefox") {
    manifest.allowed_extensions = [`${extId}@simple-password-manager`];
  } else {
    manifest.allowed_origins = [`chrome-extension://${extId}/`];
  }

  mkdirSync(dirname(browser.manifestPath), { recursive: true });
  writeFileSync(browser.manifestPath, JSON.stringify(manifest, null, 2));
  console.log(`  ${browser.label}: wrote ${browser.manifestPath}`);

  if (OS === "win32" && browser.vendor) {
    const reg = `HKCU\\Software\\${browser.vendor}\\NativeMessagingHosts\\${HOST_NAME}`;
    execSync(
      `reg add "${reg}" /ve /t REG_SZ /d "${browser.manifestPath.replace(/\\/g, "\\\\")}" /f`,
      { stdio: "ignore" },
    );
    console.log(`  ${browser.label}: HKCU\\Software\\${browser.vendor}\\NativeMessagingHosts\\${HOST_NAME}`);
  }
}

function uninstall() {
  const browsers = detectBrowsers();
  let removed = 0;
  for (const b of browsers) {
    try {
      if (existsSync(b.manifestPath)) {
        rmSync(b.manifestPath);
        console.log(`  ${b.label}: removed ${b.manifestPath}`);
        removed++;
      }
      if (OS === "win32" && b.vendor) {
        try {
          execSync(
            `reg delete "HKCU\\Software\\${b.vendor}\\NativeMessagingHosts\\${HOST_NAME}" /f`,
            { stdio: "ignore" },
          );
          console.log(`  ${b.label}: registry cleaned`);
        } catch {
          /* registry key wasn't there */
        }
      }
    } catch (e) {
      console.warn(`  ${b.label}: ${e.message}`);
    }
  }
  // Clean up the manifestDir if empty (Windows only — it lives under our app dir).
  if (OS === "win32") {
    const lad = process.env.LOCALAPPDATA ?? join(homedir(), "AppData", "Local");
    const dir = join(
      lad,
      "digital.laux.passwordmanager",
      "bridge",
      "native-hosts",
    );
    try {
      if (existsSync(dir) && readdirSync(dir).length === 0) {
        rmSync(dir, { recursive: true });
      }
    } catch {
      /* ignore */
    }
  }
  console.log(`Done. Removed ${removed} manifest(s).`);
}

// ---------------- helpers ----------------

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith("--")) continue;
    const key = a.slice(2);
    const next = argv[i + 1];
    if (next && !next.startsWith("--")) {
      out[key] = next;
      i++;
    } else {
      out[key] = true;
    }
  }
  return out;
}

function locateHostBinary() {
  const exe = OS === "win32" ? "browser-bridge-host.exe" : "browser-bridge-host";
  for (const profile of ["release", "debug"]) {
    const p = join(REPO_ROOT, "src-tauri", "target", profile, exe);
    if (existsSync(p)) return p;
  }
  return null;
}
