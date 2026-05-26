// "Never offer save for this site" list, persisted across sessions in
// chrome.storage.local.

const STORAGE_KEY = "save_blocklist";

export async function isDomainBlocked(domain: string): Promise<boolean> {
  if (typeof chrome === "undefined" || !chrome.storage?.local) return false;
  const result = await chrome.storage.local.get(STORAGE_KEY);
  const raw = result?.[STORAGE_KEY];
  if (!Array.isArray(raw)) return false;
  return raw.includes(domain);
}

export async function blockDomain(domain: string): Promise<void> {
  if (typeof chrome === "undefined" || !chrome.storage?.local) return;
  const result = await chrome.storage.local.get(STORAGE_KEY);
  const raw = result?.[STORAGE_KEY];
  const list: string[] = Array.isArray(raw) ? raw.slice() : [];
  if (!list.includes(domain)) list.push(domain);
  await chrome.storage.local.set({ [STORAGE_KEY]: list });
}

export async function unblockDomain(domain: string): Promise<void> {
  if (typeof chrome === "undefined" || !chrome.storage?.local) return;
  const result = await chrome.storage.local.get(STORAGE_KEY);
  const raw = result?.[STORAGE_KEY];
  if (!Array.isArray(raw)) return;
  const filtered = raw.filter((d) => d !== domain);
  await chrome.storage.local.set({ [STORAGE_KEY]: filtered });
}
