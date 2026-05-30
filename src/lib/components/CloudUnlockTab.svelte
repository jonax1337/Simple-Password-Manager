<script lang="ts">
  import { Button, Input, Label, toast } from "$lib/ui";
  import { Cloud, ArrowDownToLine, Loader2, KeyRound } from "@lucide/svelte";
  import {
    cloudLogin,
    cloudListVaults,
    cloudOpenVault,
    cloudPersistSession,
    openDatabase,
    type CloudVaultEntry,
  } from "$lib/tauri";
  import { appDataDir, join } from "@tauri-apps/api/path";
  import { saveLastDatabasePath, addRecentDatabase } from "$lib/storage";

  type Phase = "form" | "picker";

  type Props = {
    onUnlock: () => void | Promise<void>;
  };

  let { onUnlock }: Props = $props();

  let phase = $state<Phase>("form");

  // form state — defaults persist between sessions so users land on their
  // own server with one fewer keystroke.
  let serverUrl = $state(
    (typeof window !== "undefined" && localStorage.getItem("cloudServerUrl")) ||
      "http://localhost:8090",
  );
  let username = $state(
    (typeof window !== "undefined" && localStorage.getItem("cloudEmail")) || "",
  );
  let masterPassword = $state("");
  let rememberMe = $state(true);
  let loading = $state(false);
  let error = $state("");

  // picker state
  let vaults = $state<CloudVaultEntry[]>([]);
  let selectedVaultId = $state<string | null>(null);

  async function handleSignIn() {
    if (!serverUrl || !username || !masterPassword) {
      error = "Server, username and master password are required.";
      return;
    }
    loading = true;
    error = "";
    try {
      await cloudLogin(serverUrl, username, masterPassword);
      localStorage.setItem("cloudServerUrl", serverUrl);
      localStorage.setItem("cloudEmail", username);
      masterPassword = "";
      if (rememberMe) {
        try {
          await cloudPersistSession();
        } catch {
          // Persistence is best-effort — non-Windows or denied-by-user
          // shouldn't block the sign-in itself.
        }
      }
      vaults = await cloudListVaults();
      if (vaults.length === 1) {
        selectedVaultId = vaults[0].id;
      }
      phase = "picker";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function vaultTargetPath(vaultId: string, vaultName: string): Promise<string> {
    const base = await appDataDir();
    // Sanitize vault name for filename usage; keep id as the unique key.
    const safeName = (vaultName || vaultId).replace(/[^a-zA-Z0-9_-]+/g, "-").slice(0, 40);
    return join(base, "cloud-vaults", `${safeName}-${vaultId.slice(0, 8)}.kdbx`);
  }

  async function handleOpenSelected() {
    if (!selectedVaultId) return;
    const vault = vaults.find((v) => v.id === selectedVaultId);
    if (!vault) return;
    loading = true;
    error = "";
    try {
      const target = await vaultTargetPath(vault.id, vault.name);
      await cloudOpenVault(vault.id, target);

      // Now open the freshly-written kdbx as if the user had picked a local
      // file. The vault is sealed twice (KeePass + AES-GCM) — we just
      // undid the outer layer; the inner one still needs the kdbx master
      // password the user originally chose.
      const kdbxPassword = window.prompt(
        `Master password for "${vault.name}" (the KDBX password, not the cloud password):`,
      );
      if (!kdbxPassword) {
        toast.error("Cancelled", "Vault downloaded but not opened");
        return;
      }
      await openDatabase(target, kdbxPassword, null);
      saveLastDatabasePath(target);
      addRecentDatabase(target);
      toast.success("Unlocked", `${vault.name} opened from cloud`);
      await onUnlock();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function backToForm() {
    phase = "form";
    vaults = [];
    selectedVaultId = null;
  }
</script>

<div class="space-y-5">
  {#if phase === "form"}
    <div class="space-y-2">
      <h2 class="text-xl font-semibold tracking-tight flex items-center gap-2">
        <Cloud class="size-5 text-primary" />
        Sign in to cloud
      </h2>
      <p class="text-sm text-muted-foreground">
        Connect to your self-hosted vault server. We never see your master password.
      </p>
    </div>

    <div class="space-y-4">
      <div class="space-y-1.5">
        <Label for="cloud-url">Server URL</Label>
        <Input id="cloud-url" bind:value={serverUrl} placeholder="https://vault.example.com" />
      </div>
      <div class="space-y-1.5">
        <Label for="cloud-username">Username</Label>
        <Input id="cloud-username" bind:value={username} placeholder="you@example.com" />
      </div>
      <div class="space-y-1.5">
        <Label for="cloud-pw">Master password</Label>
        <Input
          id="cloud-pw"
          type="password"
          bind:value={masterPassword}
          placeholder="Enter master password"
          onkeydown={(e) => e.key === "Enter" && handleSignIn()}
        />
      </div>

      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" bind:checked={rememberMe} class="rounded" />
        <span>Remember me on this device</span>
      </label>

      {#if error}
        <p class="text-xs text-destructive break-all">{error}</p>
      {/if}

      <Button
        class="w-full"
        onclick={handleSignIn}
        disabled={loading || !serverUrl || !username || !masterPassword}
      >
        {#if loading}
          <Loader2 class="size-4 animate-spin" />
          Signing in…
        {:else}
          Sign in
        {/if}
      </Button>
    </div>
  {:else}
    <div class="space-y-2">
      <h2 class="text-xl font-semibold tracking-tight flex items-center gap-2">
        <KeyRound class="size-5 text-primary" />
        Pick a vault
      </h2>
      <p class="text-sm text-muted-foreground">
        Signed in as <span class="font-medium">{username}</span>.
      </p>
    </div>

    <div class="space-y-2 max-h-[40vh] overflow-y-auto border rounded-md">
      {#each vaults as v (v.id)}
        {@const selected = selectedVaultId === v.id}
        <button
          type="button"
          class="w-full text-left p-3 border-b last:border-b-0 hover:bg-accent/50 transition-colors {selected ? 'bg-accent/70' : ''}"
          onclick={() => (selectedVaultId = v.id)}
        >
          <div class="flex items-center gap-2">
            <span class="font-medium text-sm flex-1 truncate">{v.name}</span>
            <span class="text-2xs uppercase tracking-wider text-muted-foreground">{v.role}</span>
          </div>
          <div class="text-2xs text-muted-foreground">
            Updated {new Date(v.updated_at * 1000).toLocaleString()}
          </div>
        </button>
      {:else}
        <p class="p-4 text-sm text-muted-foreground">No vaults yet on this account.</p>
      {/each}
    </div>

    {#if error}
      <p class="text-xs text-destructive break-all">{error}</p>
    {/if}

    <div class="flex gap-2">
      <Button variant="outline" onclick={backToForm} disabled={loading}>Back</Button>
      <div class="flex-1"></div>
      <Button onclick={handleOpenSelected} disabled={loading || !selectedVaultId}>
        {#if loading}
          <Loader2 class="size-4 animate-spin" />
          Opening…
        {:else}
          <ArrowDownToLine class="size-4" />
          Open vault
        {/if}
      </Button>
    </div>
  {/if}
</div>
