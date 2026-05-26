// In-field "saved login available" indicator — 1Password style, but
// positioned OUTSIDE the input so it never collides with Chrome's native
// autofill icon. Floats as a small chip just to the right of (or below,
// if no horizontal room) the input, with a smooth fade+slide entrance.
//
// We don't touch the host page's CSS; the chip lives in a Shadow DOM and
// uses position: fixed with its anchor recomputed on scroll/resize/
// layout-change so it stays glued to its input.

import { isSameSite } from "./domain";

const KNOWN_HOSTS_KEY = "known_hosts";
const CHIP_WIDTH = 132;
const CHIP_HEIGHT = 34;

type AttachState = {
  input: HTMLInputElement;
  host: HTMLDivElement;
  shadow: ShadowRoot;
  resizeObserver: ResizeObserver;
  mutationObserver: MutationObserver;
  reposition: () => void;
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

  const host = document.createElement("div");
  Object.assign(host.style, {
    position: "fixed",
    zIndex: "2147483647",
    pointerEvents: "none",
    fontFamily:
      'system-ui, -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    opacity: "0",
    transition: "opacity 200ms ease-out, transform 200ms ease-out",
    transform: "translateX(-4px)",
  } satisfies Partial<CSSStyleDeclaration>);

  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = `
    <style>${chipCss()}</style>
    <button class="chip" type="button" aria-label="Fill with Simple Password Manager">
      <span class="icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3"/>
        </svg>
      </span>
      <span class="text">Saved login</span>
      <span class="caret" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="9 18 15 12 9 6"/>
        </svg>
      </span>
    </button>
  `;
  document.body.appendChild(host);

  const button = shadow.querySelector<HTMLButtonElement>(".chip")!;
  button.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    button.classList.add("pressed");
    setTimeout(() => button.classList.remove("pressed"), 180);
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

    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    const margin = 6;

    let top: number;
    let left: number;
    let placement: "right" | "below" | "above" = "right";

    // Prefer placing to the right of the input — out of Chrome's own
    // autofill icon's territory.
    if (rect.right + margin + CHIP_WIDTH <= viewportW) {
      placement = "right";
      left = rect.right + margin;
      top = rect.top + (rect.height - CHIP_HEIGHT) / 2;
    } else if (rect.bottom + margin + CHIP_HEIGHT <= viewportH) {
      placement = "below";
      left = Math.max(
        4,
        Math.min(rect.left, viewportW - CHIP_WIDTH - 4),
      );
      top = rect.bottom + margin;
    } else {
      placement = "above";
      left = Math.max(
        4,
        Math.min(rect.left, viewportW - CHIP_WIDTH - 4),
      );
      top = rect.top - CHIP_HEIGHT - margin;
    }

    host.style.opacity = "1";
    host.style.pointerEvents = "auto";
    host.style.transform = "translateX(0)";
    host.style.left = `${left}px`;
    host.style.top = `${top}px`;
    host.dataset.placement = placement;
  };

  reposition();
  const resizeObserver = new ResizeObserver(reposition);
  resizeObserver.observe(input);
  window.addEventListener("scroll", reposition, true);
  window.addEventListener("resize", reposition);

  // Self-removal if the input gets detached from the DOM.
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
  // Allow fade-out to play before removing.
  setTimeout(() => state.host.remove(), 220);
  attached.delete(input);
  trackedInputs.delete(input);
}

export function detachAllIndicators(): void {
  for (const input of Array.from(trackedInputs)) {
    detachIndicator(input);
  }
}

function chipCss(): string {
  return `
    :host { all: initial; }
    .chip {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      width: ${CHIP_WIDTH}px;
      height: ${CHIP_HEIGHT}px;
      padding: 0 10px 0 8px;
      border-radius: 10px;
      border: 1px solid transparent;
      background: linear-gradient(135deg, #6366f1 0%, #7c3aed 100%);
      color: #fff;
      font-family: inherit;
      font-size: 12px;
      font-weight: 500;
      letter-spacing: 0.005em;
      cursor: pointer;
      box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.08),
        0 8px 24px -10px rgba(99, 102, 241, 0.55);
      transition:
        background-position 220ms ease,
        transform 140ms ease,
        box-shadow 220ms ease,
        filter 160ms ease;
      background-size: 200% 100%;
    }
    .chip:hover {
      background-position: right center;
      box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.08),
        0 12px 30px -10px rgba(99, 102, 241, 0.75);
      transform: translateY(-1px);
    }
    .chip:active,
    .chip.pressed {
      transform: translateY(0) scale(0.97);
      filter: brightness(0.95);
    }
    .icon, .caret {
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }
    .icon svg { width: 13px; height: 13px; }
    .caret svg { width: 11px; height: 11px; opacity: 0.85; }
    .text {
      flex: 1;
      min-width: 0;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      text-align: left;
      text-shadow: 0 1px 1px rgba(0, 0, 0, 0.18);
    }
    /* When attached below or above the input, point the chip a touch with
       a soft glow on the field-facing side. */
    :host([data-placement="below"]) .chip {
      border-top-left-radius: 6px;
    }
    :host([data-placement="above"]) .chip {
      border-bottom-left-radius: 6px;
    }
    @media (prefers-color-scheme: dark) {
      .chip {
        box-shadow:
          0 1px 2px rgba(0, 0, 0, 0.4),
          0 12px 30px -10px rgba(99, 102, 241, 0.55);
      }
    }
  `;
}
