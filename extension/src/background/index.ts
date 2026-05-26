// Background service worker.
//
// Two responsibilities:
//   1. Native-messaging discovery — talk to the host binary, get back the
//      current HTTP bridge port + token, cache it for the session.
//   2. HTTP proxy — the popup/content scripts ask this worker to issue
//      authenticated fetches against the loopback bridge so the token never
//      leaves this single trust boundary.

import {
  NATIVE_HOST_NAME,
  type BgMessage,
  type BgResponse,
  type BridgeInfo,
  type CaptureMessage,
} from "../common/bridge";

const PENDING_CAPTURE_KEY = "pending_capture";
const CAPTURE_TTL_MS = 5 * 60_000;

// Persistent "I know I have an entry for this host" cache, so the content
// script can show its in-field indicator even while the vault is locked
// (and instantly after extension install, before any popup interaction).
// Values are bare hostnames; no credentials. Refreshed on every successful
// status check + on a soft interval while the SW is awake.
const KNOWN_HOSTS_KEY = "known_hosts";
const KNOWN_HOSTS_UPDATED_AT_KEY = "known_hosts_updated_at";
const KNOWN_HOSTS_TTL_MS = 10 * 60_000;

// Wipe any pending capture left over from a previous browser session —
// chrome.storage.local persists to disk, unlike .session. Keeping a stale
// captured password across a browser restart would be a security regression.
void (async () => {
  try {
    const existing = await chrome.storage.local.get(PENDING_CAPTURE_KEY);
    const cap = existing?.[PENDING_CAPTURE_KEY];
    if (
      !cap ||
      typeof cap.capturedAt !== "number" ||
      Date.now() - cap.capturedAt > CAPTURE_TTL_MS
    ) {
      await chrome.storage.local.remove(PENDING_CAPTURE_KEY);
      await setBadge("");
    } else {
      await setBadge("+");
    }
  } catch {
    /* non-fatal */
  }
})();

let cached: BridgeInfo | null = null;
let cacheLoadedAt = 0;
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes — token rotates per app start

async function discoverBridge(): Promise<BridgeInfo> {
  return new Promise((resolve, reject) => {
    // sendNativeMessage is one-shot: connect, send, read one reply, close.
    const message = { action: "discover" };
    chrome.runtime.sendNativeMessage(NATIVE_HOST_NAME, message, (response) => {
      const err = chrome.runtime.lastError;
      if (err) {
        reject(new Error(err.message ?? "native messaging failed"));
        return;
      }
      if (!response || typeof response !== "object") {
        reject(new Error("native host returned no response"));
        return;
      }
      // Rust serialises `DiscoverResponse` with `#[serde(tag = "status")]`:
      //   { "status": "ok",     "port": ..., "token": ..., "pid": ... }
      //   { "status": "no-app" }
      if (response.status === "no-app") {
        reject(new Error("Simple Password Manager is not running"));
        return;
      }
      if (
        response.status === "ok" &&
        typeof response.port === "number" &&
        typeof response.token === "string" &&
        typeof response.pid === "number"
      ) {
        resolve({
          port: response.port,
          token: response.token,
          pid: response.pid,
        });
      } else {
        reject(new Error("unexpected native host response shape"));
      }
    });
  });
}

async function getBridge(force = false): Promise<BridgeInfo> {
  const fresh = Date.now() - cacheLoadedAt < CACHE_TTL_MS;
  if (!force && cached && fresh) return cached;
  const info = await discoverBridge();
  cached = info;
  cacheLoadedAt = Date.now();
  return info;
}

