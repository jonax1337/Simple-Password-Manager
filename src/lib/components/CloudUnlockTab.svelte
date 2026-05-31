<script lang="ts">
  import { Button, Input, Label, Dialog, Checkbox, toast } from "$lib/ui";
  import { Cloud, ArrowDownToLine, Loader2, KeyRound, FileLock2, Fingerprint } from "@lucide/svelte";
  import { onMount } from "svelte";
  import {
    cloudLogin,
    cloudListVaults,
    cloudOpenVault,
    cloudPersistSession,
    cloudStatus,
    helloAvailable,
    helloCloudIsEnrolled,
    helloCloudStore,
    helloCloudRetrieve,
    type CloudVaultEntry,
  } from "$lib/tauri";
  import { appState } from "$lib/app-state.svelte";

  type Phase = "form" | "picker";

  type Props = {
    onUnlock: () => void | Promise<void>;
    /** If a Remember-Me session was rehydrated, this is the last opened
     * vault id — pre-selected in the picker so a single click + the
     * kdbx password lands the user in their familiar vault. */
    rehydratedVaultId?: string | null;
  };

  let { onUnlock, rehydratedVaultId = null }: Props = $props();

  let phase = $state<Phase>("form");

  // On mount: if the Rust side already has a live cloud session (Remember
  // Me from a prior run), skip the login form entirely and jump to the
  // picker. Pre-select the last vault so it's one click + the kdbx
  // password to be back in.
  onMount(async () => {
    // Hello availability probes are quick + non-prompting; run them
    // alongside the cloud-status check so the form renders with the
    // right buttons in one paint.
    Promise.all([
      helloAvailable().then((v) => (helloAvail = v)).catch(() => undefined),
      helloCloudIsEnrolled().then((v) => (helloEnrolled = v)).catch(() => undefined),
    ]);
    try {
      const cs = await cloudStatus();
      if (cs.linked) {
        username = cs.email ?? username;
        serverUrl = cs.server_url ?? serverUrl;
        vaults = await cloudListVaults();
        if (rehydratedVaultId) {
          selectedVaultId = rehydratedVaultId;
        } else if (vaults.length === 1) {
          selectedVaultId = vaults[0].id;
        }
        phase = "picker";
      }
    } catch {
      // Stale token → fall back to login form. User notices via missing
      // pre-fill; not worth blocking with a toast.
    }
  });

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

  // Hello state — drives the prominent "Sign in with Hello" button. We
  // probe both at mount so the UI doesn't blink the button in/out.
  let helloAvail = $state(false);
  let helloEnrolled = $state(false);

  // picker state
  let vaults = $state<CloudVaultEntry[]>([]);
  let selectedVaultId = $state<string | null>(null);

  // KDBX-password modal state — the cloud only undoes the outer (AES-GCM)
  // layer; the inner KeePass-AES still needs the master password the user
  // originally chose when creating the database.
  let kdbxDialogOpen = $state(false);
  let kdbxPassword = $state("");
  let pendingVault = $state<{ id: string; name: string } | null>(null);
  let kdbxBusy = $state(false);

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
      // Stash for the Hello enrollment dialog before wiping the bound
      // field so the password is gone from the DOM tree.
      pendingCloudPassword = masterPassword;
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

  function handleOpenSelected() {
    if (!selectedVaultId) return;
    const vault = vaults.find((v) => v.id === selectedVaultId);
    if (!vault) return;
    // Ask for the KDBX password first; the actual decryption happens in
    // confirmKdbxUnlock so a wrong password doesn't leave us in an
    // inconsistent state.
    pendingVault = { id: vault.id, name: vault.name };
    kdbxPassword = "";
    error = "";
    kdbxDialogOpen = true;
  }

  async function confirmKdbxUnlock() {
    if (!pendingVault || !kdbxPassword) return;
    kdbxBusy = true;
    error = "";
    try {
      // Single-call cloud open: server-fetch + AES-GCM unseal + KDBX-open
      // happens in-process in Rust. No file ever lands on disk.
      await cloudOpenVault(pendingVault.id, kdbxPassword);
      // Cloud-only mode: no local path, label comes from the vault name.
      appState.setDbPath("");
      appState.setCloudVaultName(pendingVault.name);
      appState.setCloudVaultId(pendingVault.id);
      // Pull fresh status to grab role + caller_user_id for permission gating.
      try {
        const cs = await cloudStatus();
        appState.setCloudVaultRole(cs.active_vault_role);
        appState.setCloudUserId(cs.user_id);
      } catch { /* stale snapshot — gate stays as it was */ }
      appState.rememberVaultPassword(pendingVault.id, kdbxPassword);
      const name = pendingVault.name;
      const vaultId = pendingVault.id;
      const vaultPw = kdbxPassword;
      kdbxDialogOpen = false;
      pendingVault = null;
      kdbxPassword = "";

      // Offer Hello enrollment if the device supports it and the user
      // hasn't enrolled yet. Skipping is the safe default — explicit
      // dialog so we never silently capture biometrics. Requires a fresh
      // cloud password (stashed during handleSignIn); rehydrated sessions
      // route through the "Enable Hello" button in the picker instead.
      if (helloAvail && !helloEnrolled && rememberMe && pendingCloudPassword) {
        pendingHelloBundle = {
          server_url: serverUrl,
          email: username,
          cloud_password: pendingCloudPassword,
          default_vault_id: vaultId,
          vault_passwords: appState.knownVaultPasswords(),
          // legacy shape for backwards compat
          vault_id: vaultId,
          vault_kdbx_password: vaultPw,
        };
        helloOfferOpen = true;
      }

      toast.success("Unlocked", `${name} opened from cloud`);
      await onUnlock();
    } catch (e) {
      error = String(e);
    } finally {
      kdbxBusy = false;
    }
  }

  // Hello bundle the upcoming confirmation dialog will persist if accepted.
  // `vault_passwords` is the map version of "all vaults this session has
  // unlocked"; populated from appState.knownVaultPasswords() at save time
  // so a Hello unlock can decrypt every familiar vault without re-prompts.
  type HelloBundle = {
    server_url: string;
    email: string;
    cloud_password: string;
    default_vault_id: string;
    vault_passwords: Record<string, string>;
    // Legacy fields kept for backwards-compat with bundles written by the
    // first version of this code. New writes populate both shapes; reads
    // tolerate either.
    vault_id?: string;
    vault_kdbx_password?: string;
  };
  let helloOfferOpen = $state(false);
  let pendingHelloBundle = $state<HelloBundle | null>(null);
  let helloSaveBusy = $state(false);

  // ----- explicit Hello enrollment (rehydrated session path) -----
  // The post-login auto-offer needs `pendingCloudPassword`, which is only
  // set when the user typed their cloud password into the form. Sessions
  // that came back via Remember-Me have no fresh password, so we offer
  // a manual "Enable Hello" dialog inside the picker that asks for it.
  let helloEnrollDialogOpen = $state(false);
  let helloEnrollCloudPw = $state("");
  let helloEnrollVaultPw = $state("");
  let helloEnrollBusy = $state(false);
  let helloEnrollError = $state("");

  function openHelloEnrollDialog() {
    helloEnrollCloudPw = "";
    helloEnrollVaultPw = "";
    helloEnrollError = "";
    helloEnrollDialogOpen = true;
  }

  async function confirmHelloEnroll() {
    if (!helloEnrollCloudPw) {
      helloEnrollError = "Cloud master password required.";
      return;
    }
    if (!selectedVaultId) {
      helloEnrollError = "Pick a vault first — that becomes the default.";
      return;
    }
    // We need the selected vault's KDBX password too — either freshly typed
    // here, or pulled from the session cache if the user already unlocked
    // it once in this run.
    const cachedPw = appState.getVaultPassword(selectedVaultId);
    const vaultPw = helloEnrollVaultPw || cachedPw;
    if (!vaultPw) {
      helloEnrollError = "KDBX password for the default vault required.";
      return;
    }

    helloEnrollBusy = true;
    helloEnrollError = "";
    try {
      // Verify cloud password by attempting a (no-op) login. Wrong password
      // would otherwise only fail on the next Hello unlock — surface it now.
      await cloudLogin(serverUrl, username, helloEnrollCloudPw);
      try { await cloudPersistSession(); } catch { /* best-effort */ }

      // Persist any newly typed vault password into the session cache so
      // the bundle picks it up below.
      appState.rememberVaultPassword(selectedVaultId, vaultPw);

      const bundle: HelloBundle = {
        server_url: serverUrl,
        email: username,
        cloud_password: helloEnrollCloudPw,
        default_vault_id: selectedVaultId,
        vault_passwords: appState.knownVaultPasswords(),
        vault_id: selectedVaultId,
        vault_kdbx_password: vaultPw,
      };
      await helloCloudStore(JSON.stringify(bundle));
      helloEnrolled = true;
      helloEnrollDialogOpen = false;
      helloEnrollCloudPw = "";
      helloEnrollVaultPw = "";
      toast.success("Hello enabled", "Next launch will unlock with one tap");
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("cancelled")) {
        helloEnrollError = msg;
      }
    } finally {
      helloEnrollBusy = false;
    }
  }
  // The cloud password from the login form is needed when persisting a
  // Hello bundle, but the login flow clears `masterPassword` for safety.
  // Stash it for the next dialog instead of holding it on the form field.
  let pendingCloudPassword = $state("");

  async function saveHelloBundle() {
    if (!pendingHelloBundle) return;
    helloSaveBusy = true;
    try {
      await helloCloudStore(JSON.stringify(pendingHelloBundle));
      helloEnrolled = true;
      helloOfferOpen = false;
      pendingHelloBundle = null;
      pendingCloudPassword = "";
      toast.success("Hello enabled", "Next launch will unlock with one tap");
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("cancelled")) {
        toast.error("Hello save failed", msg);
      }
    } finally {
      helloSaveBusy = false;
    }
  }

  function skipHelloBundle() {
    helloOfferOpen = false;
    pendingHelloBundle = null;
    pendingCloudPassword = "";
  }

  async function handleHelloUnlock() {
    loading = true;
    error = "";
    try {
      const json = await helloCloudRetrieve();
      const bundle: HelloBundle = JSON.parse(json);

      // Migrate legacy bundles that only carried a single vault.
      const passwordMap: Record<string, string> =
        bundle.vault_passwords && Object.keys(bundle.vault_passwords).length > 0
          ? { ...bundle.vault_passwords }
          : bundle.vault_id && bundle.vault_kdbx_password
            ? { [bundle.vault_id]: bundle.vault_kdbx_password }
            : {};
      const defaultVaultId =
        bundle.default_vault_id || bundle.vault_id || Object.keys(passwordMap)[0];
      if (!defaultVaultId) {
        throw new Error("Hello bundle has no vault");
      }
      const defaultPw = passwordMap[defaultVaultId];
      if (!defaultPw) {
        throw new Error("Hello bundle missing password for default vault");
      }

      // 1. Restore the cloud session from the bundle's saved password.
      await cloudLogin(bundle.server_url, bundle.email, bundle.cloud_password);
      try { await cloudPersistSession(); } catch { /* best-effort */ }

      // 2. Push every vault password we know into the session cache so
      //    subsequent switches require zero prompts.
      appState.loadVaultPasswords(passwordMap);

      // 3. Open the default vault straight into memory.
      await cloudOpenVault(defaultVaultId, defaultPw);

      appState.setDbPath("");
      appState.setCloudVaultId(defaultVaultId);
      // Resolve name + role from the listing in one shot — Hello bundle
      // only carries the password, not the role metadata.
      void cloudListVaults().then((vs) => {
        const v = vs.find((x) => x.id === defaultVaultId);
        if (v) {
          appState.setCloudVaultName(v.name);
          appState.setCloudVaultRole(v.role);
        }
      });
      try {
        const cs = await cloudStatus();
        appState.setCloudUserId(cs.user_id);
      } catch { /* ignore */ }

      toast.success("Unlocked", "Hello signed you in");
      await onUnlock();
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("cancelled")) {
        error = msg;
      }
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

    {#if helloAvail && helloEnrolled}
      <div class="space-y-2">
        <Button class="w-full" onclick={handleHelloUnlock} disabled={loading}>
          {#if loading}
            <Loader2 class="size-4 animate-spin" />
            Verifying…
          {:else}
            <Fingerprint class="size-4" />
            Unlock with Windows Hello
          {/if}
        </Button>
        <div class="relative my-2">
          <div class="absolute inset-0 flex items-center"><span class="w-full border-t"></span></div>
          <div class="relative flex justify-center">
            <span class="bg-background px-3 text-2xs uppercase tracking-wider text-muted-foreground">Or sign in manually</span>
          </div>
        </div>
      </div>
    {/if}

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

      <label class="flex items-center gap-2 text-sm cursor-pointer select-none">
        <Checkbox bind:checked={rememberMe} />
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

    {#if helloAvail && !helloEnrolled}
      <button
        type="button"
        onclick={openHelloEnrollDialog}
        class="w-full inline-flex items-center justify-center gap-2 h-9 px-4 rounded-md border border-dashed border-border bg-background hover:bg-accent/30 text-xs text-muted-foreground transition-colors"
      >
        <Fingerprint class="size-3.5" />
        Enable one-tap unlock with Windows Hello
      </button>
    {/if}
  {/if}
</div>

<!-- KDBX master password modal. The cloud only decrypts the outer AES-GCM
     wrapper; the inner KeePass-AES layer still needs the password the user
     originally chose when creating the database. We collect it in a proper
     dialog so it doesn't look like a browser hijack. -->
<Dialog
  bind:open={kdbxDialogOpen}
  title={pendingVault ? `Unlock "${pendingVault.name}"` : "Unlock vault"}
  description="Enter the KeePass master password. The vault opens directly into memory — no local file."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !kdbxBusy) {
      pendingVault = null;
      kdbxPassword = "";
    }
  }}
>
  <div class="space-y-3">
    <div class="space-y-1.5">
      <Label for="kdbx-pw">KeePass master password</Label>
      <Input
        id="kdbx-pw"
        type="password"
        bind:value={kdbxPassword}
        autofocus
        onkeydown={(e) => e.key === "Enter" && void confirmKdbxUnlock()}
        placeholder="••••••••"
      />
      <p class="text-2xs text-muted-foreground">
        This is the password you set when you first created the <code class="font-mono text-2xs">.kdbx</code>. Not the cloud login password.
      </p>
    </div>
    {#if error}
      <p class="text-xs text-destructive break-all">{error}</p>
    {/if}
  </div>

  {#snippet footer()}
    <Button
      variant="outline"
      onclick={() => (kdbxDialogOpen = false)}
      disabled={kdbxBusy}
    >
      Cancel
    </Button>
    <Button onclick={confirmKdbxUnlock} disabled={kdbxBusy || !kdbxPassword}>
      {#if kdbxBusy}
        <Loader2 class="size-4 animate-spin" />
        Opening…
      {:else}
        <FileLock2 class="size-4" />
        Unlock
      {/if}
    </Button>
  {/snippet}
</Dialog>

<!-- Post-login offer: enroll this account + vault behind Hello so the
     next launch is a one-tap unlock. Surfaced only when Hello is set up
     on the device and the user hasn't already enrolled. -->
<Dialog
  bind:open={helloOfferOpen}
  title="Unlock with Windows Hello next time?"
  description="Save your cloud account + this vault behind Hello. One tap on next launch and you land straight in here."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !helloSaveBusy) skipHelloBundle();
  }}
>
  <ul class="text-xs text-muted-foreground list-disc pl-4 space-y-1">
    <li>Hello prompt confirms the save right now (you'll see it).</li>
    <li>Credentials are stored in Windows Credential Manager, bound to your account.</li>
    <li>To revoke: Settings → Security → Windows Hello → Remove.</li>
  </ul>
  {#snippet footer()}
    <Button variant="outline" onclick={skipHelloBundle} disabled={helloSaveBusy}>Not now</Button>
    <Button onclick={saveHelloBundle} disabled={helloSaveBusy}>
      {#if helloSaveBusy}
        <Loader2 class="size-4 animate-spin" />
        Saving…
      {:else}
        <Fingerprint class="size-4" />
        Enable Hello
      {/if}
    </Button>
  {/snippet}
</Dialog>

<!-- Manual Hello enrollment for rehydrated sessions: we don't have the
     fresh cloud password from a form submission, so the user types it
     here. The selected vault's KDBX password is auto-pulled from the
     session cache when available. -->
<Dialog
  bind:open={helloEnrollDialogOpen}
  title="Enable Windows Hello"
  description="Save your cloud account + default vault behind Hello. One tap on next launch lands you in your vault."
  size="sm"
  onOpenChange={(o) => {
    if (!o && !helloEnrollBusy) {
      helloEnrollCloudPw = "";
      helloEnrollVaultPw = "";
      helloEnrollError = "";
    }
  }}
>
  <div class="space-y-3">
    <div class="space-y-1.5">
      <Label for="enroll-cloud-pw">Cloud master password</Label>
      <Input
        id="enroll-cloud-pw"
        type="password"
        bind:value={helloEnrollCloudPw}
        placeholder="••••••••"
        autofocus
      />
    </div>
    {#if selectedVaultId && !appState.getVaultPassword(selectedVaultId)}
      <div class="space-y-1.5">
        <Label for="enroll-vault-pw">KeePass password for the selected vault</Label>
        <Input
          id="enroll-vault-pw"
          type="password"
          bind:value={helloEnrollVaultPw}
          placeholder="••••••••"
        />
      </div>
    {:else if selectedVaultId}
      <p class="text-xs text-muted-foreground">
        KDBX password for this vault is already cached from this session — Hello will reuse it.
      </p>
    {/if}
    {#if helloEnrollError}
      <p class="text-xs text-destructive break-all">{helloEnrollError}</p>
    {/if}
  </div>
  {#snippet footer()}
    <Button
      variant="outline"
      onclick={() => (helloEnrollDialogOpen = false)}
      disabled={helloEnrollBusy}
    >
      Cancel
    </Button>
    <Button onclick={confirmHelloEnroll} disabled={helloEnrollBusy || !helloEnrollCloudPw}>
      {#if helloEnrollBusy}
        <Loader2 class="size-4 animate-spin" />
        Enabling…
      {:else}
        <Fingerprint class="size-4" />
        Enable Hello
      {/if}
    </Button>
  {/snippet}
</Dialog>
