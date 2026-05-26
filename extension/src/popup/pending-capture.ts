// Helpers for the "captured login waiting to be saved" workflow.
//
// The content script captures form submissions and posts them to the
// background service worker, which stashes them in `chrome.storage.session`.
// The popup reads from the same storage and shows a Save banner when
// something is pending.

const STORAGE_KEY = "pending_capture";

export interface PendingCapture {
  domain: string;
  url: string;
  username: string;
  password: string;
  capturedAt: number;
}

export async function readPendingCapture(): Promise<PendingCapture | null> {
  if (typeof chrome === "undefined" || !chrome.storage?.session) return null;
  const result = await chrome.storage.session.get(STORAGE_KEY);
  const raw = result?.[STORAGE_KEY];
  if (!raw || typeof raw !== "object") return null;
  // Defensive cast — anything sent into storage by the BG should look like this.
  return raw as PendingCapture;
}

export async function consumePendingCapture(): Promise<void> {
  if (typeof chrome === "undefined" || !chrome.storage?.session) return;
  await chrome.storage.session.remove(STORAGE_KEY);
}

/** Subscribe to changes; calls back with the current value on mount too. */
export function observePendingCapture(
  cb: (value: PendingCapture | null) => void,
): () => void {
  void readPendingCapture().then(cb);

  const listener = (
    changes: Record<string, chrome.storage.StorageChange>,
    areaName: string,
  ) => {
    if (areaName !== "session") return;
    if (!(STORAGE_KEY in changes)) return;
    const newValue = changes[STORAGE_KEY]?.newValue ?? null;
    cb(newValue ?? null);
  };
  chrome.storage.onChanged.addListener(listener);
  return () => chrome.storage.onChanged.removeListener(listener);
}
