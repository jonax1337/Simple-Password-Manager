// Content script: runs on every page.
//
// Three jobs:
//   1. Form-fill on demand (popup sends a "fill" message with credentials).
//   2. Capture submitted logins (real submits, Enter in a password field,
//      button clicks whose label matches sign-in/up patterns).
//   3. Inline page banner: "Save this login?" injected via Shadow DOM, on
//      the current page (best-effort) AND on the next page (post-nav).
//
// Logs are prefixed with `[SPM]` so the user can grep DevTools.

import { detectIntent, type Intent } from "./detect-intent";
import { showInlineBanner } from "./inline-banner";
import { baseDomain, isSameSite } from "./domain";

const LOG = "[SPM]";

interface FillMessage {
  kind: "fill";
  username: string;
  password: string;
}

interface CaptureMessage {
  kind: "capture";
  domain: string;
  url: string;
  username: string;
  password: string;
  intent: Intent;
}

console.debug(LOG, "content script loaded on", window.location.hostname);

// Blocklist gets loaded into memory once at startup so the submit-time path
// can decide synchronously, no await before showing the banner.
let blocklistCache: Set<string> = new Set();
void (async () => {
  try {
    const result = await chrome.storage.local.get("save_blocklist");
    if (Array.isArray(result?.save_blocklist)) {
      blocklistCache = new Set(result.save_blocklist as string[]);
    }
  } catch {
    /* default to empty */
  }
})();
chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "local") return;
  if (!("save_blocklist" in changes)) return;
  const next = changes.save_blocklist?.newValue;
  blocklistCache = new Set(Array.isArray(next) ? (next as string[]) : []);
});

// ---------------- pending banner on next page load ----------------
//
// When a submit causes a navigation, the banner injected on the old page
// is gone. The BG-stashed capture surfaces here on the destination page.

async function maybeShowFromPending(): Promise<void> {
  if (!chrome.storage?.session) return;
  const res = await chrome.storage.session.get("pending_capture");
  const cap = res?.pending_capture as
    | undefined
    | {
        domain: string;
        url: string;
        username: string;
        password: string;
        capturedAt: number;
        intent?: Intent;
      };
  if (!cap) return;
  // 5 minutes is long enough for a sluggish MFA challenge or a captcha,
  // short enough that the stale token from a previous session doesn't pop
  // up randomly when the user revisits the site days later.
  if (Date.now() - (cap.capturedAt ?? 0) > 5 * 60_000) return;

  const currentHost = window.location.hostname;
  if (!isSameSite(currentHost, cap.domain)) {
    console.debug(LOG, "pending capture skipped (different site):", {
      current: baseDomain(currentHost),
      captured: baseDomain(cap.domain),
    });
    return;
  }
  if (blocklistCache.has(cap.domain)) return;

  console.debug(LOG, "resurrecting banner from pending capture on", currentHost);
  showInlineBanner({
    domain: cap.domain,
    url: cap.url,
    username: cap.username,
    password: cap.password,
    intent: cap.intent ?? "login",
  });
}

void maybeShowFromPending();

// Also listen for changes to session storage in case the BG hasn't written
// the capture yet by the time the content script first checks above.
chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "session") return;
  if (!("pending_capture" in changes)) return;
  if (changes.pending_capture?.newValue === undefined) return;
  void maybeShowFromPending();
});

// ---------------- fill on demand ----------------

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if ((msg as FillMessage)?.kind !== "fill") return false;
  const { username, password } = msg as FillMessage;
  const result = fillCurrentForm(username, password);
  sendResponse(result);
  return false;
});

// ---------------- capture on submit ----------------

let lastSentHash: string | null = null;
let lastSentAt = 0;

// Synchronous version — safe to call from a submit handler where the page
// may navigate within the same tick. Reads all DOM values up front.
function captureNow(reason: string): void {
  const passwordField = findPasswordField();
  if (!passwordField || !passwordField.value) {
    console.debug(LOG, "capture skipped (no password field):", reason);
    return;
  }
  if (looksLikeOtpField(passwordField, passwordField.value)) {
    console.debug(LOG, "capture skipped (looks like an OTP/MFA field)");
    return;
  }
  const usernameField = findUsernameField(passwordField);
  const username = usernameField?.value ?? "";
  const password = passwordField.value;
  if (!password) return;

  const url = window.location.href;
  let domain = window.location.hostname;
  try {
    domain = new URL(url).hostname.replace(/^www\./, "");
  } catch {
    /* keep raw hostname */
  }

  if (blocklistCache.has(domain)) {
    console.debug(LOG, "capture skipped (domain blocklisted):", domain);
    return;
  }

  const hash = `${domain}|${username}|${password}`;
  const now = Date.now();
  if (hash === lastSentHash && now - lastSentAt < 5000) {
    console.debug(LOG, "capture skipped (duplicate within 5s)");
    return;
  }
  lastSentHash = hash;
  lastSentAt = now;

  const intent = detectIntent(passwordField);
  console.debug(LOG, "captured", { reason, domain, intent, username });

  const payload: CaptureMessage = {
    kind: "capture",
    domain,
    url,
    username,
    password,
    intent,
  };

  // Fire-and-forget to BG so the popup also has access. The .catch is
  // there because the BG can refuse messages from disappearing tabs.
  chrome.runtime.sendMessage(payload).catch((err) => {
    console.debug(LOG, "sendMessage(capture) error:", err);
  });

  // Inject the banner synchronously on the current page. Will be torn down
  // if the page navigates; the BG-stashed capture above gives us a
  // second chance on the destination page via maybeShowFromPending().
  try {
    showInlineBanner({ domain, url, username, password, intent });
  } catch (e) {
    console.debug(LOG, "showInlineBanner threw:", e);
  }
}

