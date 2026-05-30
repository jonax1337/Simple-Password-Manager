import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { toast } from "$lib/ui";

// 1Password's default — long enough to paste into a slow form,
// short enough that an idle session doesn't leak.
const CLEAR_AFTER_MS = 90_000;

// Copy `value` to the clipboard, toast on success, and schedule a clipboard
// wipe so secrets don't sit in the OS clipboard forever. Used everywhere a
// row exposes a "copy" action — keeps the wipe interval consistent across
// password, username, URL, TOTP, and custom-field copy buttons.
export async function copyWithFeedback(
  value: string,
  label: string,
  options: { clearAfter?: boolean } = {},
): Promise<boolean> {
  if (!value) return false;
  try {
    await writeText(value);
    toast.success("Copied", `${label} copied to clipboard`);
    if (options.clearAfter !== false) {
      setTimeout(() => void writeText(""), CLEAR_AFTER_MS);
    }
    return true;
  } catch {
    toast.error("Copy failed");
    return false;
  }
}
