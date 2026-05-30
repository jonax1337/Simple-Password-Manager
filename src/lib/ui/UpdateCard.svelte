<script lang="ts">
  import { Check, Download } from "@lucide/svelte";
  import { createUpdater } from "$lib/updater.svelte";
  import { cn } from "$lib/utils";
  import Button from "./Button.svelte";

  type Props = {
    // `card` is the bordered tile used in AboutDialog. `inline` is the bare
    // status-line + button used in SettingsDialog.
    variant?: "card" | "inline";
    class?: string;
  };

  let { variant = "card", class: klass }: Props = $props();
  const updater = createUpdater();

  const statusText = $derived.by(() => {
    const s = updater.status;
    switch (s.kind) {
      case "idle":
        return "Check if a newer version is available.";
      case "checking":
        return "Checking for updates…";
      case "uptodate":
        return "You're on the latest version.";
      case "available":
        return null; // rendered inline with version emphasis
      case "installing":
        return "Downloading and installing…";
      case "error":
        return s.message;
    }
  });
</script>

{#if variant === "card"}
  <div class={cn("rounded-lg border p-3", klass)}>
    <div class="flex items-center gap-2.5">
      {#if updater.status.kind === "uptodate"}
        <div class="size-7 shrink-0 grid place-items-center rounded-full bg-success/15">
          <Check class="size-3.5 text-success" />
        </div>
      {:else}
        <div class="size-7 shrink-0 grid place-items-center rounded-full bg-primary/10">
          <Download class="size-3.5 text-primary" />
        </div>
      {/if}
      <div class="min-w-0 flex-1 text-sm">
        {#if updater.status.kind === "available"}
          <span><span class="font-medium">{updater.status.version}</span> is ready to install.</span>
        {:else if updater.status.kind === "error"}
          <span class="text-destructive text-xs">{statusText}</span>
        {:else}
          <span class={updater.status.kind === "uptodate" ? "" : "text-muted-foreground"}>
            {statusText}
          </span>
        {/if}
      </div>
    </div>
    <div class="mt-3">
      {#if updater.status.kind === "available"}
        <Button size="sm" class="w-full" onclick={updater.installNow} disabled={updater.busy}>
          Install &amp; restart
        </Button>
      {:else}
        <Button size="sm" variant="outline" class="w-full" onclick={updater.checkNow} disabled={updater.busy}>
          {updater.status.kind === "checking" ? "Checking…" : "Check for updates"}
        </Button>
      {/if}
    </div>
  </div>
{:else}
  <div class={klass}>
    <p class="text-xs text-muted-foreground">
      {#if updater.status.kind === "available"}
        <span class="font-medium text-foreground">v{updater.status.version}</span> is ready to install.
      {:else if updater.status.kind === "error"}
        <span class="text-destructive">{statusText}</span>
      {:else}
        {statusText}
      {/if}
    </p>
    <div class="pt-2">
      {#if updater.status.kind === "available"}
        <Button size="sm" onclick={updater.installNow} disabled={updater.busy}>Install &amp; restart</Button>
      {:else}
        <Button size="sm" variant="outline" onclick={updater.checkNow} disabled={updater.busy}>
          {updater.status.kind === "checking" ? "Checking…" : "Check for updates"}
        </Button>
      {/if}
    </div>
  </div>
{/if}
