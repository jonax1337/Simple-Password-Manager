<script lang="ts">
  import { onMount } from "svelte";
  import { Dialog as DialogPrimitive } from "bits-ui";
  import {
    Button,
    Input,
    Label,
    Select,
    Switch,
    Dialog,
    toast,
  } from "$lib/ui";
  import {
    Palette,
    Shield,
    Database,
    Settings as SettingsIcon,
    Info,
    Sun,
    Moon,
    Monitor,
    Lock,
    ShieldAlert,
    Download,
    RefreshCw,
    KeyRound,
    Fingerprint,
    Rocket,
    Puzzle,
    Trash2,
    X,
  } from "@lucide/svelte";
  import { theme, type ThemeMode } from "$lib/theme.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
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
    type BrowserInfo,
    type InstallReport,
    type YubikeyInfo,
  } from "$lib/tauri";
  import {
    getHibpEnabled,
    setHibpEnabled,
    getLiveUpdates,
    setLiveUpdates,
    getCloseToTray,
    setCloseToTray,
    setYubikeyHint,
  } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";

  type Props = {
    open?: boolean;
    initialSection?: SectionId;
    onOpenAbout?: () => void;
  };

  type SectionId = "appearance" | "security" | "database" | "application" | "about";

  let { open = $bindable(false), initialSection = "appearance", onOpenAbout }: Props = $props();

  // svelte-ignore state_referenced_locally
  let section = $state<SectionId>(initialSection);

  $effect(() => {
    if (open) section = initialSection;
  });

  const sections: { id: SectionId; label: string; icon: typeof Palette }[] = [
    { id: "appearance", label: "Appearance", icon: Palette },
    { id: "security", label: "Security", icon: Shield },
    { id: "database", label: "Database", icon: Database },
    { id: "application", label: "Application", icon: SettingsIcon },
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

  // ---------- database ----------
  let liveUpdatesEnabled = $state(false);

  // ---------- application ----------
  let closeToTray = $state(true);
  let autostartOn = $state(false);
  let updateStatus = $state<"idle" | "checking" | "uptodate" | "available" | "installing" | "error">("idle");
  let updateVersion = $state("");
  let updateError = $state("");

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

  onMount(() => {
    const saved = localStorage.getItem("autoLockSeconds");
    if (saved) autoLockSeconds = saved;
    closeToTray = getCloseToTray();
    hibp = getHibpEnabled();
    if (appState.dbPath) liveUpdatesEnabled = getLiveUpdates(appState.dbPath);
    isAutostartEnabled().then((v) => (autostartOn = v)).catch(() => (autostartOn = false));
    detectBrowsers().then((b) => (browsers = b)).catch(() => (browsers = []));
    yubikeyEnabledForOpenDb().then((v) => (yubikeyActive = v)).catch(() => (yubikeyActive = false));
    helloAvailable().then((v) => (helloAvail = v)).catch(() => (helloAvail = false));
    if (appState.dbPath) {
      helloIsEnrolled(appState.dbPath).then((v) => (helloEnrolled = v)).catch(() => (helloEnrolled = false));
    }
  });

  function applyTheme(v: string) {
    theme.set(v as ThemeMode);
  }

  function applyAutoLock(v: string) {
    autoLockSeconds = v;
    localStorage.setItem("autoLockSeconds", v);
  }

  function applyCloseToTray(v: boolean) {
    closeToTray = v;
    setCloseToTray(v);
  }

  async function applyAutostart(v: boolean) {
    try {
      if (v) await enableAutostart();
      else await disableAutostart();
      autostartOn = v;
      toast.success(v ? "Autostart enabled" : "Autostart disabled");
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  function applyHibp(v: boolean) {
    hibp = v;
    setHibpEnabled(v);
  }

  function applyLive(v: boolean) {
    if (!appState.dbPath) return;
    setLiveUpdates(appState.dbPath, v);
    liveUpdatesEnabled = v;
  }

  async function check() {
    updateStatus = "checking";
    try {
      const u = await checkForUpdate();
      if (u) {
        updateStatus = "available";
        updateVersion = u.version;
      } else {
        updateStatus = "uptodate";
      }
    } catch (e) {
      updateStatus = "error";
      updateError = e instanceof Error ? e.message : "Check failed";
    }
  }

  async function install() {
    updateStatus = "installing";
    try {
      const u = await checkForUpdate();
      if (u) {
        await u.downloadAndInstall();
        await relaunch();
      }
    } catch (e) {
      updateStatus = "error";
      updateError = e instanceof Error ? e.message : "Install failed";
    }
  }

  async function installExtension() {
    if (!extensionId.trim()) {
      toast.error("Extension ID required");
      return;
    }
    installStatus = "installing";
    installError = "";
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
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  async function openYubikeyDialog() {
    yubikeyDialog = true;
    yubikeyError = "";
    yubikeyDetecting = true;
    try {
      yubikeyDevices = await listYubikeys();
      if (yubikeyDevices.length > 0) yubikeySelected = yubikeyDevices[0].serial_number;
    } catch (e) {
      yubikeyError = String(e);
    } finally {
      yubikeyDetecting = false;
    }
  }

  async function applyYubikey() {
    if (yubikeySelected === null) return;
    yubikeyBusy = true;
    yubikeyError = "";
    try {
      await enableYubikey(yubikeySelected, yubikeySlot);
      if (appState.dbPath) setYubikeyHint(appState.dbPath, { serial_number: yubikeySelected, slot: yubikeySlot });
      yubikeyActive = true;
      yubikeyDialog = false;
      appState.markDirty();
      toast.success("Yubikey enrolled", "Save the database to persist this change");
    } catch (e) {
      yubikeyError = String(e);
    } finally {
      yubikeyBusy = false;
    }
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
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }

  async function applyHello() {
    if (!appState.dbPath) return;
    helloBusy = true;
    helloError = "";
    try {
      await helloStore(appState.dbPath, helloPassword);
      helloEnrolled = true;
      helloDialogOpen = false;
      helloPassword = "";
      toast.success("Windows Hello enrolled");
    } catch (e) {
      helloError = String(e);
    } finally {
      helloBusy = false;
    }
  }

  async function clearHello() {
    if (!appState.dbPath) return;
    const ok = await ask("Remove Windows Hello credential for this database?", {
      kind: "warning",
      title: "Remove credential",
    });
    if (!ok) return;
    try {
      await helloClear(appState.dbPath);
      helloEnrolled = false;
      toast.success("Credential removed");
    } catch (e) {
      toast.error("Failed", String(e));
    }
  }
</script>

<DialogPrimitive.Root bind:open>
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay
      class="fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
    />
    <DialogPrimitive.Content
      class="fixed top-[50%] left-[50%] z-50 grid w-full max-w-[860px] h-[min(640px,80vh)] translate-x-[-50%] translate-y-[-50%] grid-cols-[200px_1fr] overflow-hidden rounded-xl border bg-background shadow-2xl outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
    >
      <DialogPrimitive.Title class="sr-only">Settings</DialogPrimitive.Title>
      <DialogPrimitive.Description class="sr-only">Application settings</DialogPrimitive.Description>

      <!-- Internal sidebar -->
      <aside class="bg-sidebar border-r flex flex-col">
        <div class="px-4 pt-4 pb-3 text-sm font-semibold tracking-tight">Settings</div>
        <nav class="flex-1 px-2 py-1 space-y-0.5 overflow-y-auto">
          {#each sections as s (s.id)}
            {@const active = section === s.id}
            <button
              type="button"
              onclick={() => (section = s.id)}
              class="w-full flex items-center gap-2.5 rounded-md px-2.5 h-8 text-sm font-medium transition-colors {active
                ? 'bg-primary/15 text-primary'
                : 'text-foreground/80 hover:bg-accent/60'}"
            >
              <s.icon class="size-4 shrink-0" />
              <span>{s.label}</span>
            </button>
          {/each}
        </nav>
        {#if onOpenAbout}
          <div class="p-2 border-t">
            <button
              type="button"
              onclick={onOpenAbout}
              class="w-full flex items-center gap-2.5 rounded-md px-2.5 h-8 text-sm text-muted-foreground hover:bg-accent/60 hover:text-foreground transition-colors"
            >
              <Info class="size-4" />
              <span>About</span>
            </button>
          </div>
        {/if}
      </aside>

      <!-- Right pane -->
      <div class="relative flex flex-col min-h-0">
        <div class="shrink-0 flex items-center justify-between border-b px-6 py-3">
          <h2 class="text-base font-semibold">
            {sections.find((s) => s.id === section)?.label ?? "Settings"}
          </h2>
          <DialogPrimitive.Close
            class="size-7 inline-flex items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
          >
            <X class="size-4" />
            <span class="sr-only">Close</span>
          </DialogPrimitive.Close>
        </div>

        <div class="flex-1 overflow-y-auto px-6 py-4">
          {#if section === "appearance"}
            <div class="space-y-6">
              <div class="space-y-1.5">
                <h3 class="text-sm font-medium">Theme</h3>
                <p class="text-xs text-muted-foreground">Choose how the app looks.</p>
                <div class="flex items-center gap-3 pt-2">
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
              </div>
            </div>
          {:else if section === "security"}
            <div class="space-y-8">
              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <Lock class="h-4 w-4" /> Auto-lock
                </h3>
                <p class="text-xs text-muted-foreground">Lock the database after inactivity.</p>
                <div class="pt-2 w-56">
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
              </div>

              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <ShieldAlert class="h-4 w-4" /> Breach detection (HIBP)
                </h3>
                <p class="text-xs text-muted-foreground">
                  Check passwords against haveibeenpwned.com using k-anonymity. No plaintext leaves your
                  machine.
                </p>
                <label class="flex items-center gap-2 pt-2">
                  <Switch checked={hibp} onCheckedChange={applyHibp} />
                  <span class="text-sm">{hibp ? "Enabled" : "Disabled"}</span>
                </label>
              </div>

              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <KeyRound class="h-4 w-4" /> Yubikey
                </h3>
                <p class="text-xs text-muted-foreground">
                  Add an HMAC-SHA1 challenge-response factor for this database.
                </p>
                <div class="flex items-center gap-3 pt-2">
                  <span class="text-sm {yubikeyActive ? 'text-success' : 'text-muted-foreground'}">
                    {yubikeyActive ? "Enabled" : "Not enrolled"}
                  </span>
                  {#if yubikeyActive}
                    <Button size="sm" variant="outline" onclick={disableYk}>Disable</Button>
                  {:else}
                    <Button size="sm" onclick={openYubikeyDialog}>Enable…</Button>
                  {/if}
                </div>
              </div>

              {#if helloAvail}
                <div class="space-y-1.5">
                  <h3 class="text-sm font-medium flex items-center gap-2">
                    <Fingerprint class="h-4 w-4" /> Windows Hello
                  </h3>
                  <p class="text-xs text-muted-foreground">
                    Seal the master password behind Windows Hello for quick unlock.
                  </p>
                  <div class="flex items-center gap-3 pt-2">
                    <span class="text-sm {helloEnrolled ? 'text-success' : 'text-muted-foreground'}">
                      {helloEnrolled ? "Credential stored" : "Not enrolled"}
                    </span>
                    {#if helloEnrolled}
                      <Button size="sm" variant="outline" onclick={clearHello}>Remove</Button>
                    {:else}
                      <Button size="sm" onclick={() => (helloDialogOpen = true)}>Enable…</Button>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {:else if section === "database"}
            <div class="space-y-8">
              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <RefreshCw class="h-4 w-4" /> Live updates
                </h3>
                <p class="text-xs text-muted-foreground">
                  Auto-merge changes from disk every few seconds.
                </p>
                <label class="flex items-center gap-2 pt-2">
                  <Switch checked={liveUpdatesEnabled} onCheckedChange={applyLive} />
                  <span class="text-sm">{liveUpdatesEnabled ? "Enabled" : "Disabled"}</span>
                </label>
                {#if appState.dbPath}
                  <p class="text-xs text-muted-foreground mt-2 break-all font-mono">{appState.dbPath}</p>
                {/if}
              </div>
            </div>
          {:else if section === "application"}
            <div class="space-y-8">
              <div class="space-y-1.5">
                <h3 class="text-sm font-medium">Close behavior</h3>
                <p class="text-xs text-muted-foreground">What happens when you close the window.</p>
                <label class="flex items-center gap-2 pt-2">
                  <Switch checked={closeToTray} onCheckedChange={applyCloseToTray} />
                  <span class="text-sm">Minimize to system tray instead of quitting</span>
                </label>
              </div>

              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <Rocket class="h-4 w-4" /> Autostart
                </h3>
                <p class="text-xs text-muted-foreground">Launch the app when you sign in.</p>
                <label class="flex items-center gap-2 pt-2">
                  <Switch checked={autostartOn} onCheckedChange={applyAutostart} />
                  <span class="text-sm">{autostartOn ? "Enabled" : "Disabled"}</span>
                </label>
              </div>

              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <Download class="h-4 w-4" /> Updates
                </h3>
                <p class="text-xs text-muted-foreground">
                  {#if updateStatus === "idle"}Check for a newer version.{:else if updateStatus === "checking"}Checking…{:else if updateStatus === "uptodate"}You're on the latest version.{:else if updateStatus === "available"}<span class="font-medium text-foreground">v{updateVersion}</span> is ready to install.{:else if updateStatus === "installing"}Downloading and installing…{:else if updateStatus === "error"}<span class="text-destructive">{updateError}</span>{/if}
                </p>
                <div class="pt-2">
                  {#if updateStatus === "available"}
                    <Button size="sm" onclick={install}>Install &amp; restart</Button>
                  {:else}
                    <Button
                      size="sm"
                      variant="outline"
                      onclick={check}
                      disabled={updateStatus === "checking" || updateStatus === "installing"}
                    >
                      {updateStatus === "checking" ? "Checking…" : "Check for updates"}
                    </Button>
                  {/if}
                </div>
              </div>

              <div class="space-y-1.5">
                <h3 class="text-sm font-medium flex items-center gap-2">
                  <Puzzle class="h-4 w-4" /> Browser extension
                </h3>
                <p class="text-xs text-muted-foreground">
                  Connect a browser extension via the native messaging host.
                </p>
                {#if browsers && browsers.length > 0}
                  <p class="text-xs text-muted-foreground">
                    Detected: {browsers.map((b) => b.label).join(", ")}
                  </p>
                {/if}
                <div class="pt-2 space-y-2">
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
              </div>
            </div>
          {/if}
        </div>
      </div>
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>

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
      <Select
        items={[{ value: "1", label: "Slot 1" }, { value: "2", label: "Slot 2" }]}
        bind:value={yubikeySlot}
      />
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
    <Button variant="outline" onclick={() => { helloDialogOpen = false; helloPassword = ""; }} disabled={helloBusy}>
      Cancel
    </Button>
    <Button onclick={applyHello} disabled={helloBusy || !helloPassword}>
      {helloBusy ? "Storing…" : "Store"}
    </Button>
  {/snippet}
</Dialog>
