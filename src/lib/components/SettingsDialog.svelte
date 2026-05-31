<script lang="ts">
  import { onMount } from "svelte";
  import {
    Button, Input, Label, Select, Switch, Dialog, toast,
    SettingsRow, UpdateCard,
  } from "$lib/ui";
  import { navRow } from "$lib/ui/recipes";
  import { cn } from "$lib/utils";
  import {
    Palette, Shield, Database, Settings as SettingsIcon, Info,
    Sun, Moon, Monitor, Lock, ShieldAlert, RefreshCw,
    KeyRound, Fingerprint, Rocket, Puzzle, Trash2, Heart, ExternalLink,
    Cloud, CloudOff, ArrowUpFromLine, ArrowDownToLine, Users,
  } from "@lucide/svelte";
  import VaultMembersDialog from "./VaultMembersDialog.svelte";
  import { theme, type ThemeMode } from "$lib/theme.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import { open as openShell } from "@tauri-apps/plugin-shell";
  import {
    enable as enableAutostart,
    disable as disableAutostart,
    isEnabled as isAutostartEnabled,
  } from "@tauri-apps/plugin-autostart";
  import {
    detectBrowsers,
    installNativeHost,
    uninstallNativeHost,
    listYubikeys,
    enableYubikey,
    disableYubikey,
    yubikeyEnabledForOpenDb,
    helloAvailable,
    helloIsEnrolled,
    helloStore,
    helloClear,
    cloudStatus,
    cloudSignup,
    cloudLogin,
    cloudPush,
    cloudPull,
    cloudDisconnect,
    cloudShareVault,
    cloudStatus as fetchCloudStatus,
    type BrowserInfo,
    type InstallReport,
    type YubikeyInfo,
    type CloudStatus,
  } from "$lib/tauri";
  import {
    getHibpEnabled,
    setHibpEnabled,
    getCloseToTray,
    setCloseToTray,
    setYubikeyHint,
  } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";

  type SectionId = "appearance" | "security" | "database" | "cloud" | "application" | "about";

  type Props = {
    open?: boolean;
    initialSection?: SectionId;
  };

  let { open = $bindable(false), initialSection = "appearance" }: Props = $props();
  let section = $state<SectionId>("appearance");

  $effect(() => {
    if (open) section = initialSection;
  });

  const sections: { id: SectionId; label: string; icon: typeof Palette }[] = [
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "security", label: "Security", icon: Shield },
    { id: "database", label: "Database", icon: Database },
    { id: "cloud", label: "Cloud Sync", icon: Cloud },
    { id: "application", label: "Application", icon: SettingsIcon },
    { id: "about", label: "About", icon: Info },
  ];

  // ---------- appearance ----------
  const themeItems = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  // ---------- security ----------
  let autoLockSeconds = $state("0");
  let hibp = $state(false);

  // ---------- application ----------
  let closeToTray = $state(true);
  let autostartOn = $state(false);

  let browsers = $state<BrowserInfo[] | null>(null);
  let extensionId = $state(typeof window !== "undefined" ? localStorage.getItem("browserExtensionId") ?? "" : "");
  let installStatus = $state<"idle" | "installing" | "done" | "error">("idle");
  let installReport = $state<InstallReport | null>(null);
  let installError = $state("");

  // ---------- yubikey ----------
  let yubikeyActive = $state(false);
  let yubikeyDialog = $state(false);
  let yubikeyDevices = $state<YubikeyInfo[]>([]);
  let yubikeySelected = $state<number | null>(null);
  let yubikeySlot = $state("2");
  let yubikeyDetecting = $state(false);
  let yubikeyBusy = $state(false);
  let yubikeyError = $state("");

  // ---------- hello ----------
  let helloAvail = $state(false);
  let helloEnrolled = $state(false);
  let helloDialogOpen = $state(false);
  let helloPassword = $state("");
  let helloBusy = $state(false);
  let helloError = $state("");

  // ---------- cloud sync ----------
  let cloudState = $state<CloudStatus>({
    linked: false,
    server_url: null,
    email: null,
    active_vault_id: null,
    active_vault_name: null,
  });
  let cloudMode = $state<"signup" | "login">("login");
  let cloudServerUrl = $state(
    typeof window !== "undefined"
      ? localStorage.getItem("cloudServerUrl") ?? "http://localhost:8090"
      : "http://localhost:8090",
  );
  let cloudEmail = $state(
    typeof window !== "undefined" ? localStorage.getItem("cloudEmail") ?? "" : "",
  );
  let cloudPassword = $state("");
  let cloudBusy = $state(false);
  let cloudError = $state("");

  async function refreshCloudStatus() {
    try {
      cloudState = await cloudStatus();
    } catch {
      // ignore — settings UI shouldn't block on it
    }
  }

  async function applyCloudLink() {
    cloudBusy = true;
    cloudError = "";
    try {
      if (cloudMode === "signup") {
        await cloudSignup(cloudServerUrl, cloudEmail, cloudPassword);
        toast.success("Cloud account created", "Vault uploaded");
      } else {
        await cloudLogin(cloudServerUrl, cloudEmail, cloudPassword);
        toast.success("Linked", `Signed in as ${cloudEmail}`);
      }
      localStorage.setItem("cloudServerUrl", cloudServerUrl);
      localStorage.setItem("cloudEmail", cloudEmail);
      cloudPassword = "";
      await refreshCloudStatus();
    } catch (e) {
      cloudError = String(e);
    } finally {
      cloudBusy = false;
    }
  }

  async function doCloudPush() {
    cloudBusy = true;
    cloudError = "";
    try {
      appState.setSyncStatus("cloud-sync");
      await cloudPush();
      appState.markSynced();
      toast.success("Pushed", "Local vault uploaded");
      await refreshCloudStatus();
    } catch (e) {
      appState.setSyncStatus("conflict");
      cloudError = String(e);
    } finally {
      cloudBusy = false;
    }
  }

  async function doCloudPull() {
    cloudBusy = true;
    cloudError = "";
    try {
      appState.setSyncStatus("cloud-sync");
      await cloudPull();
      appState.markSynced();
      appState.refresh();
      toast.success("Pulled", "Remote vault written to disk — reopen to load new contents");
      await refreshCloudStatus();
    } catch (e) {
      appState.setSyncStatus("conflict");
      cloudError = String(e);
    } finally {
      cloudBusy = false;
    }
  }

  // ---------- vault membership ----------
  let membersDialogOpen = $state(false);
  function openMembersDialog() {
    membersDialogOpen = true;
  }

  async function doCloudDisconnect() {
    try {
      await cloudDisconnect();
      appState.clearVaultPasswords();
      await refreshCloudStatus();
      toast.success("Disconnected");
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  // ---------- about ----------
  let version = $state("…");

  onMount(() => {
    const saved = localStorage.getItem("autoLockSeconds");
    if (saved) autoLockSeconds = saved;
    closeToTray = getCloseToTray();
    hibp = getHibpEnabled();
    isAutostartEnabled().then((v) => (autostartOn = v)).catch(() => (autostartOn = false));
    detectBrowsers().then((b) => (browsers = b)).catch(() => (browsers = []));
    yubikeyEnabledForOpenDb().then((v) => (yubikeyActive = v)).catch(() => (yubikeyActive = false));
    helloAvailable().then((v) => (helloAvail = v)).catch(() => (helloAvail = false));
    if (appState.dbPath) {
      helloIsEnrolled(appState.dbPath).then((v) => (helloEnrolled = v)).catch(() => (helloEnrolled = false));
    }
    getVersion().then((v) => (version = v)).catch(() => (version = "unknown"));
    void refreshCloudStatus();
  });

  function applyTheme(v: string) { theme.set(v as ThemeMode); }
  function applyAutoLock(v: string) { autoLockSeconds = v; localStorage.setItem("autoLockSeconds", v); }
  function applyCloseToTray(v: boolean) { closeToTray = v; setCloseToTray(v); }
  async function applyAutostart(v: boolean) {
    try {
      if (v) await enableAutostart(); else await disableAutostart();
      autostartOn = v;
      toast.success(v ? "Autostart enabled" : "Autostart disabled");
    } catch (e) { toast.error("Failed", String(e)); }
  }
  function applyHibp(v: boolean) { hibp = v; setHibpEnabled(v); }

  async function installExtension() {
    if (!extensionId.trim()) { toast.error("Extension ID required"); return; }
    installStatus = "installing"; installError = "";
    try {
      localStorage.setItem("browserExtensionId", extensionId.trim());
      const r = await installNativeHost(extensionId.trim());
      installReport = r;
      installStatus = "done";
    } catch (e) {
      installStatus = "error";
      installError = e instanceof Error ? e.message : String(e);
    }
  }

  async function removeExtension() {
    try {
      const removed = await uninstallNativeHost();
      toast.success("Removed", `${removed.length} entries removed`);
      installStatus = "idle";
      installReport = null;
    } catch (e) { toast.error("Failed", String(e)); }
  }

  async function openYubikeyDialog() {
    yubikeyDialog = true;
    yubikeyError = "";
    yubikeyDetecting = true;
    try {
      yubikeyDevices = await listYubikeys();
      if (yubikeyDevices.length > 0) yubikeySelected = yubikeyDevices[0].serial_number;
    } catch (e) { yubikeyError = String(e); }
    finally { yubikeyDetecting = false; }
  }

  async function applyYubikey() {
    if (yubikeySelected === null) return;
    yubikeyBusy = true; yubikeyError = "";
    try {
      await enableYubikey(yubikeySelected, yubikeySlot);
      if (appState.dbPath) setYubikeyHint(appState.dbPath, { serial_number: yubikeySelected, slot: yubikeySlot });
      yubikeyActive = true;
      yubikeyDialog = false;
      appState.markDirty();
      toast.success("Yubikey enrolled", "Save the database to persist this change");
    } catch (e) { yubikeyError = String(e); }
    finally { yubikeyBusy = false; }
  }

  async function disableYk() {
    const ok = await ask("Disable Yubikey for this database?", { kind: "warning", title: "Disable Yubikey" });
    if (!ok) return;
    try {
      await disableYubikey();
      if (appState.dbPath) setYubikeyHint(appState.dbPath, null);
      yubikeyActive = false;
      appState.markDirty();
      toast.success("Yubikey disabled");
    } catch (e) { toast.error("Failed", String(e)); }
  }

  async function applyHello() {
    if (!appState.dbPath) return;
    helloBusy = true; helloError = "";
    try {
      await helloStore(appState.dbPath, helloPassword);
      helloEnrolled = true;
      helloDialogOpen = false;
      helloPassword = "";
      toast.success("Windows Hello enrolled");
    } catch (e) { helloError = String(e); }
    finally { helloBusy = false; }
  }

  async function clearHello() {
    if (!appState.dbPath) return;
    const ok = await ask("Remove Windows Hello credential for this database?", { kind: "warning", title: "Remove credential" });
    if (!ok) return;
    try {
      await helloClear(appState.dbPath);
      helloEnrolled = false;
      toast.success("Credential removed");
    } catch (e) { toast.error("Failed", String(e)); }
  }
</script>

<Dialog bind:open bare size="xl">
  <!-- Nav rail -->
  <aside class="w-[200px] shrink-0 bg-sidebar border-r border-border-subtle flex flex-col">
    <div class="px-4 pt-4 pb-3 text-sm font-semibold tracking-tight">Settings</div>
    <nav class="flex-1 px-2 py-1 space-y-0.5 overflow-y-auto">
      {#each sections as s (s.id)}
        {@const active = section === s.id}
        <button
          type="button"
          onclick={() => (section = s.id)}
          class={cn(
            navRow({ active: false, size: "sm" }),
            active && "bg-primary/15 text-primary hover:bg-primary/15 hover:text-primary",
          )}
        >
          <s.icon class="size-4 shrink-0" />
          <span>{s.label}</span>
        </button>
      {/each}
    </nav>
  </aside>

  <!-- Content pane -->
  <div class="flex-1 min-w-0 flex flex-col">
    <div class="shrink-0 flex items-center justify-between border-b border-border-subtle px-6 py-3">
      <h2 class="text-base font-semibold">
        {sections.find((s) => s.id === section)?.label ?? "Settings"}
      </h2>
    </div>

        <div class="flex-1 overflow-y-auto px-6 py-5">
          {#if section === "appearance"}
            <div class="space-y-6 max-w-md">
              <SettingsRow title="Theme" description="Choose how the app looks.">
                <div class="flex items-center gap-3">
                  {#if theme.mode === "system"}
                    <Monitor class="h-5 w-5 text-muted-foreground" />
                  {:else if theme.mode === "light"}
                    <Sun class="h-5 w-5 text-muted-foreground" />
                  {:else}
                    <Moon class="h-5 w-5 text-muted-foreground" />
                  {/if}
                  <div class="w-48">
                    <Select items={themeItems} value={theme.mode} onValueChange={applyTheme} />
                  </div>
                </div>
              </SettingsRow>
            </div>
          {:else if section === "security"}
            <div class="space-y-8 max-w-md">
              <SettingsRow icon={Lock} title="Auto-lock" description="Lock the database after inactivity.">
                <div class="w-56">
                  <Select
                    items={[
                      { value: "0", label: "Never" },
                      { value: "60", label: "1 minute" },
                      { value: "300", label: "5 minutes" },
                      { value: "600", label: "10 minutes" },
                      { value: "1800", label: "30 minutes" },
                      { value: "3600", label: "1 hour" },
                    ]}
                    value={autoLockSeconds}
                    onValueChange={applyAutoLock}
                  />
                </div>
              </SettingsRow>

              <SettingsRow
                icon={ShieldAlert}
                title="Breach detection (HIBP)"
                description="Check passwords against haveibeenpwned.com using k-anonymity. No plaintext leaves your machine."
              >
                <label class="flex items-center gap-2">
                  <Switch checked={hibp} onCheckedChange={applyHibp} />
                  <span class="text-sm">{hibp ? "Enabled" : "Disabled"}</span>
                </label>
              </SettingsRow>

              <SettingsRow
                icon={KeyRound}
                title="Yubikey"
                description="Add an HMAC-SHA1 challenge-response factor for this database."
              >
                <div class="flex items-center gap-3">
                  <span class="text-sm {yubikeyActive ? 'text-success' : 'text-muted-foreground'}">
                    {yubikeyActive ? "Enabled" : "Not enrolled"}
                  </span>
                  {#if yubikeyActive}
                    <Button size="sm" variant="outline" onclick={disableYk}>Disable</Button>
                  {:else}
                    <Button size="sm" onclick={openYubikeyDialog}>Enable…</Button>
                  {/if}
                </div>
              </SettingsRow>

              {#if helloAvail}
                <SettingsRow
                  icon={Fingerprint}
                  title="Windows Hello"
                  description="Seal the master password behind Windows Hello for quick unlock."
                >
                  <div class="flex items-center gap-3">
                    <span class="text-sm {helloEnrolled ? 'text-success' : 'text-muted-foreground'}">
                      {helloEnrolled ? "Credential stored" : "Not enrolled"}
                    </span>
                    {#if helloEnrolled}
                      <Button size="sm" variant="outline" onclick={clearHello}>Remove</Button>
                    {:else}
                      <Button size="sm" onclick={() => (helloDialogOpen = true)}>Enable…</Button>
                    {/if}
                  </div>
                </SettingsRow>
              {/if}
            </div>
          {:else if section === "database"}
            <div class="space-y-8 max-w-md">
              <SettingsRow
                icon={RefreshCw}
                title="Sync"
                description="Changes are saved automatically to disk. If the file is modified by another app, the new version is merged in silently — no action needed."
              >
                {#if appState.dbPath}
                  <p class="text-xs text-muted-foreground break-all font-mono">{appState.dbPath}</p>
                {/if}
              </SettingsRow>
            </div>
          {:else if section === "cloud"}
            <div class="space-y-6 max-w-lg">
              <div class="rounded-md border bg-muted/30 p-3 text-xs text-muted-foreground space-y-1">
                <p class="font-medium text-foreground">End-to-end encrypted sync</p>
                <p>
                  Your master password never leaves this device. The server stores only an
                  Argon2-hashed proof of it plus the AES-GCM-encrypted vault blob. Run your own
                  instance — see <span class="font-mono">server/README.md</span>.
                </p>
              </div>

              {#if cloudState.linked}
                <SettingsRow
                  icon={Cloud}
                  title="Linked account"
                  description="This session is connected to a cloud account."
                >
                  <div class="space-y-2">
                    <p class="text-xs font-mono break-all">{cloudState.email}</p>
                    <p class="text-2xs text-muted-foreground break-all">{cloudState.server_url}</p>
                    {#if cloudState.active_vault_name}
                      <p class="text-2xs text-muted-foreground">
                        Active vault: <span class="font-medium">{cloudState.active_vault_name}</span>
                      </p>
                    {/if}
                    <div class="flex gap-2 pt-1">
                      <Button size="sm" onclick={doCloudPush} disabled={cloudBusy}>
                        <ArrowUpFromLine class="size-3.5" />
                        Push
                      </Button>
                      <Button size="sm" variant="outline" onclick={doCloudPull} disabled={cloudBusy}>
                        <ArrowDownToLine class="size-3.5" />
                        Pull
                      </Button>
                      <Button size="sm" variant="outline" onclick={doCloudDisconnect} disabled={cloudBusy}>
                        <CloudOff class="size-3.5" />
                        Disconnect
                      </Button>
                    </div>
                  </div>
                </SettingsRow>

                {#if cloudState.active_vault_id}
                  <SettingsRow
                    icon={Users}
                    title="Manage access"
                    description="Invite people to this vault, change roles, or revoke access."
                  >
                    <Button size="sm" onclick={openMembersDialog}>
                      <Users class="size-3.5" />
                      Manage members…
                    </Button>
                  </SettingsRow>
                {/if}
              {:else}
                <SettingsRow
                  icon={Cloud}
                  title="Link this vault to a server"
                  description={cloudMode === "signup"
                    ? "Creates a new server-side account using this database as the initial vault."
                    : "Sign in to an existing server account. Your local vault will be replaced on Pull."}
                >
                  <div class="space-y-3">
                    <div class="flex gap-1 text-xs">
                      <button
                        type="button"
                        class="px-2.5 py-1 rounded border {cloudMode === 'login' ? 'bg-primary text-primary-foreground border-primary' : 'bg-background hover:bg-accent/40'}"
                        onclick={() => (cloudMode = "login")}
                      >
                        Sign in
                      </button>
                      <button
                        type="button"
                        class="px-2.5 py-1 rounded border {cloudMode === 'signup' ? 'bg-primary text-primary-foreground border-primary' : 'bg-background hover:bg-accent/40'}"
                        onclick={() => (cloudMode = "signup")}
                      >
                        Create account
                      </button>
                    </div>

                    <div class="space-y-1.5">
                      <Label for="cloud-server">Server URL</Label>
                      <Input id="cloud-server" bind:value={cloudServerUrl} placeholder="https://vault.example.com" />
                    </div>
                    <div class="space-y-1.5">
                      <Label for="cloud-email">Email</Label>
                      <Input id="cloud-email" type="email" bind:value={cloudEmail} placeholder="you@example.com" />
                    </div>
                    <div class="space-y-1.5">
                      <Label for="cloud-pw">Master password</Label>
                      <Input
                        id="cloud-pw"
                        type="password"
                        bind:value={cloudPassword}
                        placeholder="••••••••"
                      />
                      <p class="text-2xs text-muted-foreground">
                        Used to derive your vault encryption key. Reuse the database's master
                        password to keep things simple.
                      </p>
                    </div>

                    <Button onclick={applyCloudLink} disabled={cloudBusy || !cloudPassword || !cloudEmail || !cloudServerUrl}>
                      {#if cloudBusy}
                        Working…
                      {:else if cloudMode === "signup"}
                        Create &amp; upload vault
                      {:else}
                        Sign in
                      {/if}
                    </Button>
                  </div>
                </SettingsRow>
              {/if}

              {#if cloudError}
                <p class="text-xs text-destructive break-all">{cloudError}</p>
              {/if}
            </div>
          {:else if section === "application"}
            <div class="space-y-8 max-w-md">
              <SettingsRow title="Close behavior" description="What happens when you close the window.">
                <label class="flex items-center gap-2">
                  <Switch checked={closeToTray} onCheckedChange={applyCloseToTray} />
                  <span class="text-sm">Minimize to system tray instead of quitting</span>
                </label>
              </SettingsRow>

              <SettingsRow icon={Rocket} title="Autostart" description="Launch the app when you sign in.">
                <label class="flex items-center gap-2">
                  <Switch checked={autostartOn} onCheckedChange={applyAutostart} />
                  <span class="text-sm">{autostartOn ? "Enabled" : "Disabled"}</span>
                </label>
              </SettingsRow>

              <SettingsRow icon={SettingsIcon} title="Updates">
                <UpdateCard variant="inline" />
              </SettingsRow>

              <SettingsRow
                icon={Puzzle}
                title="Browser extension"
                description="Connect a browser extension via the native messaging host."
              >
                {#if browsers && browsers.length > 0}
                  <p class="text-xs text-muted-foreground mb-2">Detected: {browsers.map((b) => b.label).join(", ")}</p>
                {/if}
                <div class="space-y-2">
                  <Label for="ext-id">Extension ID</Label>
                  <Input id="ext-id" bind:value={extensionId} placeholder="abcdefghijklmnopqrstuvwxyzabcdef" />
                  <div class="flex gap-2">
                    <Button size="sm" onclick={installExtension} disabled={installStatus === "installing"}>
                      {installStatus === "installing" ? "Installing…" : "Install"}
                    </Button>
                    <Button size="sm" variant="outline" onclick={removeExtension}>
                      <Trash2 class="h-3.5 w-3.5" /> Remove
                    </Button>
                  </div>
                </div>
                {#if installReport}
                  <p class="text-xs text-muted-foreground pt-2">
                    Installed for: {installReport.registered.join(", ") || "none"}
                  </p>
                  {#if installReport.failed.length > 0}
                    <p class="text-xs text-destructive">
                      Failed: {installReport.failed.map((f) => `${f.label} (${f.reason})`).join(", ")}
                    </p>
                  {/if}
                {/if}
                {#if installError}
                  <p class="text-xs text-destructive">{installError}</p>
                {/if}
              </SettingsRow>
            </div>
          {:else if section === "about"}
            <div class="flex flex-col items-center text-center space-y-4 max-w-sm mx-auto pt-4">
              <img src="/app-icon.png" alt="App icon" class="size-20 drop-shadow-md" />
              <div>
                <h2 class="text-xl font-semibold tracking-tight">Simple Password Manager</h2>
                <p class="text-xs text-muted-foreground mt-1">Version {version}</p>
              </div>
              <p class="text-sm text-muted-foreground">
                A secure and modern password manager built with the proven KeePass database format.
              </p>
              <Button variant="outline" class="w-full" onclick={() => openShell("https://github.com/jonax1337/Simple-Password-Manager")}>
                <ExternalLink class="size-3.5" />
                View on GitHub
              </Button>
              <div class="pt-2 text-2xs text-muted-foreground space-y-0.5">
                <p class="flex items-center justify-center gap-1">
                  Made with <Heart class="h-3 w-3 fill-current text-destructive" /> by Jonas Laux
                </p>
                <p>Open Source · MIT License</p>
              </div>
            </div>
          {/if}
        </div>
      </div>
</Dialog>

<!-- Yubikey enroll dialog -->
<Dialog bind:open={yubikeyDialog} title="Enable Yubikey" description="Choose a Yubikey and a slot to enroll for this database.">
  {#if yubikeyDetecting}
    <p class="text-sm text-muted-foreground">Detecting Yubikeys…</p>
  {:else if yubikeyDevices.length === 0}
    <p class="text-sm text-destructive">No Yubikeys detected. Plug one in and reopen this dialog.</p>
  {:else}
    <div class="space-y-2">
      <Label>Device</Label>
      <Select
        items={yubikeyDevices.map((d) => ({
          value: String(d.serial_number),
          label: `${d.name ?? "Yubikey"} (#${d.serial_number})`,
        }))}
        value={yubikeySelected !== null ? String(yubikeySelected) : undefined}
        onValueChange={(v) => (yubikeySelected = Number(v))}
      />
    </div>
    <div class="space-y-2">
      <Label>Slot</Label>
      <Select items={[{ value: "1", label: "Slot 1" }, { value: "2", label: "Slot 2" }]} bind:value={yubikeySlot} />
    </div>
  {/if}
  {#if yubikeyError}
    <p class="text-xs text-destructive">{yubikeyError}</p>
  {/if}
  {#snippet footer()}
    <Button variant="outline" onclick={() => (yubikeyDialog = false)} disabled={yubikeyBusy}>Cancel</Button>
    <Button onclick={applyYubikey} disabled={yubikeyBusy || yubikeySelected === null}>
      {yubikeyBusy ? "Enrolling…" : "Enroll"}
    </Button>
  {/snippet}
</Dialog>

<!-- Vault members dialog -->
<VaultMembersDialog
  bind:open={membersDialogOpen}
  vaultId={cloudState.active_vault_id}
  vaultName={cloudState.active_vault_name ?? ""}
  callerRole={appState.cloudVaultRole}
  callerUserId={appState.cloudUserId}
/>

<!-- Hello enroll dialog -->
<Dialog
  bind:open={helloDialogOpen}
  title="Enable Windows Hello"
  description="Enter your master password — it will be sealed behind Windows Hello."
>
  <div class="space-y-2">
    <Label for="hello-pw">Master Password</Label>
    <Input id="hello-pw" type="password" bind:value={helloPassword} />
  </div>
  {#if helloError}
    <p class="text-xs text-destructive">{helloError}</p>
  {/if}
  {#snippet footer()}
    <Button variant="outline" onclick={() => { helloDialogOpen = false; helloPassword = ""; }} disabled={helloBusy}>Cancel</Button>
    <Button onclick={applyHello} disabled={helloBusy || !helloPassword}>
      {helloBusy ? "Storing…" : "Store"}
    </Button>
  {/snippet}
</Dialog>
