<script lang="ts">
  import { Button } from "$lib/ui";
  import { Heart, Download, Check, ArrowLeft } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import { pop } from "svelte-spa-router";

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

<div class="flex h-full flex-col">
  <div class="shrink-0 flex items-center gap-2 border-b px-4 py-3">
    <Button variant="ghost" size="icon" onclick={() => pop()}>
      <ArrowLeft class="h-4 w-4" />
    </Button>
    <h1 class="text-lg font-semibold">About</h1>
  </div>

  <div class="flex-1 overflow-auto">
    <div class="flex flex-col items-center justify-center p-12 space-y-8">
      <div class="flex flex-col items-center space-y-4">
        <img src="/app-icon.png" alt="App Icon" class="h-24 w-24 drop-shadow-md" />
        <div class="text-center space-y-2">
          <h1 class="text-3xl font-bold tracking-tight">Simple Password Manager</h1>
          <p class="text-sm text-muted-foreground">Version {version}</p>
        </div>
      </div>

      <p class="text-center text-muted-foreground max-w-md leading-relaxed">
        A secure and modern password manager built with the proven KeePass database format. Keep your
        passwords safe with strong encryption.
      </p>

      <div class="w-full max-w-xs rounded-md border p-3">
        <div class="flex items-center gap-2.5">
          {#if status.kind === "uptodate"}
            <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-success/15">
              <Check class="h-3.5 w-3.5 text-success" />
            </div>
          {:else}
            <div class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10">
              <Download class="h-3.5 w-3.5 text-primary" />
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
        <div class="mt-3 flex gap-2">
          {#if status.kind === "available"}
            <Button size="sm" class="flex-1" onclick={install} disabled={busy}>
              Install &amp; restart
            </Button>
          {:else}
            <Button size="sm" variant="outline" class="flex-1" onclick={check} disabled={busy}>
              {status.kind === "checking" ? "Checking…" : "Check for updates"}
            </Button>
          {/if}
        </div>
      </div>

      <div class="flex flex-col gap-3 w-full max-w-xs">
        <Button variant="outline" class="w-full justify-center gap-2" onclick={() => openShell("https://github.com/jonax1337/Simple-Password-Manager")}>
          View on GitHub
        </Button>
      </div>

      <div class="flex flex-wrap justify-center gap-2 mt-4">
        {#each ["Tauri", "Svelte 5", "Vite", "Rust", "TypeScript"] as t (t)}
          <span class="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">{t}</span>
        {/each}
      </div>

      <div class="pt-8 text-center space-y-1">
        <p class="text-xs text-muted-foreground flex items-center justify-center gap-1">
          Made with <Heart class="h-3 w-3 fill-current text-destructive" /> by Jonas Laux
        </p>
        <p class="text-xs text-muted-foreground">Open Source • MIT License</p>
      </div>
    </div>
  </div>
</div>