async function doFetch<T>(
  method: "GET" | "POST",
  path: string,
  body?: unknown,
): Promise<BgResponse<T>> {
  let info: BridgeInfo;
  try {
    info = await getBridge(false);
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }

  const url = `http://127.0.0.1:${info.port}${path}`;
  try {
    const res = await fetch(url, {
      method,
      headers: {
        Authorization: `Bearer ${info.token}`,
        ...(body !== undefined ? { "content-type": "application/json" } : {}),
      },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (res.status === 401) {
      // Token might have rotated. Force-rediscover once and retry.
      try {
        await getBridge(true);
        return doFetch(method, path, body);
      } catch {
        return { ok: false, status: 401, error: "unauthorized" };
      }
    }
    if (!res.ok) {
      return { ok: false, status: res.status, error: `HTTP ${res.status}` };
    }

    const contentType = res.headers.get("content-type") ?? "";
    const data = contentType.includes("application/json")
      ? ((await res.json()) as T)
      : ((await res.text()) as unknown as T);
    return { ok: true, status: res.status, data };
  } catch (e) {
    // fetch() throws on network/DNS errors. The bridge is local, so a fetch
    // failure almost always means the Tauri app isn't running.
    cached = null;
    return { ok: false, error: (e as Error).message };
  }
}

type IncomingMessage = BgMessage | CaptureMessage;

chrome.runtime.onMessage.addListener(
  (msg: IncomingMessage, _sender, sendResponse) => {
    (async () => {
      try {
        if (msg.kind === "get-bridge") {
          const info = await getBridge(msg.force ?? false);
          sendResponse({ ok: true, data: info });
        } else if (msg.kind === "fetch") {
          const resp = await doFetch(msg.method, msg.path, msg.body);
          // Piggy-back: if the popup just polled /v1/status and the vault
          // is unlocked, kick a known-hosts refresh.
          if (
            msg.path === "/v1/status" &&
            resp.ok &&
            (resp.data as { unlocked?: boolean })?.unlocked
          ) {
            maybeRefreshAfterStatus(true);
          }
          sendResponse(resp);
        } else if (msg.kind === "capture") {
          await handleCapture(msg);
          sendResponse({ ok: true });
        } else if (msg.kind === "get-pending-capture") {
          const res = await chrome.storage.local.get(PENDING_CAPTURE_KEY);
          sendResponse({ ok: true, data: res?.[PENDING_CAPTURE_KEY] ?? null });
        } else if (msg.kind === "consume-pending-capture") {
          await chrome.storage.local.remove(PENDING_CAPTURE_KEY);
          sendResponse({ ok: true });
        } else {
          sendResponse({ ok: false, error: "unknown message kind" });
        }
      } catch (e) {
        sendResponse({ ok: false, error: (e as Error).message });
      }
    })();
    return true;
  },
);

// Push notification to every loaded tab when a new capture is stashed, so
// the content scripts on already-open tabs can render the banner without
// polling. (chrome.storage.onChanged only fires inside contexts that have
// storage access — content scripts can't rely on it for `session`.)
function broadcastCaptureAvailable(): void {
  chrome.tabs.query({}, (tabs) => {
    for (const tab of tabs) {
      if (!tab.id) continue;
      chrome.tabs
        .sendMessage(tab.id, { kind: "capture-available" })
        .catch(() => {
          /* tab may not have a content script loaded — ignore */
        });
    }
  });
}

// Stash a captured login in session storage so the popup can pick it up on
// next open. Also paint a "+" badge on the extension icon so the user knows
// something's waiting.
async function handleCapture(msg: CaptureMessage): Promise<void> {
  await chrome.storage.local.set({
    [PENDING_CAPTURE_KEY]: {
      domain: msg.domain,
      url: msg.url,
      username: msg.username,
      password: msg.password,
      capturedAt: Date.now(),
      intent: msg.intent,
    },
  });
  await setBadge("+");
  broadcastCaptureAvailable();
}

// ---------- known-hosts cache ----------
//
// Lightweight: only hostnames (no usernames/passwords/titles), enough for
// the content script to decide whether to draw its in-field indicator.

let knownHostsRefreshInFlight: Promise<void> | null = null;

async function refreshKnownHostsIfNeeded(force = false): Promise<void> {
  if (knownHostsRefreshInFlight) return knownHostsRefreshInFlight;

  if (!force) {
    const stored = await chrome.storage.local.get(KNOWN_HOSTS_UPDATED_AT_KEY);
    const updatedAt = (stored?.[KNOWN_HOSTS_UPDATED_AT_KEY] as number) ?? 0;
    if (Date.now() - updatedAt < KNOWN_HOSTS_TTL_MS) return;
  }

  knownHostsRefreshInFlight = (async () => {
    try {
      const resp = await doFetch<unknown[]>("GET", "/v1/entries");
      // Locked vault returns 423; not an error, just nothing to refresh.
      if (!resp.ok || !Array.isArray(resp.data)) return;
      const hosts = new Set<string>();
      for (const raw of resp.data) {
        const entry = raw as { url?: string; title?: string };
        const u = (entry.url ?? "").trim();
        if (u) {
          // Try as a URL first.
          let host: string | null = null;
          try {
            host = new URL(u.includes("://") ? u : `https://${u}`).hostname;
          } catch {
            host = u.split("/")[0] ?? null;
          }
          if (host) hosts.add(host.replace(/^www\./, "").toLowerCase());
        }
        // Best-effort secondary: a domain-looking token inside the title
        // ("github.com Personal" → github.com).
        const titleMatch = (entry.title ?? "").match(
          /([a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z]{2,})+)/i,
        );
        if (titleMatch?.[1]) hosts.add(titleMatch[1].toLowerCase());
      }
      await chrome.storage.local.set({
        [KNOWN_HOSTS_KEY]: Array.from(hosts),
        [KNOWN_HOSTS_UPDATED_AT_KEY]: Date.now(),
      });
    } catch {
      /* keep the previous cache around */
    } finally {
      knownHostsRefreshInFlight = null;
    }
  })();
  return knownHostsRefreshInFlight;
}

// Refresh after every status check that reports "unlocked" (the popup
// triggers one every time the user clicks the icon).
function maybeRefreshAfterStatus(unlocked: boolean): void {
  if (!unlocked) return;
  void refreshKnownHostsIfNeeded(false);
}

async function setBadge(text: string): Promise<void> {
  try {
    await chrome.action.setBadgeBackgroundColor({ color: "#6366f1" });
    await chrome.action.setBadgeText({ text });
  } catch {
    /* MV3 minor api drift; nothing actionable here */
  }
}

// Badge tracks pending captures. Either the BG itself writes them (when
// the capture arrives via runtime message) or the content script writes
// directly to storage.local (fire-and-forget reliability) — either way we
// update the badge from the same storage event.
chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName !== "local") return;
  if (!(PENDING_CAPTURE_KEY in changes)) return;
  const newValue = changes[PENDING_CAPTURE_KEY]?.newValue;
  if (newValue === undefined) {
    void setBadge("");
  } else {
    void setBadge("+");
    broadcastCaptureAvailable();
  }
});

// On install / start-up, make sure the badge reflects whatever was already
// stashed in storage. Service workers can be re-spawned cold.
async function restoreBadge() {
  const v = await chrome.storage.local.get(PENDING_CAPTURE_KEY);
  if (v?.[PENDING_CAPTURE_KEY]) {
    void setBadge("+");
  } else {
    void setBadge("");
  }
}
chrome.runtime.onStartup.addListener(restoreBadge);
chrome.runtime.onInstalled.addListener(restoreBadge);
void restoreBadge();
