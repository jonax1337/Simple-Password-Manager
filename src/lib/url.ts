import { open as openShell } from "@tauri-apps/plugin-shell";
import { toast } from "$lib/ui";

// Open `value` in the OS default browser, prefixing `https://` if missing.
// Centralises the open/error-toast pattern used by EntryEditor, EntryList,
// and the URL field row.
export async function openUrl(value: string): Promise<void> {
  if (!value) return;
  try {
    const full = value.match(/^https?:\/\//) ? value : `https://${value}`;
    await openShell(full);
  } catch {
    toast.error("Failed to open URL");
  }
}
