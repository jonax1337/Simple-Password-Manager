<script lang="ts">
  import { onMount } from "svelte";
  import { Button, Input, Label, toast } from "$lib/ui";
  import { KeyRound, Fingerprint } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openDatabase, helloAvailable, helloIsEnrolled, helloRetrieve } from "$lib/tauri";
  import { addRecentDatabase, getYubikeyHint } from "$lib/storage";
  import KdfWarningDialog from "$lib/components/KdfWarningDialog.svelte";

  type Props = {
    lastDatabasePath: string;
    onUnlock: () => void | Promise<void>;
    onCancel: () => void;
  };

  let { lastDatabasePath, onUnlock, onCancel }: Props = $props();

  let password = $state("");
  let loading = $state(false);
  let showKdfWarning = $state(false);
  let kdfType = $state("");
  let helloShown = $state(false);
  let autoTriggered = false;

  const yubikeyHint = $derived(getYubikeyHint(lastDatabasePath));
  const dbFile = $derived(lastDatabasePath.split(/[\\/]/).pop() ?? "Vault");
  const dbName = $derived(dbFile.replace(/\.kdbx$/i, ""));

  onMount(() => {
    void (async () => {
      try {
        const [avail, enrolled] = await Promise.all([
          helloAvailable(),
          helloIsEnrolled(lastDatabasePath),
        ]);
        helloShown = avail && enrolled;
      } catch {
        helloShown = false;
      }
    })();
  });

  $effect(() => {
    if (!helloShown || autoTriggered) return;
    const trigger = () => {
      if (autoTriggered) return;
      autoTriggered = true;
      void handleHelloUnlock();
    };
    if (document.hasFocus()) {
      trigger();
      return;
    }
    window.addEventListener("focus", trigger, { once: true });
    return () => window.removeEventListener("focus", trigger);
  });

  async function handleHelloUnlock() {
    loading = true;
    try {
      const pw = await helloRetrieve(lastDatabasePath);
      await openDatabase(lastDatabasePath, pw, yubikeyHint);
      addRecentDatabase(lastDatabasePath);
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
    if (!password) {
      toast.error("Missing Password", "Please enter your master password");
      return;
    }
    loading = true;
    try {
      await openDatabase(lastDatabasePath, password, yubikeyHint);
      addRecentDatabase(lastDatabasePath);

      const dismissedDbs = JSON.parse(localStorage.getItem("kdf_warning_dismissed_dbs") || "[]");
      if (!dismissedDbs.includes(lastDatabasePath)) {
        try {
          const kdfInfo = await invoke<{ kdf_type: string; is_weak: boolean }>("get_kdf_info");
          if (kdfInfo.is_weak) {
            kdfType = kdfInfo.kdf_type;
            showKdfWarning = true;
            loading = false;
            return;
          }
        } catch (e) {
          console.error("KDF check failed", e);
        }
      }

      toast.success("Unlocked", "Database unlocked successfully");
      await onUnlock();
    } catch (error) {
      toast.error("Failed to Unlock", String(error) || "Invalid password");
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
</script>

<KdfWarningDialog
  bind:open={showKdfWarning}
  onSkip={() => {
    showKdfWarning = false;
    void onUnlock();
  }}
  onUpgrade={handleKdfUpgrade}
  {kdfType}
  databasePath={lastDatabasePath}
/>

<div class="flex h-full w-full items-center justify-center overflow-hidden bg-background">
  <div class="w-full max-w-[380px] mx-6">
    <div class="rounded-2xl border bg-card px-7 pt-7 pb-6">
      <div class="flex flex-col items-center text-center space-y-3">
        <img src="/app-icon.png" alt="" aria-hidden="true" class="size-14 object-contain" />
        <div>
          <h2 class="text-[18px] font-semibold tracking-tight">{dbName}</h2>
          <p class="text-[11px] text-muted-foreground truncate max-w-[300px]" title={lastDatabasePath}>
            {lastDatabasePath}
          </p>
        </div>
      </div>

      <div class="mt-6 space-y-4">
        <div class="space-y-1.5">
          <Label for="quick-password">Master password</Label>
          <!-- svelte-ignore a11y_autofocus -->
          <Input
            id="quick-password"
            type="password"
            bind:value={password}
            onkeydown={(e) => e.key === "Enter" && handleUnlock()}
            placeholder="Enter your master password"
            autofocus
          />
        </div>

        {#if yubikeyHint}
          <div class="flex items-start gap-2.5 rounded-lg border border-primary/30 bg-primary/5 p-3">
            <KeyRound class="mt-0.5 size-4 text-primary shrink-0" />
            <div class="text-[12px] leading-relaxed">
              <p class="font-medium">Yubikey required</p>
              <p class="text-muted-foreground">
                Plug in serial #{yubikeyHint.serial_number} and touch when prompted.
              </p>
            </div>
          </div>
        {/if}

        <div class="space-y-2">
          {#if helloShown}
            <Button onclick={handleHelloUnlock} disabled={loading} class="w-full">
              <Fingerprint class="size-4" />
              Unlock with Windows Hello
            </Button>
            <Button onclick={handleUnlock} variant="outline" disabled={loading || !password} class="w-full">
              {#if loading}
                {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
              {:else}
                Use master password
              {/if}
            </Button>
          {:else}
            <Button onclick={handleUnlock} disabled={loading || !password} class="w-full">
              {#if loading}
                {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
              {:else}
                Unlock
              {/if}
            </Button>
          {/if}
        </div>
      </div>
    </div>

    <div class="text-center mt-4">
      <button
        type="button"
        onclick={onCancel}
        disabled={loading}
        class="text-[11.5px] text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
      >
        Open a different database
      </button>
    </div>
  </div>
</div>
