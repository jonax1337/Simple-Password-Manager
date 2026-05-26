// "Saved login" pill placed inside the login input on its right edge —
// 1Password style. Clicking the pill opens the picker (see picker.ts)
// which always asks the user to confirm which account to fill, even
// when there's only one match.

import { isSameSite } from "./domain";

const KNOWN_HOSTS_KEY = "known_hosts";
const PILL_SIZE = 22;
const PILL_MARGIN_FROM_EDGE = 6;

type AttachState = {
  input: HTMLInputElement;
  host: HTMLDivElement;
  shadow: ShadowRoot;
  resizeObserver: ResizeObserver;
  mutationObserver: MutationObserver;
  reposition: () => void;
  originalPaddingRight: string;
};

const attached = new WeakMap<HTMLInputElement, AttachState>();
const trackedInputs = new Set<HTMLInputElement>();
let knownHostsCache: Set<string> = new Set();

export async function refreshKnownHostsCache(): Promise<void> {
  try {
    const res = await chrome.storage.local.get(KNOWN_HOSTS_KEY);
    const raw = res?.[KNOWN_HOSTS_KEY];
    knownHostsCache = new Set(Array.isArray(raw) ? (raw as string[]) : []);
  } catch {
    knownHostsCache = new Set();
  }
}

export function pageHasSavedLogin(): boolean {
  const current = window.location.hostname;
  if (!current) return false;
  for (const h of knownHostsCache) {
    if (isSameSite(current, h)) return true;
  }
  return false;
}

export function attachIndicator(
  input: HTMLInputElement,
  onClick: () => void,
): void {
  if (attached.has(input)) return;
  if (!document.body) return;

  // Reserve a little space at the right edge of the input so the pill
  // doesn't overlap with typed content. We remember the original value
  // and restore it on detach.
  const originalPaddingRight = input.style.paddingRight;
  const currentPad = parseFloat(
    window.getComputedStyle(input).paddingRight || "0",
  );
  const desiredPad = PILL_SIZE + PILL_MARGIN_FROM_EDGE * 2;
  if (currentPad < desiredPad) {
    input.style.paddingRight = `${desiredPad}px`;
  }

  const host = document.createElement("div");
  Object.assign(host.style, {
    position: "fixed",
    zIndex: "2147483647",
    pointerEvents: "none",
    fontFamily:
      'system-ui, -apple-system, BlinkMacSystemFont, "Inter Variable", "Inter", "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    opacity: "0",
    transition: "opacity 200ms ease-out, transform 200ms ease-out",
    transform: "scale(0.85)",
  } satisfies Partial<CSSStyleDeclaration>);

  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = `
    <style>${pillCss()}</style>
    <button class="pill" type="button" aria-label="Fill with Simple Password Manager">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
        <path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3"/>
      </svg>
    </button>
  `;
  document.body.appendChild(host);

  const button = shadow.querySelector<HTMLButtonElement>(".pill")!;
  button.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    button.classList.add("pressed");
    setTimeout(() => button.classList.remove("pressed"), 180);
    // Blur the input so the browser's autofill dropdown closes before
    // we show our own picker.
    input.blur();
    onClick();
  });
  button.addEventListener("mousedown", (e) => e.stopPropagation());

  const reposition = () => {
    const rect = input.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
      host.style.opacity = "0";
      host.style.pointerEvents = "none";
      return;
    }
    host.style.opacity = "1";
    host.style.pointerEvents = "auto";
    host.style.transform = "scale(1)";
    host.style.left = `${rect.right - PILL_SIZE - PILL_MARGIN_FROM_EDGE}px`;
    host.style.top = `${rect.top + (rect.height - PILL_SIZE) / 2}px`;
  };

  reposition();
  const resizeObserver = new ResizeObserver(reposition);
  resizeObserver.observe(input);
  window.addEventListener("scroll", reposition, true);
  window.addEventListener("resize", reposition);

  const mutationObserver = new MutationObserver(() => {
    if (!document.contains(input)) {
      detachIndicator(input);
    }
  });
  mutationObserver.observe(document.documentElement, {
    subtree: true,
    childList: true,
  });

  attached.set(input, {
    input,
    host,
    shadow,
    resizeObserver,
    mutationObserver,
    reposition,
    originalPaddingRight,
  });
  trackedInputs.add(input);
}

export function detachIndicator(input: HTMLInputElement): void {
  const state = attached.get(input);
  if (!state) return;
  state.resizeObserver.disconnect();
  state.mutationObserver.disconnect();
  window.removeEventListener("scroll", state.reposition, true);
  window.removeEventListener("resize", state.reposition);
  state.host.style.opacity = "0";
  state.host.style.transform = "scale(0.85)";
  // Restore the input's original padding once the fade is done.
  try {
    state.input.style.paddingRight = state.originalPaddingRight;
  } catch {
    /* input might already be detached */
  }
  setTimeout(() => state.host.remove(), 220);
  attached.delete(input);
  trackedInputs.delete(input);
}

export function detachAllIndicators(): void {
  for (const input of Array.from(trackedInputs)) {
    detachIndicator(input);
  }
}

function pillCss(): string {
  return `
    :host { all: initial; }
    .pill {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: ${PILL_SIZE}px;
      height: ${PILL_SIZE}px;
      border-radius: 6px;
      border: 1px solid hsl(250 55% 45%);
      background: hsl(250 55% 50%);
      color: #fff;
      cursor: pointer;
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.10);
      transition: background 120ms ease, transform 120ms ease;
      padding: 0;
    }
    .pill:hover {
      background: hsl(250 55% 45%);
    }
    .pill:active,
    .pill.pressed {
      transform: scale(0.95);
    }
    svg { width: 12px; height: 12px; }
  `;
}
