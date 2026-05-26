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

export interface CreateEntryResult {
  uuid: string;
  title: string;
  group_uuid: string;
}

export interface TotpResult {
  code: string;
  period: number;
  remaining_seconds: number;
  algorithm: string;
}

export interface CaptureMessage {
  kind: "capture";
  domain: string;
  url: string;
  username: string;
  password: string;
  intent: "login" | "signup";
}

/** Background-script message envelope. */
export type BgMessage =
  | { kind: "get-bridge"; force?: boolean }
  | {
      kind: "fetch";
      method: "GET" | "POST" | "PUT";
      path: string;
      body?: unknown;
    }
  | { kind: "get-pending-capture" }
  | { kind: "consume-pending-capture" };

export interface PendingCaptureData {
  domain: string;
  url: string;
  username: string;
  password: string;
  capturedAt: number;
  intent: "login" | "signup";
}

export interface BgResponse<T> {
  ok: boolean;
  status?: number;
  error?: string;
  data?: T;
}
