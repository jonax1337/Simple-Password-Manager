// Content script: runs on every page, listens for fill requests from the
// popup, and writes the credentials into the most likely login form on
// the active document.

interface FillRequest {
  kind: "fill";
  username: string;
  password: string;
}

interface FillResponse {
  filled: { username: boolean; password: boolean };
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if ((msg as FillRequest)?.kind !== "fill") return false;
  const { username, password } = msg as FillRequest;
  const result = fillCurrentForm(username, password);
  sendResponse(result satisfies FillResponse);
  return false; // synchronous response, no need to keep the port open
});

function fillCurrentForm(
  username: string,
  password: string,
): FillResponse {
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

// Find the most likely password input — visible, type=password, biggest if multiple.
function findPasswordField(): HTMLInputElement | null {
  const candidates = Array.from(
    document.querySelectorAll<HTMLInputElement>("input[type=password]"),
  ).filter(isVisible);

  if (candidates.length === 0) return null;
  // Heuristic: largest visible password field wins (handles double "confirm
  // password" forms by preferring the bigger / first one).
  candidates.sort((a, b) => boundingArea(b) - boundingArea(a));
  return candidates[0];
}

// Find the username/email field — usually the first visible text-like input
// before the password field in DOM order.
function findUsernameField(
  passwordField: HTMLInputElement | null,
): HTMLInputElement | null {
  const all = Array.from(
    document.querySelectorAll<HTMLInputElement>("input"),
  ).filter(isVisible);

  // Build the ordered list of inputs that come before the password field;
  // if there's no password field, use all of them.
  const beforePassword = passwordField
    ? all.slice(
        0,
        all.findIndex((el) => el === passwordField),
      )
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

  // Prefer email > text by autocomplete/name hint, otherwise last text input
  // before password.
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

// Setting `.value` directly skips React's controlled-input synthetic event
// machinery — frameworks won't see the change. Use the native property
// setter and dispatch an input event so React et al. pick it up.
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