// Real <form> submissions.
document.addEventListener(
  "submit",
  (event) => {
    const form = event.target as HTMLElement | null;
    if (!(form instanceof HTMLFormElement)) return;
    if (!form.querySelector("input[type=password]")) return;
    captureNow("form submit");
  },
  true, // capture phase, run before the form may detach
);

// Enter in a password field — covers SPA forms that use fetch() instead of
// a real submit.
document.addEventListener(
  "keydown",
  (event) => {
    if (event.key !== "Enter") return;
    const target = event.target as HTMLElement | null;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.type !== "password") return;
    captureNow("Enter in password field");
  },
  true,
);

// Clicks on buttons whose visible label matches login/signup verbs.
document.addEventListener(
  "click",
  (event) => {
    const el = event.target as HTMLElement | null;
    if (!el) return;
    const btn = el.closest("button, [role=button], input[type=submit], a");
    if (!btn) return;
    const label = readableLabel(btn as HTMLElement);
    if (!label) return;
    if (
      !/log\s*in|log\s+on|sign\s*in|sign\s*on|sign\s*up|signin|signup|continue|submit|anmelden|einloggen|registrieren|konto\s+erstellen/i.test(
        label,
      )
    ) {
      return;
    }
    captureNow(`button click "${label.slice(0, 40)}"`);
  },
  true,
);

function readableLabel(el: HTMLElement): string {
  const parts: string[] = [];
  if (el.innerText) parts.push(el.innerText);
  const aria = el.getAttribute("aria-label");
  if (aria) parts.push(aria);
  const value = el.getAttribute("value");
  if (value) parts.push(value);
  const title = el.getAttribute("title");
  if (title) parts.push(title);
  return parts.join(" ").trim();
}

// ---------------- fill implementation ----------------

function fillCurrentForm(username: string, password: string) {
  const passwordField = findPasswordField();
  const usernameField = findUsernameField(passwordField);

  const filled = { username: false, password: false };

  if (passwordField) {
    setNativeValue(passwordField, password);
    filled.password = true;
  }
  if (usernameField) {
    setNativeValue(usernameField, username);
    filled.username = true;
  }

  return { filled };
}

function findPasswordField(): HTMLInputElement | null {
  const candidates = Array.from(
    document.querySelectorAll<HTMLInputElement>("input[type=password]"),
  ).filter(isVisible);
  if (candidates.length === 0) return null;
  candidates.sort((a, b) => boundingArea(b) - boundingArea(a));
  return candidates[0];
}

function findUsernameField(
  passwordField: HTMLInputElement | null,
): HTMLInputElement | null {
  const all = Array.from(
    document.querySelectorAll<HTMLInputElement>("input"),
  ).filter(isVisible);

  const beforePassword = passwordField
    ? all.slice(0, all.findIndex((el) => el === passwordField))
    : all;

  const textLike = beforePassword.filter((el) => {
    const t = (el.type || "text").toLowerCase();
    return (
      t === "text" ||
      t === "email" ||
      t === "tel" ||
      t === "username" ||
      t === ""
    );
  });

  const hinted = textLike.find((el) => {
    const hay = [
      el.autocomplete,
      el.name,
      el.id,
      el.placeholder,
      el.getAttribute("aria-label") ?? "",
    ]
      .join(" ")
      .toLowerCase();
    return /user|email|login|account/.test(hay);
  });
  if (hinted) return hinted;
  return textLike.at(-1) ?? null;
}

function isVisible(el: HTMLElement): boolean {
  if (el.hidden) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const style = window.getComputedStyle(el);
  if (style.visibility === "hidden" || style.display === "none") return false;
  if (parseFloat(style.opacity) < 0.1) return false;
  return true;
}

function boundingArea(el: HTMLElement): number {
  const r = el.getBoundingClientRect();
  return r.width * r.height;
}

// Skip capture when the password field is actually a 2FA / OTP / PIN box.
// Otherwise we'd overwrite the user's just-captured real login with a
// 6-digit code the moment they submit the MFA challenge page.
function looksLikeOtpField(el: HTMLInputElement, value: string): boolean {
  // Pure short-digit values are almost always OTP codes (6 digits standard,
  // sometimes 4 or 8). A real password being all digits and < 10 chars is
  // exceedingly rare and not worth saving anyway.
  const trimmed = value.trim();
  if (/^\d{4,10}$/.test(trimmed)) return true;

  // Standard browser hint for one-time-code inputs.
  const auto = (el.autocomplete ?? "").toLowerCase();
  if (auto.includes("one-time-code")) return true;

  // Field-naming hints. We check name/id/placeholder/aria-label/maxlength.
  const hay = [
    el.name ?? "",
    el.id ?? "",
    el.placeholder ?? "",
    el.getAttribute("aria-label") ?? "",
    el.getAttribute("autocomplete") ?? "",
  ]
    .join(" ")
    .toLowerCase();
  if (/\b(otp|2fa|mfa|one[-_ ]?time|verif(y|ication)|auth[-_ ]?code|token|pin|tan|sms|backup[-_ ]?code|recovery[-_ ]?code)\b/.test(hay)) {
    return true;
  }

  // Short maxlength is another good signal (most OTP fields cap at 6-8).
  const maxLen = el.maxLength;
  if (maxLen > 0 && maxLen <= 10 && /^\d+$/.test(trimmed)) return true;

  return false;
}

function setNativeValue(el: HTMLInputElement, value: string) {
  const proto = Object.getPrototypeOf(el);
  const descriptor = Object.getOwnPropertyDescriptor(proto, "value");
  const nativeSetter = descriptor?.set;
  if (nativeSetter) {
    nativeSetter.call(el, value);
  } else {
    el.value = value;
  }
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
}

