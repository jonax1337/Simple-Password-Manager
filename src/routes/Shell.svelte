<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getLastDatabasePath, clearLastDatabasePath } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import { cloudRehydrateSession } from "$lib/tauri";
  import UnlockScreen from "./UnlockScreen.svelte";
  import QuickUnlockScreen from "./QuickUnlockScreen.svelte";
  import MainApp from "./MainApp.svelte";
  import { checkForUpdate } from "$lib/updater.svelte";
  import { toast } from "$lib/ui";

  let lastDatabasePath = $state<string | null>(null);
  let filePathFromAssociation = $state<string | null>(null);

  /** If a Remember-Me cloud session rehydrated at startup, this is the
   * last-opened vault id the picker should highlight + auto-target. */
  let rehydratedVaultId = $state<string | null>(null);

  onMount(() => {
    void init();
    // silent update check
    const t = setTimeout(async () => {
      try {
        const u = await checkForUpdate();
        if (u.available) {
          toast.action(
            `Update v${u.version} available`,
            { label: "Install", onClick: () => u.install() },
            { description: "Click to download and restart." },
          );
        }
      } catch {
        // ignore
      }
    }, 3000);
    return () => clearTimeout(t);
  });

  async function init() {
    // First, see if a Remember-Me cloud session is on disk. If so, the
    // Rust side rehydrates the in-memory CloudSession (master_key + token)
    // automatically; we just remember which vault to highlight and route
    // straight to the unlock screen so the cloud tab can take over.
    try {
      const rehydrated = await cloudRehydrateSession();
      if (rehydrated.linked) {
        rehydratedVaultId = rehydrated.last_vault_id;
        appState.setPhase("unlock");
        return;
      }
    } catch {
      // Persistence backend unavailable (non-Windows) or corrupt entry.
      // Fall through to the local-file flow.
    }

    try {
      const initialFilePath = await invoke<string | null>("get_initial_file_path");
      if (initialFilePath) {
        filePathFromAssociation = initialFilePath;
        appState.setPhase("unlock");
      } else {
        const last = getLastDatabasePath();
        lastDatabasePath = last;
        appState.setPhase(last ? "quick-unlock" : "unlock");
      }
    } catch {
      appState.setPhase("unlock");
    }
  }

  async function handleUnlocked(path: string) {
    appState.setDbPath(path);
    appState.markClean();
    appState.setPhase("main");
    filePathFromAssociation = null;
    try {
      await invoke("clear_initial_file_path");
    } catch {
      // ignore
    }
    const win = getCurrentWindow();
    // Cloud-only vaults have no path — fall back to the vault name the
    // CloudUnlockTab stashed in appState.
    const label =
      path && path.length > 0
        ? path.split(/[\\/]/).pop()
        : appState.cloudVaultName ?? "Vault";
    await win.setTitle(`Simple Password Manager — ${label}`);
  }

  function handleMainClose(manual = false) {
    appState.setPhase("checking");
    appState.setCloudVaultName(null);
    appState.setCloudVaultId(null);
    if (manual) {
      clearLastDatabasePath();
      lastDatabasePath = null;
    }
    // re-init phase decision
    const last = getLastDatabasePath();
    lastDatabasePath = last;
    appState.setPhase(last ? "quick-unlock" : "unlock");
  }
</script>

<div class="h-full w-full flex flex-col">
  {#if appState.phase === "checking"}
    <!-- empty splash -->
  {:else if appState.phase === "unlock"}
    <UnlockScreen
      initialFilePath={filePathFromAssociation}
      {rehydratedVaultId}
      onUnlock={async () => {
        const p = filePathFromAssociation ?? getLastDatabasePath() ?? "";
        await handleUnlocked(p);
      }}
    />
  {:else if appState.phase === "quick-unlock" && lastDatabasePath}
    <QuickUnlockScreen
      {lastDatabasePath}
      onUnlock={() => handleUnlocked(lastDatabasePath!)}
      onCancel={() => appState.setPhase("unlock")}
    />
  {:else if appState.phase === "main"}
    <MainApp onClose={handleMainClose} />
  {/if}
</div>
