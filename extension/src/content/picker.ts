// Dropdown picker attached to a login input. Lists matching saved entries
// even when there's only one — 1Password-style "confirm which account".
//
// Lives in its own Shadow DOM, positioned below or above the anchored input
// depending on viewport room. Closes on outside click, Escape, or scroll
// of the host page (so it doesn't visually detach).

const PICKER_ID = "spm-login-picker-host";
const PICKER_WIDTH = 280;
const MAX_HEIGHT = 320;

export interface PickerEntry {
  uuid: string;
  title: string;
  username: string;
  url: string;
}

export interface PickerOptions {
  anchor: HTMLInputElement;
  entries: PickerEntry[];
  hostDomain: string;
  onPick: (entry: PickerEntry) => void;
  onDismiss?: () => void;
}

let activePicker: { host: HTMLDivElement; cleanup: () => void } | null = null;

export function dismissActivePicker(): void {
  if (!activePicker) return;
  const { host, cleanup } = activePicker;
  activePicker = null;
  cleanup();
  host.style.opacity = "0";
  host.style.transform = "translateY(-4px) scale(0.985)";
  setTimeout(() => host.remove(), 160);
}

export function showPicker(opts: PickerOptions): void {
  dismissActivePicker();
  if (!document.body) return;

  const host = document.createElement("div");
  host.id = PICKER_ID;
  Object.assign(host.style, {
    position: "fixed",
    zIndex: "2147483647",
    fontFamily:
      'system-ui, -apple-system, BlinkMacSystemFont, "Inter Variable", "Inter", "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    opacity: "0",
    transform: "translateY(-4px) scale(0.985)",
    transition:
      "opacity 160ms ease-out, transform 200ms cubic-bezier(0.16, 1, 0.3, 1)",
  } satisfies Partial<CSSStyleDeclaration>);

  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = `
    <style>${pickerCss()}</style>
    <div class="card" role="listbox" aria-label="Choose saved login">
      <header class="head">
        <div class="head-title">
          <span class="head-dot"></span>
          <span class="head-label">Saved logins</span>
        </div>
        <span class="head-domain" title="${escapeAttr(opts.hostDomain)}">${escapeHtml(opts.hostDomain)}</span>
      </header>
      <ul class="list" tabindex="-1">
        ${opts.entries
          .map((entry, i) => renderRow(entry, i))
          .join("")}
      </ul>
      <footer class="foot">
        <span>↵ to fill · Esc to close</span>
      </footer>
    </div>
  `;
  document.body.appendChild(host);

  // Compute placement.
  const reposition = () => {
    const rect = opts.anchor.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
      dismissActivePicker();
      return;
    }
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    const margin = 6;

    // Width is fixed at PICKER_WIDTH; left-align with the input but clamp
    // so it doesn't overflow viewport.
    const left = Math.max(
      4,
      Math.min(rect.left, viewportW - PICKER_WIDTH - 4),
    );

    const card = shadow.querySelector<HTMLElement>(".card");
    const cardHeight = card?.offsetHeight ?? 200;

    // Prefer below the input.
    let top: number;
    if (rect.bottom + margin + cardHeight <= viewportH) {
      top = rect.bottom + margin;
    } else if (rect.top - margin - cardHeight >= 4) {
      top = rect.top - margin - cardHeight;
    } else {
      top = Math.max(4, viewportH - cardHeight - 4);
    }

    host.style.left = `${left}px`;
    host.style.top = `${top}px`;
    host.style.width = `${PICKER_WIDTH}px`;
    host.style.opacity = "1";
    host.style.transform = "translateY(0) scale(1)";
  };

  // Two-rAF gives us measured dimensions for cardHeight.
  requestAnimationFrame(() => requestAnimationFrame(reposition));

  // Wiring: row clicks + keyboard nav.
  let focusedIndex = 0;
  const rows = shadow.querySelectorAll<HTMLElement>("[data-uuid]");
  const updateFocus = () => {
    rows.forEach((r, i) => {
      r.dataset.focused = String(i === focusedIndex);
    });
    rows[focusedIndex]?.scrollIntoView({ block: "nearest" });
  };
  updateFocus();

  function pickByIndex(idx: number) {
    const uuid = rows[idx]?.dataset.uuid;
    const entry = opts.entries.find((e) => e.uuid === uuid);
    if (!entry) return;
    dismissActivePicker();
    opts.onPick(entry);
  }

  shadow.addEventListener("click", (e) => {
    e.stopPropagation();
    const row = (e.target as HTMLElement | null)?.closest<HTMLElement>(
      "[data-uuid]",
    );
    if (!row) return;
    const idx = Array.from(rows).indexOf(row);
    pickByIndex(idx);
  });

  const keyHandler = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      dismissActivePicker();
      opts.onDismiss?.();
    } else if (e.key === "Enter") {
      e.preventDefault();
      pickByIndex(focusedIndex);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      focusedIndex = (focusedIndex + 1) % rows.length;
      updateFocus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      focusedIndex = (focusedIndex - 1 + rows.length) % rows.length;
      updateFocus();
    }
  };
  window.addEventListener("keydown", keyHandler, true);

  const outsideClickHandler = (e: MouseEvent) => {
    if (e.target instanceof Node && host.contains(e.target)) return;
    dismissActivePicker();
    opts.onDismiss?.();
  };
  // Defer outside-click hookup so the *same* click that opened us doesn't
  // immediately close us.
  setTimeout(() => {
    window.addEventListener("mousedown", outsideClickHandler, true);
  }, 0);

  const scrollHandler = () => reposition();
  window.addEventListener("scroll", scrollHandler, true);
  window.addEventListener("resize", scrollHandler);

  const cleanup = () => {
    window.removeEventListener("keydown", keyHandler, true);
    window.removeEventListener("mousedown", outsideClickHandler, true);
    window.removeEventListener("scroll", scrollHandler, true);
    window.removeEventListener("resize", scrollHandler);
  };

  activePicker = { host, cleanup };
}

