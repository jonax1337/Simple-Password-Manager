import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "uptodate" }
  | { kind: "available"; version: string }
  | { kind: "installing" }
  | { kind: "error"; message: string };

export type UpdateResult =
  | { available: false }
  | {
      available: true;
      version: string;
      install: () => Promise<void>;
    };

export async function checkForUpdate(): Promise<UpdateResult> {
  const u = await check();
  if (!u) return { available: false };
  return {
    available: true,
    version: u.version,
    install: async () => {
      await u.downloadAndInstall();
      await relaunch();
    },
  };
}

// Stateful updater store consumed by AboutDialog + SettingsDialog. Wraps the
// raw plugin in a state machine so the UI doesn't have to reinvent the same
// idle/checking/available/installing flow in two places.
export function createUpdater() {
  let status = $state<UpdateStatus>({ kind: "idle" });

  async function checkNow() {
    status = { kind: "checking" };
    try {
      const r = await checkForUpdate();
      status = r.available
        ? { kind: "available", version: r.version }
        : { kind: "uptodate" };
    } catch (e) {
      status = {
        kind: "error",
        message: e instanceof Error ? e.message : "Update check failed",
      };
    }
  }

  async function installNow() {
    status = { kind: "installing" };
    try {
      const r = await checkForUpdate();
      if (r.available) await r.install();
    } catch (e) {
      status = {
        kind: "error",
        message: e instanceof Error ? e.message : "Install failed",
      };
    }
  }

  return {
    get status() {
      return status;
    },
    get busy() {
      return status.kind === "checking" || status.kind === "installing";
    },
    checkNow,
    installNow,
  };
}
