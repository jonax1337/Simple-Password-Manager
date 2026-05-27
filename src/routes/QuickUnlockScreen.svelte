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

<div class="flex flex-1 items-center justify-center bg-gradient-to-br from-background to-muted/40">
  <div class="w-full max-w-sm space-y-6 rounded-xl border bg-card p-6 shadow-lg">
    <div class="flex flex-col items-center space-y-2">
      <img src="/quick-unlock.png" alt="Quick Unlock" width="64" height="64" class="mb-2" />
      <h2 class="text-xl font-semibold tracking-tight">Quick Unlock</h2>
      <p class="text-center text-xs text-muted-foreground break-all px-2">{lastDatabasePath}</p>
    </div>

    <div class="space-y-4">
      <div class="space-y-2">
        <Label for="quick-password">Master Password</Label>
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
        <div class="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-xs">
          <KeyRound class="mt-0.5 h-4 w-4 text-primary shrink-0" />
          <div>
            <p class="font-medium text-sm">Yubikey required</p>
            <p class="text-muted-foreground">
              Plug in serial #{yubikeyHint.serial_number} and touch it when prompted.
            </p>
          </div>
        </div>
      {/if}

      {#if helloShown}
        <Button onclick={handleHelloUnlock} disabled={loading} class="w-full">
          <Fingerprint class="h-4 w-4" />
          Unlock with Windows Hello
        </Button>
      {/if}

      <Button
        onclick={handleUnlock}
        variant={helloShown ? "outline" : "default"}
        disabled={loading || !password}
        class="w-full"
      >
        {#if loading}
          {yubikeyHint ? "Touch your Yubikey…" : "Unlocking…"}
        {:else if helloShown}
          Use master password
        {:else}
          Unlock
        {/if}
      </Button>

      <Button variant="outline" onclick={onCancel} class="w-full" disabled={loading}>
        Open Different Database
      </Button>
    </div>
  </div>
</div>