function renderRow(entry: PickerEntry, _idx: number): string {
  const initial = (entry.title.trim()[0] ?? "?").toUpperCase();
  const hue = stableHue(entry.title || entry.username || entry.url);
  const username = entry.username || "—";
  return `
    <li class="row" data-uuid="${escapeAttr(entry.uuid)}" data-focused="false" role="option" tabindex="-1">
      <span class="badge" style="--badge-hue: ${hue};">${escapeHtml(initial)}</span>
      <span class="row-text">
        <span class="row-title">${escapeHtml(entry.title || "(untitled)")}</span>
        <span class="row-sub">${escapeHtml(username)}</span>
      </span>
      <span class="row-fill">Fill</span>
    </li>
  `;
}

function stableHue(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) - h + s.charCodeAt(i)) | 0;
  }
  // Map to a palette of pleasing hues (skip lime/yellow).
  return Math.abs(h) % 360;
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
function escapeAttr(s: string): string {
  return escapeHtml(s).replace(/`/g, "&#x60;");
}

function pickerCss(): string {
  // Plain shadcn vibes — same tokens as the popup.
  return `
    :host { all: initial; }
    .card {
      background: #ffffff;
      color: #18181b;
      border: 1px solid #e4e4e7;
      border-radius: 8px;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.10), 0 2px 4px -2px rgba(0, 0, 0, 0.06);
      overflow: hidden;
      font-size: 13px;
    }
    @media (prefers-color-scheme: dark) {
      .card {
        background: #16142a;
        color: #f4f4f5;
        border-color: #2c2745;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.5), 0 2px 4px -2px rgba(0, 0, 0, 0.3);
      }
    }
    .head {
      display: flex; align-items: center; justify-content: space-between;
      gap: 8px;
      padding: 10px 12px;
      border-bottom: 1px solid #e4e4e7;
    }
    @media (prefers-color-scheme: dark) {
      .head { border-bottom-color: #2c2745; }
    }
    .head-title {
      display: flex; align-items: center; gap: 6px;
      font-size: 12px; font-weight: 600;
      color: inherit;
    }
    .head-dot { display: none; }
    .head-domain {
      font-size: 11px; color: #71717a;
      max-width: 140px; overflow: hidden; text-overflow: ellipsis;
      white-space: nowrap;
    }
    @media (prefers-color-scheme: dark) {
      .head-domain { color: #a1a1aa; }
    }

    .list {
      list-style: none; margin: 0; padding: 4px;
      max-height: ${MAX_HEIGHT}px; overflow-y: auto;
    }
    .list::-webkit-scrollbar { width: 6px; }
    .list::-webkit-scrollbar-thumb {
      background: #d4d4d8; border-radius: 3px;
    }
    @media (prefers-color-scheme: dark) {
      .list::-webkit-scrollbar-thumb { background: #3f3a5e; }
    }
    .row {
      display: flex; align-items: center; gap: 10px;
      padding: 8px 10px;
      border-radius: 6px;
      cursor: pointer;
      transition: background 120ms ease;
    }
    .row:hover,
    .row[data-focused="true"] {
      background: #f4f4f5;
    }
    @media (prefers-color-scheme: dark) {
      .row:hover,
      .row[data-focused="true"] {
        background: #27243f;
      }
    }
    .row + .row { margin-top: 1px; }
    .row-fill {
      opacity: 0;
      font-size: 11px; font-weight: 500;
      color: #71717a;
    }
    .row:hover .row-fill,
    .row[data-focused="true"] .row-fill {
      opacity: 1;
    }
    @media (prefers-color-scheme: dark) {
      .row-fill { color: #a1a1aa; }
    }
    .badge {
      flex: 0 0 28px;
      width: 28px; height: 28px;
      border-radius: 6px;
      display: inline-flex; align-items: center; justify-content: center;
      font-weight: 600; font-size: 12px;
      background: #f4f4f5;
      color: #52525b;
    }
    @media (prefers-color-scheme: dark) {
      .badge { background: #27243f; color: #d4d4d8; }
    }
    .row-text {
      flex: 1; min-width: 0;
      display: flex; flex-direction: column; gap: 1px;
    }
    .row-title {
      font-weight: 500; font-size: 13px;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    .row-sub {
      font-size: 11px;
      color: #71717a;
      white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    }
    @media (prefers-color-scheme: dark) {
      .row-sub { color: #a1a1aa; }
    }

    .foot {
      padding: 6px 12px;
      border-top: 1px solid #e4e4e7;
      font-size: 10px;
      color: #71717a;
      text-align: center;
    }
    @media (prefers-color-scheme: dark) {
      .foot { border-top-color: #2c2745; color: #a1a1aa; }
    }
  `;
}
