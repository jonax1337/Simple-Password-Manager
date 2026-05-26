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

// Content scripts can't see chrome.storage.session unless the extension
// explicitly opens it up. Without this, maybeShowFromPending() in the
// content script returns nothing — the banner appears once on the login
// page and never resurfaces on the MFA / 2FA destination page.
void (async () => {
  try {
    await chrome.storage.session.setAccessLevel({
      accessLevel: "TRUSTED_AND_UNTRUSTED_CONTEXTS",
    });
  } catch {
    /* older Chrome versions reject the call — fail-safe, just means the
       inline banner on the next page won't appear, but the popup-side
       save banner still works. */
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
          sendResponse(resp);
        } else if (msg.kind === "capture") {
          await handleCapture(msg);
          sendResponse({ ok: true });
        } else if (msg.kind === "get-pending-capture") {
          const res = await chrome.storage.session.get(PENDING_CAPTURE_KEY);
          sendResponse({ ok: true, data: res?.[PENDING_CAPTURE_KEY] ?? null });
        } else if (msg.kind === "consume-pending-capture") {
          await chrome.storage.session.remove(PENDING_CAPTURE_KEY);
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
  await chrome.storage.session.set({
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

async function setBadge(text: string): Promise<void> {
  try {
    await chrome.action.setBadgeBackgroundColor({ color: "#6366f1" });
    await chrome.action.setBadgeText({ text });
  } catch {
    /* MV3 minor api drift; nothing actionable here */
  }
}

// Clear the badge whenever the popup empties the pending capture.
chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName !== "session") return;
  if (!(PENDING_CAPTURE_KEY in changes)) return;
  const newValue = changes[PENDING_CAPTURE_KEY]?.newValue;
  if (newValue === undefined) {
    void setBadge("");
  }
});

// On install / start-up, make sure the badge reflects whatever was already
// stashed in storage. Service workers can be re-spawned cold.
async function restoreBadge() {
  const v = await chrome.storage.session.get(PENDING_CAPTURE_KEY);
  if (v?.[PENDING_CAPTURE_KEY]) {
    void setBadge("+");
  } else {
    void setBadge("");
  }
}
chrome.runtime.onStartup.addListener(restoreBadge);
chrome.runtime.onInstalled.addListener(restoreBadge);
void restoreBadge();
