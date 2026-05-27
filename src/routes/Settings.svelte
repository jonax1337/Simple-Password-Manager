<script lang="ts">
  import { onMount } from "svelte";
  import { pop } from "svelte-spa-router";
  import {
    Button,
    Card,
    CardHeader,
    CardTitle,
    CardDescription,
    CardContent,
    Input,
    Label,
    Select,
    Switch,
    Tabs,
    TabContent,
    Dialog,
    toast,
  } from "$lib/ui";
  import {
    ArrowLeft, Sun, Moon, Monitor, Lock, ShieldAlert, Download, RefreshCw, KeyRound, Fingerprint, Rocket, Puzzle, Trash2,
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
    detectBrowsers, installNativeHost, uninstallNativeHost,
    listYubikeys, enableYubikey, disableYubikey, yubikeyEnabledForOpenDb,
    helloAvailable, helloIsEnrolled, helloStore, helloClear,
    type BrowserInfo, type InstallReport, type YubikeyInfo,
  } from "$lib/tauri";
  import {
    getHibpEnabled, setHibpEnabled,
    getLiveUpdates, setLiveUpdates,
    getCloseToTray, setCloseToTray,
    setYubikeyHint,
  } from "$lib/storage";
  import { appState } from "$lib/app-state.svelte";

  let tab = $state("appearance");

  // appearance
  const themeItems = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  // security
  let autoLockSeconds = $state("0");
  let hibp = $state(false);

  // database
  let liveUpdatesEnabled = $state(false);

  // application
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

  // Yubikey
  let yubikeyActive = $state(false);
  let yubikeyDialog = $state(false);
  let yubikeyDevices = $state<YubikeyInfo[]>([]);
  let yubikeySelected = $state<number | null>(null);
  let yubikeySlot = $state("2");
  let yubikeyDetecting = $state(false);
  let yubikeyBusy = $state(false);
  let yubikeyError = $state("");

  // Hello
  let helloAvail = $state(false);
  let helloEnrolled = $state(false);
  let helloDialog = $state(false);
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
    window.dispatchEvent(new CustomEvent("liveUpdatesChanged", { detail: { enabled: v } }));
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
      helloDialog = false;
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

