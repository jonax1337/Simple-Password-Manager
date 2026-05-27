<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { getLastDatabasePath, clearLastDatabasePath } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";
  import UnlockScreen from "./UnlockScreen.svelte";
  import QuickUnlockScreen from "./QuickUnlockScreen.svelte";
  import MainApp from "./MainApp.svelte";
  import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
  import { toast } from "$lib/ui";

  let lastDatabasePath = $state<string | null>(null);
  let filePathFromAssociation = $state<string | null>(null);

  onMount(() => {
    void init();
    // silent update check
    const t = setTimeout(async () => {
      try {
        const u = await checkForUpdate();
        if (u) {
          toast.action(
            `Update v${u.version} available`,
            { label: "Install", onClick: () => u.downloadAndInstall() },
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
    await win.setTitle(`Simple Password Manager — ${path.split(/[\\/]/).pop()}`);
  }

  function handleMainClose(manual = false) {
    appState.setPhase("checking");
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
