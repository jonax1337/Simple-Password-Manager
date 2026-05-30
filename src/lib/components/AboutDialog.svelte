<script lang="ts">
  import { Dialog, Button, UpdateCard } from "$lib/ui";
  import { Heart, ExternalLink } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { open as openShell } from "@tauri-apps/plugin-shell";

  type Props = { open?: boolean };
  let { open = $bindable(false) }: Props = $props();

  let version = $state("…");

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => (version = "unknown"));
  });
</script>

<Dialog bind:open size="sm">
  <div class="flex flex-col items-center text-center space-y-3">
    <img src="/app-icon.png" alt="App icon" class="size-16 drop-shadow-md" />
    <div>
      <h2 class="text-lg font-semibold tracking-tight">Simple Password Manager</h2>
      <p class="text-xs text-muted-foreground mt-0.5">Version {version}</p>
    </div>
  </div>

  <UpdateCard />

  <Button
    variant="outline"
    class="w-full justify-center"
    onclick={() => openShell("https://github.com/jonax1337/Simple-Password-Manager")}
  >
    <ExternalLink class="size-3.5" />
    View on GitHub
  </Button>

  <div class="pt-4 border-t border-border-subtle text-center text-2xs text-muted-foreground space-y-1">
    <p class="flex items-center justify-center gap-1">
      Made with <Heart class="h-3 w-3 fill-current text-destructive" /> by Jonas Laux
    </p>
    <p>Open Source · MIT License</p>
  </div>
</Dialog>
