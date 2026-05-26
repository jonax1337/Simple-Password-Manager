// Shared helpers + types for talking to the local Bridge HTTP server.
//
// The native-messaging host name MUST match what we register at the OS
// level (see scripts/install-host.* for the per-platform manifests).

export const NATIVE_HOST_NAME = "digital.laux.simple_password_manager";

export interface BridgeInfo {
  port: number;
  token: string;
  pid: number;
}

export interface EntrySummary {
  uuid: string;
  title: string;
  username: string;
  url: string;
}

export interface PasswordResult {
  password: string;
  username: string;
  title: string;
}

export interface StatusResult {
  unlocked: boolean;
  database_name: string | null;
  version: string;
}

/** Background-script message envelope. */
export type BgMessage =
  | { kind: "get-bridge"; force?: boolean }
  | { kind: "fetch"; method: "GET" | "POST"; path: string; body?: unknown };

export interface BgResponse<T> {
  ok: boolean;
  status?: number;
  error?: string;
  data?: T;
}
