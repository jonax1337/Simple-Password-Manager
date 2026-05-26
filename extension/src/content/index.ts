// Content script: runs on every page.
//
// Three jobs:
//   1. Form-fill on demand (popup sends a "fill" message with credentials).
//   2. Capture submitted logins — listen for form submits and Enter-keypress
//      inside password fields, harvest the values, ship them off to the
//      background worker so the popup can offer "Save this login?" on next
//      open.
//   3. Inline page banner — show a "Save this login?" card right on the
//      page, with sign-up vs sign-in awareness, mirroring 1Password's UX.

import { detectIntent, type Intent } from "./detect-intent";
import { isDomainBlocked } from "./blocklist";
import { showInlineBanner } from "./inline-banner";

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

// ---------------- pending banner on next page load ----------------
//
// When the user submits a login form, the page often navigates away before
// the inline banner had time to show. The BG-stashed capture lets the
// freshly-loaded content script on the destination page resurface the
// banner so the user still gets a one-click save.

void (async () => {
  try {
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
    if (Date.now() - (cap.capturedAt ?? 0) > 60_000) return;

    const currentDomain = window.location.hostname.replace(/^www\./, "");
    if (currentDomain !== cap.domain) return;
    if (await isDomainBlocked(cap.domain)) return;

    showInlineBanner({
      domain: cap.domain,
      url: cap.url,
      username: cap.username,
      password: cap.password,
      intent: cap.intent ?? "login",
    });
  } catch {
    /* non-fatal */
  }
})();

// ---------------- fill on demand ----------------

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if ((msg as FillMessage)?.kind !== "fill") return false;
  const { username, password } = msg as FillMessage;
  const result = fillCurrentForm(username, password);
  sendResponse(result);
  return false;
});

// ---------------- capture on submit ----------------

// To avoid double-captures we throttle by hash of (origin, username, password).
let lastSentHash: string | null = null;
let lastSentAt = 0;

async function maybeCaptureFromActiveForm() {
  const passwordField = findPasswordField();
  if (!passwordField || !passwordField.value) return;
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

  if (await isDomainBlocked(domain)) return;

  const hash = `${domain}|${username}|${password}`;
  const now = Date.now();
  if (hash === lastSentHash && now - lastSentAt < 5000) return;
  lastSentHash = hash;
  lastSentAt = now;

  const intent = detectIntent(passwordField);

  const payload: CaptureMessage = {
    kind: "capture",
    domain,
    url,
    username,
    password,
    intent,
  };
  // Stash in BG (so the popup also picks it up if the user doesn't act on
  // the inline banner). Fire-and-forget — the inline UI is the primary path.
  chrome.runtime.sendMessage(payload).catch(() => {
    /* background may be inactive */
  });

  // Show the inline banner on the current page. It may get torn down if
  // the form's submit causes a navigation, in which case the BG-stashed
  // capture surfaces in the popup as the fallback.
  showInlineBanner({ domain, url, username, password, intent });
}

// Capture on every form submission that has a password field.
document.addEventListener(
  "submit",
  (event) => {
    const form = event.target as HTMLElement | null;
    if (!(form instanceof HTMLFormElement)) return;
    if (!form.querySelector("input[type=password]")) return;
    // Capture synchronously *before* the form posts away — but don't block.
    queueMicrotask(() => void maybeCaptureFromActiveForm());
  },
  true, // capture phase, so we run before the form may detach
);

// Many modern SPA login forms use a button click + fetch instead of a real
// <form>. As a fallback, capture on Enter in a password field, and on any
// click on a button whose visible text smells like "log in"/"sign in".
document.addEventListener(
  "keydown",
  (event) => {
    if (event.key !== "Enter") return;
    const target = event.target as HTMLElement | null;
    if (!(target instanceof HTMLInputElement)) return;
    if (target.type !== "password") return;
    queueMicrotask(() => void maybeCaptureFromActiveForm());
  },
  true,
);

document.addEventListener(
  "click",
  (event) => {
    const el = event.target as HTMLElement | null;
    if (!el) return;
    const btn = el.closest("button, [role=button], input[type=submit]");
    if (!btn) return;
    const label = (
      (btn as HTMLElement).innerText ??
      btn.getAttribute("aria-label") ??
      btn.getAttribute("value") ??
      ""
    )
      .trim()
      .toLowerCase();
    if (!label) return;
    if (!/log\s*in|sign\s*in|sign\s*up|continue|submit|anmelden|einloggen|registrieren/.test(label)) {
      return;
    }
    queueMicrotask(() => void maybeCaptureFromActiveForm());
  },
  true,
);

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