<div class="flex h-full flex-col">
  <div class="shrink-0 flex items-center gap-2 border-b px-4 py-3">
    <Button variant="ghost" size="icon" onclick={() => pop()}>
      <ArrowLeft class="h-4 w-4" />
    </Button>
    <h1 class="text-lg font-semibold">Settings</h1>
  </div>

  <Tabs bind:value={tab} tabs={[
    { value: "appearance", label: "Appearance" },
    { value: "security", label: "Security" },
    { value: "database", label: "Database" },
    { value: "application", label: "Application" },
  ]} class="flex-1 min-h-0 px-4 pt-3">
    {#snippet children(_value: string)}
      <TabContent value="appearance" class="overflow-y-auto pt-2 pb-6">
        <div class="max-w-2xl space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Theme</CardTitle>
              <CardDescription>Choose how the app looks</CardDescription>
            </CardHeader>
            <CardContent>
              <div class="flex items-center gap-3">
                {#if theme.mode === "system"}<Monitor class="h-5 w-5" />{:else if theme.mode === "light"}<Sun class="h-5 w-5" />{:else}<Moon class="h-5 w-5" />{/if}
                <div class="w-48">
                  <Select items={themeItems} value={theme.mode} onValueChange={applyTheme} />
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </TabContent>

      <TabContent value="security" class="overflow-y-auto pt-2 pb-6">
        <div class="max-w-2xl space-y-4">
          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><Lock class="h-4 w-4" /> Auto-lock</CardTitle>
              <CardDescription>Automatically lock the database after a period of inactivity</CardDescription>
            </CardHeader>
            <CardContent>
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
                class="w-48"
              />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><ShieldAlert class="h-4 w-4" /> Breach detection (HIBP)</CardTitle>
              <CardDescription>Check passwords against haveibeenpwned.com (uses k-anonymity, no plaintext leaves your machine)</CardDescription>
            </CardHeader>
            <CardContent>
              <label class="flex items-center gap-2">
                <Switch checked={hibp} onCheckedChange={applyHibp} />
                <span class="text-sm">{hibp ? "Enabled" : "Disabled"}</span>
              </label>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><KeyRound class="h-4 w-4" /> Yubikey</CardTitle>
              <CardDescription>Add a Yubikey HMAC-SHA1 challenge-response as a second factor</CardDescription>
            </CardHeader>
            <CardContent class="flex items-center gap-3">
              <span class="text-sm {yubikeyActive ? 'text-success' : 'text-muted-foreground'}">
                {yubikeyActive ? "Enabled for this database" : "Not enrolled"}
              </span>
              {#if yubikeyActive}
                <Button size="sm" variant="outline" onclick={disableYk}>Disable</Button>
              {:else}
                <Button size="sm" onclick={openYubikeyDialog}>Enable…</Button>
              {/if}
            </CardContent>
          </Card>

          {#if helloAvail}
            <Card>
              <CardHeader>
                <CardTitle class="flex items-center gap-2"><Fingerprint class="h-4 w-4" /> Windows Hello</CardTitle>
                <CardDescription>Store the master password behind Windows Hello for quick unlock</CardDescription>
              </CardHeader>
              <CardContent class="flex items-center gap-3">
                <span class="text-sm {helloEnrolled ? 'text-success' : 'text-muted-foreground'}">
                  {helloEnrolled ? "Credential stored" : "Not enrolled"}
                </span>
                {#if helloEnrolled}
                  <Button size="sm" variant="outline" onclick={clearHello}>Remove</Button>
                {:else}
                  <Button size="sm" onclick={() => (helloDialog = true)}>Enable…</Button>
                {/if}
              </CardContent>
            </Card>
          {/if}
        </div>
      </TabContent>

      <TabContent value="database" class="overflow-y-auto pt-2 pb-6">
        <div class="max-w-2xl space-y-4">
          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><RefreshCw class="h-4 w-4" /> Live updates</CardTitle>
              <CardDescription>Automatically merge changes from disk every few seconds</CardDescription>
            </CardHeader>
            <CardContent>
              <label class="flex items-center gap-2">
                <Switch checked={liveUpdatesEnabled} onCheckedChange={applyLive} />
                <span class="text-sm">{liveUpdatesEnabled ? "Enabled" : "Disabled"}</span>
              </label>
              {#if appState.dbPath}
                <p class="text-xs text-muted-foreground mt-2 break-all font-mono">{appState.dbPath}</p>
              {/if}
            </CardContent>
          </Card>
        </div>
      </TabContent>

      <TabContent value="application" class="overflow-y-auto pt-2 pb-6">
        <div class="max-w-2xl space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Close behaviour</CardTitle>
              <CardDescription>What happens when you close the main window</CardDescription>
            </CardHeader>
            <CardContent>
              <label class="flex items-center gap-2">
                <Switch checked={closeToTray} onCheckedChange={applyCloseToTray} />
                <span class="text-sm">Minimize to system tray instead of quitting</span>
              </label>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><Rocket class="h-4 w-4" /> Autostart</CardTitle>
              <CardDescription>Launch the app when you sign in</CardDescription>
            </CardHeader>
            <CardContent>
              <label class="flex items-center gap-2">
                <Switch checked={autostartOn} onCheckedChange={applyAutostart} />
                <span class="text-sm">{autostartOn ? "Enabled" : "Disabled"}</span>
              </label>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><Download class="h-4 w-4" /> Updates</CardTitle>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="text-sm text-muted-foreground">
                {#if updateStatus === "idle"}Check for a newer version{:else if updateStatus === "checking"}Checking…{:else if updateStatus === "uptodate"}You're on the latest version{:else if updateStatus === "available"}<span class="font-medium text-foreground">v{updateVersion}</span> is ready to install{:else if updateStatus === "installing"}Downloading and installing…{:else if updateStatus === "error"}<span class="text-destructive">{updateError}</span>{/if}
              </div>
              {#if updateStatus === "available"}
                <Button onclick={install} disabled={false}>Install &amp; restart</Button>
              {:else}
                <Button variant="outline" onclick={check} disabled={updateStatus === "checking" || updateStatus === "installing"}>
                  {updateStatus === "checking" ? "Checking…" : "Check for updates"}
                </Button>
              {/if}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2"><Puzzle class="h-4 w-4" /> Browser extension</CardTitle>
              <CardDescription>Connect a browser extension via the native messaging host</CardDescription>
            </CardHeader>
            <CardContent class="space-y-3">
              {#if browsers && browsers.length > 0}
                <p class="text-xs text-muted-foreground">
                  Detected: {browsers.map((b) => b.label).join(", ")}
                </p>
              {/if}
              <div class="space-y-2">
                <Label for="ext-id">Extension ID</Label>
                <Input id="ext-id" bind:value={extensionId} placeholder="abcdefghijklmnopqrstuvwxyzabcdef" />
              </div>
              <div class="flex gap-2">
                <Button size="sm" onclick={installExtension} disabled={installStatus === "installing"}>
                  {installStatus === "installing" ? "Installing…" : "Install"}
                </Button>
                <Button size="sm" variant="outline" onclick={removeExtension}>
                  <Trash2 class="h-3.5 w-3.5" />
                  Remove
                </Button>
              </div>
              {#if installReport}
                <div class="text-xs text-muted-foreground">
                  Installed for: {installReport.registered.join(", ") || "none"}
                  {#if installReport.failed.length > 0}
                    <div class="text-destructive">
                      Failed: {installReport.failed.map((f) => `${f.label} (${f.reason})`).join(", ")}
                    </div>
                  {/if}
                </div>
              {/if}
              {#if installError}
                <p class="text-xs text-destructive">{installError}</p>
              {/if}
            </CardContent>
          </Card>
        </div>
      </TabContent>
    {/snippet}
  </Tabs>
</div>

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
<Dialog bind:open={helloDialog} title="Enable Windows Hello" description="Enter your master password — it will be sealed behind Windows Hello.">
  <div class="space-y-2">
    <Label for="hello-pw">Master Password</Label>
    <Input id="hello-pw" type="password" bind:value={helloPassword} />
  </div>
  {#if helloError}
    <p class="text-xs text-destructive">{helloError}</p>
  {/if}
  {#snippet footer()}
    <Button variant="outline" onclick={() => { helloDialog = false; helloPassword = ""; }} disabled={helloBusy}>Cancel</Button>
    <Button onclick={applyHello} disabled={helloBusy || !helloPassword}>
      {helloBusy ? "Storing…" : "Store"}
    </Button>
  {/snippet}
</Dialog>
