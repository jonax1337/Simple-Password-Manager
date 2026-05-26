// Floating "Save this login?" banner injected directly into the host page.
//
// Lives in a Shadow DOM so the host page's CSS can't deface us and vice
// versa. Uses inline styles + Tailwind-flavoured class names defined inside
// the shadow root.

import { blockDomain } from "./blocklist";

export interface InlineBannerInput {
  domain: string;
  url: string;
  username: string;
  password: string;
  intent: "login" | "signup";
}

const HOST_ID = "spm-inline-save-banner";

export function showInlineBanner(input: InlineBannerInput) {
  // Remove any previously-shown banner so we don't stack them.
  document.getElementById(HOST_ID)?.remove();

  const host = document.createElement("div");
  host.id = HOST_ID;
  Object.assign(host.style, {
    position: "fixed",
    top: "16px",
    right: "16px",
    zIndex: "2147483647", // top of the z stack
    width: "320px",
    fontFamily:
      '-apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue", Arial, sans-serif',
  });

  const shadow = host.attachShadow({ mode: "open" });
  shadow.innerHTML = templateHtml(input);

  document.body.appendChild(host);

  wireUp(shadow, host, input);
}

function templateHtml(input: InlineBannerInput): string {
  const title = input.intent === "signup" ? "Save this new account?" : "Save this login?";
  const defaultTitle = input.domain;

  return `
    <style>${css()}</style>
    <div class="card" role="dialog" aria-labelledby="spm-title">
      <header class="header">
        <div class="brand">
          <div class="brand-icon">${KEY_SVG}</div>
          <span class="brand-name">Simple Password Manager</span>
        </div>
        <button class="icon-btn" data-action="dismiss" aria-label="Close">${X_SVG}</button>
      </header>

      <div class="body">
        <h2 id="spm-title" class="title">${escapeHtml(title)}</h2>
        <p class="subtitle">
          <strong>${escapeHtml(input.domain)}</strong>
          ${input.username ? ` &middot; ${escapeHtml(input.username)}` : ""}
        </p>
        <label class="label" for="spm-title-input">Title</label>
        <input
          id="spm-title-input"
          type="text"
          class="input"
          value="${escapeAttr(defaultTitle)}"
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <footer class="footer">
        <button class="btn btn-primary" data-action="save">Save to vault</button>
        <button class="btn btn-outline" data-action="dismiss">Not now</button>
      </footer>

      <button class="ghost-link" data-action="never">Never on this site</button>

      <div class="status" data-status hidden></div>
    </div>
  `;
}

function wireUp(
  shadow: ShadowRoot,
  host: HTMLElement,
  input: InlineBannerInput,
) {
  const titleInput = shadow.querySelector<HTMLInputElement>("#spm-title-input")!;
  const statusEl = shadow.querySelector<HTMLDivElement>("[data-status]")!;
  let busy = false;

  function setStatus(msg: string, tone: "info" | "error" | "success" = "info") {
    statusEl.hidden = false;
    statusEl.textContent = msg;
    statusEl.dataset.tone = tone;
  }

  function dismiss() {
    host.remove();
    // Explicit user action: don't pester them again via the popup.
    void chrome.storage.local.remove("pending_capture").catch(() => {});
  }

  async function save() {
    if (busy) return;
    busy = true;
    setStatus("Saving…");

    const title = titleInput.value.trim() || input.domain;
    const resp = await chrome.runtime.sendMessage({
      kind: "fetch",
      method: "POST",
      path: "/v1/entries",
      body: {
        title,
        username: input.username,
        password: input.password,
        url: input.url,
      },
    });
    busy = false;

    if (resp?.ok) {
      // Clear the pending-capture stash so the popup doesn't double-prompt.
      try {
        await chrome.storage.local.remove("pending_capture");
      } catch {
        /* may not exist; that's fine */
      }
      setStatus("Saved.", "success");
      setTimeout(dismiss, 1200);
    } else {
      setStatus(resp?.error ?? "Could not save", "error");
    }
  }

  async function never() {
    await blockDomain(input.domain);
    try {
      await chrome.storage.local.remove("pending_capture");
    } catch {
      /* */
    }
    dismiss();
  }

  shadow.addEventListener("click", (e) => {
    const t = e.target as HTMLElement | null;
    const action = t?.closest<HTMLElement>("[data-action]")?.dataset.action;
    if (!action) return;
    if (action === "save") void save();
    else if (action === "dismiss") dismiss();
    else if (action === "never") void never();
  });

  // Keyboard: Enter saves, Esc dismisses.
  shadow.addEventListener("keydown", (e) => {
    const ke = e as KeyboardEvent;
    if (ke.key === "Enter") {
      ke.preventDefault();
      void save();
    } else if (ke.key === "Escape") {
      ke.preventDefault();
      dismiss();
    }
  });

  // Focus the title field so the user can immediately tweak it.
  titleInput.focus();
  titleInput.select();
}

