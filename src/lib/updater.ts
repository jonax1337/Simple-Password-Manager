import { check } from "@tauri-apps/plugin-updater";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

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
    },
  };
}
