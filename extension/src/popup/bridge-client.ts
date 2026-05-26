// Thin wrapper around chrome.runtime.sendMessage so the popup never talks
// to the loopback bridge directly — the service worker owns the token.

import type {
  BgMessage,
  BgResponse,
  EntrySummary,
  PasswordResult,
  StatusResult,
} from "../common/bridge";

function send<T>(msg: BgMessage): Promise<BgResponse<T>> {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(msg, (resp: BgResponse<T> | undefined) => {
      const err = chrome.runtime.lastError;
      if (err) {
        resolve({ ok: false, error: err.message });
        return;
      }
      resolve(resp ?? { ok: false, error: "no response" });
    });
  });
}

export async function fetchStatus(): Promise<BgResponse<StatusResult>> {
  return send<StatusResult>({ kind: "fetch", method: "GET", path: "/v1/status" });
}

export async function fetchEntriesForDomain(
  domain: string,
): Promise<BgResponse<EntrySummary[]>> {
  const q = encodeURIComponent(domain);
  return send<EntrySummary[]>({
    kind: "fetch",
    method: "GET",
    path: `/v1/entries?domain=${q}`,
  });
}

export async function fetchAllEntries(): Promise<BgResponse<EntrySummary[]>> {
  return send<EntrySummary[]>({ kind: "fetch", method: "GET", path: "/v1/entries" });
}

export async function fetchPassword(
  uuid: string,
): Promise<BgResponse<PasswordResult>> {
  return send<PasswordResult>({
    kind: "fetch",
    method: "GET",
    path: `/v1/entries/${encodeURIComponent(uuid)}/password`,
  });
}

export interface GenerateOpts {
  length?: number;
  uppercase?: boolean;
  lowercase?: boolean;
  numbers?: boolean;
  symbols?: boolean;
}

export async function generatePassword(
  opts: GenerateOpts = {},
): Promise<BgResponse<{ password: string }>> {
  return send<{ password: string }>({
    kind: "fetch",
    method: "POST",
    path: "/v1/password/generate",
    body: opts,
  });
}
