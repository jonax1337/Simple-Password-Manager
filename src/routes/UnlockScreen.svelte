<script lang="ts">
  import { Button, Input, Label, toast } from "$lib/ui";
  import { FolderOpen, Plus, KeyRound, Fingerprint } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { openDatabase, helloAvailable, helloIsEnrolled, helloRetrieve } from "$lib/tauri";
  import { saveLastDatabasePath, addRecentDatabase, getYubikeyHint } from "$lib/storage";
  import CreateDatabaseDialog from "$lib/components/CreateDatabaseDialog.svelte";
  import KdfWarningDialog from "$lib/components/KdfWarningDialog.svelte";

  type Props = {
    initialFilePath?: string | null;
    onUnlock: () => void | Promise<void>;
  };

  let { initialFilePath = null, onUnlock }: Props = $props();

  let password = $state("");
  // svelte-ignore state_referenced_locally
  let filePath = $state(initialFilePath ?? "");
  let loading = $state(false);
  let showCreateDialog = $state(false);
  let showKdfWarning = $state(false);
  let kdfType = $state("");
  let helloShown = $state(false);

  const yubikeyHint = $derived(filePath ? getYubikeyHint(filePath) : null);

  $effect(() => {
    if (!filePath) {
      helloShown = false;
      return;
    }
    void (async () => {
      try {
        const [avail, enrolled] = await Promise.all([
          helloAvailable(),
          helloIsEnrolled(filePath),
        ]);
        helloShown = avail && enrolled;
      } catch {
        helloShown = false;
      }
    })();
  });

  async function handleSelectFile() {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "KeePass Database", extensions: ["kdbx"] }],
      });
      if (selected) filePath = selected as string;
    } catch (error) {
      toast.error("Error", String(error) || "Failed to select file");
    }
  }

  async function handleHelloUnlock() {
    if (!filePath) return;
    loading = true;
    try {
      const pw = await helloRetrieve(filePath);
      await openDatabase(filePath, pw, yubikeyHint);
      saveLastDatabasePath(filePath);
      addRecentDatabase(filePath);
      toast.success("Unlocked", "Database unlocked with Windows Hello");
      await onUnlock();
    } catch (error) {
      const msg = String(error);
      if (!msg.includes("cancelled")) {
        toast.error("Hello unlock failed", msg || "Could not unlock with Windows Hello");
      }
    } finally {
      loading = false;
    }
  }

  async function handleUnlock() {
    if (!filePath || !password) {
      toast.error("Missing Information", "Please select a database file and enter a password");
      return;
    }
    loading = true;
    try {
      await openDatabase(filePath, password, yubikeyHint);
      saveLastDatabasePath(filePath);
      addRecentDatabase(filePath);

      const dismissedDbs = JSON.parse(localStorage.getItem("kdf_warning_dismissed_dbs") || "[]");
      if (!dismissedDbs.includes(filePath)) {
        try {
          const kdfInfo = await invoke<{
            kdf_type: string;
            is_weak: boolean;
          }>("get_kdf_info");
          if (kdfInfo.is_weak) {
            kdfType = kdfInfo.kdf_type;
            showKdfWarning = true;
            loading = false;
            return;
          }
        } catch (e) {
          console.error("Failed to check KDF info:", e);
        }
      }

      toast.success("Unlocked", "Database unlocked successfully");
      await onUnlock();
    } catch (error) {
      toast.error("Failed to Unlock", String(error) || "Invalid password or corrupted database");
    } finally {
      loading = false;
    }
  }

  async function handleKdfUpgrade() {
    try {
      await invoke("upgrade_kdf_parameters");
      toast.success("Upgraded", "Key transformation settings upgraded");
      showKdfWarning = false;
      await onUnlock();
    } catch (error) {
      toast.error("Upgrade Failed", String(error) || "Failed to upgrade KDF parameters");
    }
  }

  function handleKdfSkip() {
    showKdfWarning = false;
    void onUnlock();
  }
</script>

<CreateDatabaseDialog
  bind:open={showCreateDialog}
  onSuccess={() => onUnlock()}
/>

<KdfWarningDialog
  bind:open={showKdfWarning}
  onSkip={handleKdfSkip}
  onUpgrade={handleKdfUpgrade}
  {kdfType}
  databasePath={filePath}
/>

<div class="flex flex-1 items-center justify-center bg-gradient-to-br from-background to-muted/40">
  <div class="w-full max-w-md space-y-8 rounded-xl border bg-card p-8 shadow-lg">
    <div class="flex flex-col items-center space-y-4">
      <img src="/app-icon.png" alt="Simple Password Manager" width="96" height="96" class="drop-shadow-md" />
      <h1 class="text-2xl font-bold tracking-tight">Simple Password Manager</h1>
      <p class="text-sm text-muted-foreground">Open and unlock your password database</p>
    </div>

    <div class="space-y-4">
      <div class="space-y-2">
        <Label for="database">Database File</Label>
        <div class="flex gap-2">
          <Input id="database" bind:value={filePath} placeholder="Select a .kdbx file" readonly class="flex-1" />
          <Button type="button" variant="outline" size="icon" onclick={handleSelectFile} title="Open existing database">
            <FolderOpen class="h-4 w-4" />
          </Button>
        </div>
      </div>

      <div class="space-y-2">
        <Label for="password">Master Password</Label>
        <Input
          id="password"
          type="password"
          bind:value={password}
          onkeydown={(e) => e.key === "Enter" && handleUnlock()}
          placeholder="Enter your master password"
        />
      </div>

      {#if yubikeyHint}
        <div class="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-sm">
          <KeyRound class="mt-0.5 h-4 w-4 text-primary shrink-0" />
          <div>
            <p class="font-medium">Yubikey required</p>
            <p class="text-xs text-muted-foreground">
              Plug in your Yubikey (serial #{yubikeyHint.serial_number}) — you'll be asked to touch it after submitting.
            </p>
          </div>
        </div>
      {/if}

      {#if helloShown}
        <Button onclick={handleHelloUnlock} disabled={loading || !filePath} class="w-full">
          <Fingerprint class="h-4 w-4" />
          Unlock with Windows Hello
        </Button>
      {/if}

      <Button
        onclick={handleUnlock}
        variant={helloShown ? "outline" : "default"}
        disabled={loading || !filePath || !password}
        class="w-full"
      >
        {#if loading}
          {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
        {:else if helloShown}
          Use master password
        {:else}
          Unlock Database
        {/if}
      </Button>

      <div class="relative">
        <div class="absolute inset-0 flex items-center">
          <span class="w-full border-t"></span>
        </div>
        <div class="relative flex justify-center text-xs uppercase">
          <span class="bg-card px-2 text-muted-foreground">Or</span>
        </div>
      </div>

      <Button variant="outline" onclick={() => (showCreateDialog = true)} class="w-full">
        <Plus class="mr-2 h-4 w-4" />
        Create New Database
      </Button>
    </div>
  </div>
</div>
