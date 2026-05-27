<script lang="ts">
  import { Button, Input, Label, toast } from "$lib/ui";
  import { FolderOpen, Plus, KeyRound, Fingerprint, Lock } from "@lucide/svelte";
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
  const fileLabel = $derived(filePath ? filePath.split(/[\\/]/).pop() : "");

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
          const kdfInfo = await invoke<{ kdf_type: string; is_weak: boolean }>("get_kdf_info");
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

<!-- Two-panel unlock — branding on the left, form on the right (collapses on narrow). -->
<div class="relative flex h-full w-full overflow-hidden bg-background">
  <!-- Backdrop atmosphere -->
  <div
    class="pointer-events-none absolute inset-0 -z-10"
    aria-hidden="true"
    style="
      background:
        radial-gradient(900px 600px at 10% 10%, color-mix(in oklch, var(--color-primary) 14%, transparent), transparent 60%),
        radial-gradient(800px 500px at 90% 100%, color-mix(in oklch, var(--color-primary) 8%, transparent), transparent 60%);
    "
  ></div>

  <!-- Brand panel -->
  <aside class="hidden lg:flex w-[44%] max-w-[520px] flex-col justify-between p-12 border-r border-border/60 bg-sidebar/60 backdrop-blur-sm">
    <div class="flex items-center gap-2.5">
      <div class="grid place-items-center size-7 rounded-md bg-gradient-to-br from-primary to-primary/70 text-primary-foreground font-bold text-xs shadow-sm">P</div>
      <span class="text-[13px] font-semibold tracking-tight">Simple Password Manager</span>
    </div>

    <div class="space-y-6">
      <div class="grid place-items-center size-20 rounded-3xl bg-primary/10 text-primary ring-soft">
        <Lock class="size-10" />
      </div>
      <div class="space-y-3">
        <h1 class="text-[34px] font-semibold tracking-tight leading-[1.1]">
          Your vault,<br/>locked &amp; loaded.
        </h1>
        <p class="text-[13.5px] text-muted-foreground leading-relaxed max-w-[360px]">
          Unlock your KeePass database to access every saved item, generate strong passwords,
          and keep your accounts in order — all stored locally on your machine.
        </p>
      </div>
    </div>

    <div class="text-[11px] text-muted-foreground/80 space-y-1">
      <p class="flex items-center gap-1.5">
        <span class="size-1.5 rounded-full bg-success/70"></span>
        End-to-end encrypted — your master password never leaves this device.
      </p>
    </div>
  </aside>

  <!-- Form panel -->
  <main class="flex-1 grid place-items-center px-6 py-10 overflow-y-auto">
    <div class="w-full max-w-[400px] space-y-7">
      <div class="lg:hidden flex items-center gap-2.5 justify-center">
        <div class="grid place-items-center size-7 rounded-md bg-gradient-to-br from-primary to-primary/70 text-primary-foreground font-bold text-xs shadow-sm">P</div>
        <span class="text-[13px] font-semibold tracking-tight">Simple Password Manager</span>
      </div>

      <div class="space-y-2">
        <h2 class="text-[22px] font-semibold tracking-tight">Welcome back</h2>
        <p class="text-[13px] text-muted-foreground">
          Open your <code class="font-mono text-[12px]">.kdbx</code> file and enter your master password.
        </p>
      </div>

      <div class="space-y-4">
        <div class="space-y-1.5">
          <Label for="database">Database file</Label>
          <button
            type="button"
            onclick={handleSelectFile}
            class="group/file w-full flex items-center gap-2.5 h-10 rounded-md border border-input bg-card hover:bg-accent/30 px-3 transition-colors text-left"
          >
            <FolderOpen class="size-4 text-muted-foreground shrink-0" />
            <span class="flex-1 min-w-0 text-[13px] truncate {fileLabel ? '' : 'text-muted-foreground'}">
              {fileLabel || "Choose a .kdbx file…"}
            </span>
            <span class="text-[11px] text-muted-foreground group-hover/file:text-foreground transition-colors">Browse</span>
          </button>
          {#if filePath}
            <p class="text-[10.5px] text-muted-foreground/80 truncate font-mono px-1" title={filePath}>{filePath}</p>
          {/if}
        </div>

        <div class="space-y-1.5">
          <Label for="password">Master password</Label>
          <Input
            id="password"
            type="password"
            bind:value={password}
            onkeydown={(e) => e.key === "Enter" && handleUnlock()}
            placeholder="Enter your master password"
            class="h-10"
          />
        </div>

        {#if yubikeyHint}
          <div class="flex items-start gap-2.5 rounded-lg border border-primary/30 bg-primary/5 p-3">
            <KeyRound class="mt-0.5 size-4 text-primary shrink-0" />
            <div class="text-[12px] leading-relaxed">
              <p class="font-medium">Yubikey required</p>
              <p class="text-muted-foreground">
                Plug in serial #{yubikeyHint.serial_number} and touch it when prompted.
              </p>
            </div>
          </div>
        {/if}

        <div class="space-y-2">
          {#if helloShown}
            <Button onclick={handleHelloUnlock} disabled={loading || !filePath} class="w-full h-10">
              <Fingerprint class="size-4" />
              Unlock with Windows Hello
            </Button>
            <Button
              onclick={handleUnlock}
              variant="outline"
              disabled={loading || !filePath || !password}
              class="w-full h-10"
            >
              {#if loading}
                {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
              {:else}
                Use master password
              {/if}
            </Button>
          {:else}
            <Button onclick={handleUnlock} disabled={loading || !filePath || !password} class="w-full h-10">
              {#if loading}
                {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
              {:else}
                Unlock database
              {/if}
            </Button>
          {/if}
        </div>

        <div class="relative my-2">
          <div class="absolute inset-0 flex items-center"><span class="w-full border-t"></span></div>
          <div class="relative flex justify-center">
            <span class="bg-background px-3 text-[10px] uppercase tracking-wider text-muted-foreground">Or</span>
          </div>
        </div>

        <Button variant="outline" onclick={() => (showCreateDialog = true)} class="w-full h-10">
          <Plus class="size-4" />
          Create a new database
        </Button>
      </div>
    </div>
  </main>
</div>