// Tiny utilities ------------------------------------------------------------

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

const KEY_SVG = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none"
     stroke="currentColor" stroke-width="2"
     stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d="m21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3"/>
</svg>`;

const X_SVG = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none"
     stroke="currentColor" stroke-width="2"
     stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
</svg>`;

function css(): string {
  // The colour values mirror the desktop app and popup. We don't pull the
  // popup's compiled Tailwind sheet here on purpose: it'd 30x the content
  // script size and we only need a small subset of styles.
  return `
    :host { all: initial; }
    .card {
      background: #ffffff;
      color: #1c1a2e;
      border: 1px solid #d5cee8;
      border-radius: 10px;
      box-shadow: 0 12px 32px -8px rgba(0, 0, 0, 0.25);
      overflow: hidden;
      font-size: 13px;
      animation: spm-in 180ms ease-out both;
    }
    @media (prefers-color-scheme: dark) {
      .card {
        background: #16142a;
        color: #ebe9f4;
        border-color: #2c2745;
      }
    }
    @keyframes spm-in {
      from { transform: translateY(-6px); opacity: 0; }
      to   { transform: translateY(0);    opacity: 1; }
    }
    .header {
      display: flex; align-items: center; justify-content: space-between;
      padding: 8px 10px;
      border-bottom: 1px solid rgba(127, 127, 127, 0.15);
      background: rgba(99, 102, 241, 0.06);
    }
    .brand { display: flex; align-items: center; gap: 6px; }
    .brand-icon {
      display: inline-flex; align-items: center; justify-content: center;
      width: 22px; height: 22px;
      background: rgba(99, 102, 241, 0.15);
      color: #6366f1;
      border-radius: 6px;
    }
    .brand-name { font-weight: 600; font-size: 12px; }

    .body { padding: 12px; }
    .title {
      margin: 0 0 4px 0;
      font-size: 14px;
      font-weight: 600;
    }
    .subtitle {
      margin: 0 0 10px 0;
      font-size: 12px;
      color: #6b6386;
    }
    @media (prefers-color-scheme: dark) {
      .subtitle { color: #a09bbd; }
    }
    .label {
      display: block; font-size: 11px; font-weight: 500;
      color: #6b6386; margin-bottom: 4px;
    }
    @media (prefers-color-scheme: dark) {
      .label { color: #a09bbd; }
    }
    .input {
      width: 100%; box-sizing: border-box;
      height: 30px; padding: 0 8px;
      font-size: 12px; font-family: inherit;
      color: inherit; background: transparent;
      border: 1px solid #d5cee8; border-radius: 6px;
      outline: none;
    }
    @media (prefers-color-scheme: dark) {
      .input { border-color: #2c2745; }
    }
    .input:focus { border-color: #6366f1; box-shadow: 0 0 0 2px rgba(99,102,241,0.2); }

    .footer {
      display: flex; gap: 8px; padding: 0 12px 10px 12px;
    }
    .btn {
      flex: 1;
      height: 32px; border: none; border-radius: 6px;
      font-family: inherit; font-size: 12px; font-weight: 500;
      cursor: pointer; padding: 0 12px;
      transition: background 120ms ease;
    }
    .btn-primary {
      background: #6366f1; color: #ffffff;
    }
    .btn-primary:hover { background: #4f46e5; }
    .btn-outline {
      background: transparent; color: inherit;
      border: 1px solid #d5cee8;
    }
    @media (prefers-color-scheme: dark) {
      .btn-outline { border-color: #2c2745; }
    }
    .btn-outline:hover {
      background: rgba(99, 102, 241, 0.08);
    }
    .ghost-link {
      display: block; width: 100%;
      padding: 6px 12px 10px 12px;
      background: transparent; border: none;
      font-family: inherit; font-size: 11px;
      color: #b91c1c;
      cursor: pointer; text-align: center;
    }
    @media (prefers-color-scheme: dark) {
      .ghost-link { color: #fb7185; }
    }
    .ghost-link:hover { text-decoration: underline; }

    .icon-btn {
      background: transparent; border: none; color: inherit;
      padding: 4px; border-radius: 4px; cursor: pointer; line-height: 0;
    }
    .icon-btn:hover { background: rgba(127, 127, 127, 0.15); }

    .status {
      margin: 0 12px 10px 12px;
      padding: 6px 8px;
      font-size: 11px;
      border-radius: 6px;
      background: rgba(99, 102, 241, 0.08);
      color: #4338ca;
    }
    .status[data-tone="error"] {
      background: rgba(244, 63, 94, 0.12);
      color: #b91c1c;
    }
    .status[data-tone="success"] {
      background: rgba(34, 197, 94, 0.12);
      color: #166534;
    }
    @media (prefers-color-scheme: dark) {
      .status { background: rgba(99, 102, 241, 0.12); color: #c7d2fe; }
      .status[data-tone="error"] { color: #fda4af; }
      .status[data-tone="success"] { color: #86efac; }
    }
  `;
}
