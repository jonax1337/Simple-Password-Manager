<script lang="ts">
  import { Dialog as DialogPrimitive } from "bits-ui";
  import { Button } from "$lib/ui";
  import { Heart, Download, Check, X, ExternalLink } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { open as openShell } from "@tauri-apps/plugin-shell";

  type Props = { open?: boolean };
  let { open = $bindable(false) }: Props = $props();

  type Status =
    | { kind: "idle" }
    | { kind: "checking" }
    | { kind: "uptodate" }
    | { kind: "available"; version: string }
    | { kind: "installing" }
    | { kind: "error"; message: string };

  let version = $state("…");
  let status = $state<Status>({ kind: "idle" });

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => (version = "unknown"));
  });

  const busy = $derived(status.kind === "checking" || status.kind === "installing");

  async function check() {
    status = { kind: "checking" };
    try {
      const u = await checkForUpdate();
      if (u) status = { kind: "available", version: u.version };
      else status = { kind: "uptodate" };
    } catch (e) {
      status = { kind: "error", message: e instanceof Error ? e.message : "Update check failed" };
    }
  }

  async function install() {
    status = { kind: "installing" };
    try {
      const u = await checkForUpdate();
      if (u) {
        await u.downloadAndInstall();
        await relaunch();
      }
    } catch (e) {
      status = { kind: "error", message: e instanceof Error ? e.message : "Install failed" };
    }
  }
</script>

<DialogPrimitive.Root bind:open>
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay
      class="fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <DialogPrimitive.Content
      class="fixed top-[50%] left-[50%] z-50 w-full max-w-[440px] translate-x-[-50%] translate-y-[-50%] rounded-xl border bg-background p-7 shadow-2xl outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
    >
      <DialogPrimitive.Title class="sr-only">About</DialogPrimitive.Title>
      <DialogPrimitive.Description class="sr-only">App information</DialogPrimitive.Description>

      <DialogPrimitive.Close
        class="absolute right-3 top-3 size-7 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
      >
        <X class="size-4" />
        <span class="sr-only">Close</span>
      </DialogPrimitive.Close>

      <div class="flex flex-col items-center text-center space-y-3">
        <img src="/app-icon.png" alt="App icon" class="size-16 drop-shadow-md" />
        <div>
          <h2 class="text-lg font-semibold tracking-tight">Simple Password Manager</h2>
          <p class="text-xs text-muted-foreground mt-0.5">Version {version}</p>
        </div>
      </div>

      <div class="mt-5 rounded-lg border p-3">
        <div class="flex items-center gap-2.5">
          {#if status.kind === "uptodate"}
            <div class="size-7 shrink-0 grid place-items-center rounded-full bg-success/15">
              <Check class="size-3.5 text-success" />
            </div>
          {:else}
            <div class="size-7 shrink-0 grid place-items-center rounded-full bg-primary/10">
              <Download class="size-3.5 text-primary" />
            </div>
          {/if}
          <div class="min-w-0 flex-1 text-sm">
            {#if status.kind === "idle"}
              <span class="text-muted-foreground">Check if a newer version is available.</span>
            {:else if status.kind === "checking"}
              <span class="text-muted-foreground">Checking for updates…</span>
            {:else if status.kind === "uptodate"}
              <span>You're on the latest version.</span>
            {:else if status.kind === "available"}
              <span><span class="font-medium">{status.version}</span> is ready to install.</span>
            {:else if status.kind === "installing"}
              <span class="text-muted-foreground">Downloading and installing…</span>
            {:else if status.kind === "error"}
              <span class="text-destructive text-xs">{status.message}</span>
            {/if}
          </div>
        </div>
        <div class="mt-3">
          {#if status.kind === "available"}
            <Button size="sm" class="w-full" onclick={install} disabled={busy}>
              Install &amp; restart
            </Button>
          {:else}
            <Button size="sm" variant="outline" class="w-full" onclick={check} disabled={busy}>
              {status.kind === "checking" ? "Checking…" : "Check for updates"}
            </Button>
          {/if}
        </div>
      </div>

      <div class="mt-4">
        <Button
          variant="outline"
          class="w-full justify-center"
          onclick={() => openShell("https://github.com/jonax1337/Simple-Password-Manager")}
        >
          <ExternalLink class="size-3.5" />
          View on GitHub
        </Button>
      </div>

      <div class="mt-5 pt-4 border-t text-center text-[11px] text-muted-foreground space-y-1">
        <p class="flex items-center justify-center gap-1">
          Made with <Heart class="h-3 w-3 fill-current text-destructive" /> by Jonas Laux
        </p>
        <p>Open Source · MIT License</p>
      </div>
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>
