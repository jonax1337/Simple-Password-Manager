// In-field "saved login available" indicator.
//
// 1Password-style: when the BG's known-hosts cache says we have an entry
// for the current domain, attach a small floating key icon at the right
// edge of every username/password input. Click the icon to fill (vault
// unlocked) or to bring the desktop app to the front (locked).
//
// We don't touch the host page's CSS; the icon lives in a shadow DOM and
// is `position: fixed` with its left/top recomputed on scroll/resize/
// layout-change so it visually sticks to its input.

import { baseDomain, isSameSite } from "./domain";

const KNOWN_HOSTS_KEY = "known_hosts";

type AttachState = {
  input: HTMLInputElement;
  host: HTMLDivElement;
  shadow: ShadowRoot;
  resizeObserver: ResizeObserver;
};

const attached = new WeakMap<HTMLInputElement, AttachState>();
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

/** True if the current page's host matches any cached saved-login host. */
export function pageHasSavedLogin(): boolean {
  const current = window.location.hostname;
  if (!current) return false;
  for (const h of knownHostsCache) {
    if (isSameSite(current, h)) return true;
  }
  return false;
}

/**
 * Attach the floating indicator to the given input. Idempotent — calling
 * twice on the same input is a no-op the second time. Removes itself if
 * the input is unmounted.
 */
export function attachIndicator(
  input: HTMLInputElement,
  onClick: () => void,
): void {
  if (attached.has(input)) return;
  if (!document.body) return;

  const host = document.createElement("div");
  Object.assign(host.style, {
    position: "fixed",
    zIndex: "2147483647",
    pointerEvents: "none",
  } satisfies Partial<CSSStyleDeclaration>);
  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = `
    <style>
      :host { all: initial; }
      .btn {
        pointer-events: auto;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 22px; height: 22px;
        border-radius: 6px;
        border: 1px solid rgba(99, 102, 241, 0.4);
        background: rgba(99, 102, 241, 0.12);
        color: #6366f1;
        cursor: pointer;
        transition: background 120ms ease, transform 120ms ease;
        box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
      }
      .btn:hover {
        background: rgba(99, 102, 241, 0.22);
        transform: scale(1.05);
      }
      @media (prefers-color-scheme: dark) {
        .btn { background: rgba(99,102,241,0.20); border-color: rgba(99,102,241,0.5); }
      }
      svg { width: 12px; height: 12px; }
    </style>
    <button class="btn" type="button" aria-label="Fill with Simple Password Manager">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3"/>
      </svg>
    </button>
  `;
  document.body.appendChild(host);

  const button = shadow.querySelector("button")!;
  button.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    onClick();
  });
  // Don't let the host page steal mousedown events.
  button.addEventListener("mousedown", (e) => e.stopPropagation());

  const reposition = () => {
    const rect = input.getBoundingClientRect();
    // Hide off-screen or invisible inputs.
    if (rect.width === 0 || rect.height === 0) {
      host.style.display = "none";
      return;
    }
    host.style.display = "block";
    // Anchor flush against the right inner edge, vertically centered.
    const size = 22;
    host.style.left = `${rect.right - size - 4}px`;
    host.style.top = `${rect.top + (rect.height - size) / 2}px`;
  };

  reposition();
  const resizeObserver = new ResizeObserver(reposition);
  resizeObserver.observe(input);
  window.addEventListener("scroll", reposition, true);
  window.addEventListener("resize", reposition);

  attached.set(input, { input, host, shadow, resizeObserver });

  // Self-removal if the input gets detached from the DOM.
  const mo = new MutationObserver(() => {
    if (!document.contains(input)) {
      detachIndicator(input);
      mo.disconnect();
    }
  });
  mo.observe(document.documentElement, { subtree: true, childList: true });
}

export function detachIndicator(input: HTMLInputElement): void {
  const state = attached.get(input);
  if (!state) return;
  state.resizeObserver.disconnect();
  state.host.remove();
  attached.delete(input);
}

export function detachAllIndicators(): void {
  // WeakMap can't be iterated; track via a side-channel set instead.
  // Cheaper: iterate document and call detachIndicator on every input.
  document.querySelectorAll("input").forEach((el) => {
    if (el instanceof HTMLInputElement) detachIndicator(el);
  });
}

// re-export for tests / debugging
export const __debug = {
  baseDomain,
};
