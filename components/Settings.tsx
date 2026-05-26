"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Settings as SettingsIcon, Moon, Sun, Monitor, Lock, Timer, X, Minimize2, ShieldAlert, RefreshCw, Rocket, Download, Puzzle } from "lucide-react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useTheme } from "next-themes";
import { ask } from "@tauri-apps/plugin-dialog";
import { emit } from "@tauri-apps/api/event";
import { enable as enableAutostart, disable as disableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart";
import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { getHibpEnabled, setHibpEnabled, getLiveUpdates, setLiveUpdates, getCloseToTray, setCloseToTray } from "@/lib/storage";
import { detectBrowsers, installNativeHost, uninstallNativeHost, type BrowserInfo, type InstallReport } from "@/lib/tauri";

export function Settings() {
  const { theme, setTheme } = useTheme();
  const [autoLockSeconds, setAutoLockSeconds] = useState<string>("0");
  const [closeToTray, setCloseToTrayState] = useState<boolean>(true);
  const [hibpEnabled, setHibpEnabledState] = useState<boolean>(false);
  const [liveUpdatesEnabled, setLiveUpdatesEnabled] = useState<boolean>(false);
  const [autostartEnabled, setAutostartEnabled] = useState<boolean>(false);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "uptodate" | "available" | "installing" | "error">("idle");
  const [updateVersion, setUpdateVersion] = useState<string>("");
  const [updateError, setUpdateError] = useState<string>("");
  const [detectedBrowsers, setDetectedBrowsers] = useState<BrowserInfo[] | null>(null);
  const [extensionId, setExtensionId] = useState<string>(() => {
    if (typeof window === "undefined") return "";
    return localStorage.getItem("browserExtensionId") ?? "";
  });
  const [installStatus, setInstallStatus] = useState<"idle" | "installing" | "done" | "error">("idle");
  const [installReport, setInstallReport] = useState<InstallReport | null>(null);
  const [installError, setInstallError] = useState<string>("");
  const [mounted, setMounted] = useState(false);
  const [currentDbPath, setCurrentDbPath] = useState<string>("");

  useEffect(() => {
    setMounted(true);
    // Load auto-lock setting from localStorage
    const saved = localStorage.getItem("autoLockSeconds");
    if (saved) {
      setAutoLockSeconds(saved);
    }
    // Load close-to-tray setting (uses v2 key, migrates legacy on read)
    setCloseToTrayState(getCloseToTray());
    // Load HIBP setting
    setHibpEnabledState(getHibpEnabled());
    // Load autostart state from OS
    isAutostartEnabled().then(setAutostartEnabled).catch(() => setAutostartEnabled(false));
    // Detect browsers for the extension setup card
    detectBrowsers().then(setDetectedBrowsers).catch(() => setDetectedBrowsers([]));
    // Load current database path
    const dbPath = localStorage.getItem("lastDatabasePath");
    if (dbPath) {
      setCurrentDbPath(dbPath);
      setLiveUpdatesEnabled(getLiveUpdates(dbPath));
    }
  }, []);

  const handleClose = async () => {
    const window = getCurrentWebviewWindow();
    await window.close();
  };

  const handleAutoLockChange = (value: string) => {
    // Only allow numbers
    const numericValue = value.replace(/[^0-9]/g, '');
    setAutoLockSeconds(numericValue);
    localStorage.setItem("autoLockSeconds", numericValue);
    // Dispatch custom event to notify main app of setting change
    window.dispatchEvent(new Event('autoLockChanged'));
  };

  const handleCloseToTrayChange = (checked: boolean) => {
    setCloseToTrayState(checked);
    setCloseToTray(checked);
  };

  const handleHibpChange = async (checked: boolean) => {
    setHibpEnabledState(checked);
    setHibpEnabled(checked);
    
    const message = checked
      ? "To enable breach detection, the database needs to be reloaded. Do you want to reload now?"
      : "To disable breach detection, the database needs to be reloaded. Do you want to reload now?";
    
    const confirmed = await ask(message, { title: "Reload Required", kind: "info" });
    
    if (confirmed) {
      await emit('hibp-setting-changed', { enabled: checked });
      await handleClose();
    }
  };

  const handleAutostartChange = async (checked: boolean) => {
    try {
      if (checked) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      setAutostartEnabled(checked);
    } catch {
      // Re-read OS state in case the change partially applied
      const actual = await isAutostartEnabled().catch(() => autostartEnabled);
      setAutostartEnabled(actual);
    }
  };

  const handleCheckForUpdates = async () => {
    setUpdateStatus("checking");
    setUpdateError("");
    try {
      const update = await checkForUpdate();
      if (update) {
        setUpdateVersion(update.version);
        setUpdateStatus("available");
      } else {
        setUpdateStatus("uptodate");
      }
    } catch (err) {
      setUpdateError(
        err instanceof Error ? `${err.name}: ${err.message}` : String(err),
      );
      setUpdateStatus("error");
    }
  };

  const handleInstallUpdate = async () => {
    setUpdateStatus("installing");
    try {
      const update = await checkForUpdate();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (err) {
      setUpdateError(
        err instanceof Error ? `${err.name}: ${err.message}` : String(err),
      );
      setUpdateStatus("error");
    }
  };

  const handleInstallExtension = async () => {
    const trimmed = extensionId.trim();
    if (!trimmed) {
      setInstallError("Enter your extension ID first");
      setInstallStatus("error");
      return;
    }
    setInstallStatus("installing");
    setInstallError("");
    setInstallReport(null);
    try {
      localStorage.setItem("browserExtensionId", trimmed);
      const report = await installNativeHost(trimmed);
      setInstallReport(report);
      setInstallStatus("done");
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
      setInstallStatus("error");
    }
  };

  const handleUninstallExtension = async () => {
    setInstallStatus("installing");
    setInstallError("");
    setInstallReport(null);
    try {
      const removed = await uninstallNativeHost();
      setInstallReport({ registered: [], failed: removed.map((label) => ({ label, reason: "removed" })) });
      setInstallStatus("done");
    } catch (err) {
      setInstallError(err instanceof Error ? err.message : String(err));
      setInstallStatus("error");
    }
  };

  const handleLiveUpdatesChange = (checked: boolean) => {
    setLiveUpdatesEnabled(checked);
    if (currentDbPath) {
      setLiveUpdates(currentDbPath, checked);
      // Dispatch event to notify main app
      window.dispatchEvent(new CustomEvent('liveUpdatesChanged', { detail: { enabled: checked } }));
    }
  };

  if (!mounted) {
    return null;
  }

  return (
    <div className="flex h-screen flex-col bg-background">
      <CustomTitleBar title="Settings" hideMaximize />
      {/* Header */}
      <div className="shrink-0 flex items-center gap-3 border-b px-4 py-3 bg-muted/30">
        <div className="h-10 w-10 flex items-center justify-center rounded-md bg-primary/10">
          <SettingsIcon className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold">Settings</h1>
          <p className="text-sm text-muted-foreground">Configure application preferences</p>
        </div>
      </div>

      <Tabs defaultValue="appearance" className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <TabsList className="shrink-0 w-full justify-start rounded-none border-b bg-transparent p-0 h-auto">
          <TabsTrigger value="appearance" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            Appearance
          </TabsTrigger>
          <TabsTrigger value="security" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            Security
          </TabsTrigger>
          <TabsTrigger value="database" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            Database
          </TabsTrigger>
          <TabsTrigger value="application" className="rounded-none border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:bg-transparent px-4 py-2">
            Application
          </TabsTrigger>
        </TabsList>

        {/* Appearance Tab */}
        <TabsContent value="appearance" className="flex-1 m-0 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Sun className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Theme</CardTitle>
                  </div>
                  <CardDescription>Select your preferred color scheme</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex gap-2">
                    <button
                      onClick={() => setTheme("light")}
                      className={`flex-1 flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all ${
                        theme === "light" 
                          ? "border-primary bg-primary/5" 
                          : "border-transparent bg-muted/50 hover:bg-muted"
                      }`}
                    >
                      <div className="h-10 w-10 rounded-full bg-white border shadow-xs flex items-center justify-center">
                        <Sun className="h-5 w-5 text-yellow-500" />
                      </div>
                      <span className="text-sm font-medium">Light</span>
                    </button>
                    <button
                      onClick={() => setTheme("dark")}
                      className={`flex-1 flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all ${
                        theme === "dark" 
                          ? "border-primary bg-primary/5" 
                          : "border-transparent bg-muted/50 hover:bg-muted"
                      }`}
                    >
                      <div className="h-10 w-10 rounded-full bg-slate-900 border border-slate-700 flex items-center justify-center">
                        <Moon className="h-5 w-5 text-slate-300" />
                      </div>
                      <span className="text-sm font-medium">Dark</span>
                    </button>
                    <button
                      onClick={() => setTheme("system")}
                      className={`flex-1 flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all ${
                        theme === "system" 
                          ? "border-primary bg-primary/5" 
                          : "border-transparent bg-muted/50 hover:bg-muted"
                      }`}
                    >
                      <div className="h-10 w-10 rounded-full bg-linear-to-br from-white to-slate-900 border flex items-center justify-center">
                        <Monitor className="h-5 w-5 text-slate-500" />
                      </div>
                      <span className="text-sm font-medium">System</span>
                    </button>
                  </div>
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security" className="flex-1 m-0 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Timer className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Auto-Lock</CardTitle>
                  </div>
                  <CardDescription>Lock database after inactivity</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex items-center gap-3">
                    <Input
                      type="text"
                      inputMode="numeric"
                      value={autoLockSeconds}
                      onChange={(e) => handleAutoLockChange(e.target.value)}
                      className="w-24 text-center"
                      placeholder="0"
                    />
                    <span className="text-sm text-muted-foreground">seconds</span>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Set to 0 to disable auto-lock
                  </p>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <ShieldAlert className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Breach Detection</CardTitle>
                  </div>
                  <CardDescription>Check passwords against known data breaches</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label htmlFor="hibp-check" className="text-sm">Enable HIBP check</Label>
                      <p className="text-xs text-muted-foreground">
                        Check passwords against Have I Been Pwned database
                      </p>
                    </div>
                    <Switch
                      id="hibp-check"
                      checked={hibpEnabled}
                      onCheckedChange={handleHibpChange}
                    />
                  </div>
                  <p className="text-xs text-amber-600 dark:text-amber-500 mt-3">
                    This feature sends partial password hashes over the internet to check for breaches. Only the first 5 characters of the SHA-1 hash are sent (k-anonymity).
                  </p>
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Database Tab */}
        <TabsContent value="database" className="flex-1 m-0 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <RefreshCw className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Live Updates</CardTitle>
                  </div>
                  <CardDescription>Automatically merge database changes</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label htmlFor="live-updates" className="text-sm">Enable live updates</Label>
                      <p className="text-xs text-muted-foreground">
                        Automatically sync when others make changes
                      </p>
                    </div>
                    <Switch
                      id="live-updates"
                      checked={liveUpdatesEnabled}
                      onCheckedChange={handleLiveUpdatesChange}
                      disabled={!currentDbPath}
                    />
                  </div>
                  {!currentDbPath && (
                    <p className="text-xs text-amber-600 dark:text-amber-500 mt-3">
                      No database is currently open. This setting is per-database.
                    </p>
                  )}
                  {currentDbPath && (
                    <div className="text-xs text-muted-foreground mt-3 space-y-1">
                      <p>
                        When you have no unsaved changes, external updates are merged automatically in the background.
                      </p>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Application Tab */}
        <TabsContent value="application" className="flex-1 m-0 min-h-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Minimize2 className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">System Tray</CardTitle>
                  </div>
                  <CardDescription>Control window close behavior</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label htmlFor="close-to-tray" className="text-sm">Close to tray</Label>
                      <p className="text-xs text-muted-foreground">
                        Minimize to system tray when clicking X
                      </p>
                    </div>
                    <Switch
                      id="close-to-tray"
                      checked={closeToTray}
                      onCheckedChange={handleCloseToTrayChange}
                    />
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Rocket className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Startup</CardTitle>
                  </div>
                  <CardDescription>Launch with your operating system</CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label htmlFor="autostart" className="text-sm">Start with system</Label>
                      <p className="text-xs text-muted-foreground">
                        Launches minimized to the system tray
                      </p>
                    </div>
                    <Switch
                      id="autostart"
                      checked={autostartEnabled}
                      onCheckedChange={handleAutostartChange}
                    />
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Download className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Updates</CardTitle>
                  </div>
                  <CardDescription>Check for new versions on GitHub</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex items-center justify-between gap-3">
                    <div className="space-y-0.5 min-w-0">
                      <p className="text-sm">
                        {updateStatus === "idle" && "Click to check for the latest release."}
                        {updateStatus === "checking" && "Checking for updates…"}
                        {updateStatus === "uptodate" && "You're on the latest version."}
                        {updateStatus === "available" && `Version ${updateVersion} is available.`}
                        {updateStatus === "installing" && "Downloading and installing…"}
                        {updateStatus === "error" && "Update check failed."}
                      </p>
                      {updateStatus === "error" && updateError && (
                        <p className="text-xs text-amber-600 dark:text-amber-500 truncate">{updateError}</p>
                      )}
                    </div>
                    {updateStatus === "available" && (
                      <Button size="sm" onClick={handleInstallUpdate}>
                        Install &amp; restart
                      </Button>
                    )}
                    {updateStatus !== "available" && (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={handleCheckForUpdates}
                        disabled={updateStatus === "checking" || updateStatus === "installing"}
                      >
                        Check now
                      </Button>
                    )}
                  </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center gap-2">
                    <Puzzle className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Browser Extension</CardTitle>
                  </div>
                  <CardDescription>
                    Connect Puzzle / Edge / Brave / Vivaldi / Firefox to fill saved logins
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="text-xs text-muted-foreground">
                    {detectedBrowsers === null ? (
                      <span>Scanning installed browsers…</span>
                    ) : detectedBrowsers.length === 0 ? (
                      <span>No supported browsers detected on this machine.</span>
                    ) : (
                      <span>
                        Detected:{" "}
                        {detectedBrowsers.map((b) => b.label).join(", ")}
                      </span>
                    )}
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="extension-id" className="text-xs">
                      Extension ID
                    </Label>
                    <Input
                      id="extension-id"
                      value={extensionId}
                      onChange={(e) => setExtensionId(e.target.value)}
                      placeholder="abcdefghijklmnopqrstuvwxyzabcdef"
                      className="font-mono text-xs"
                      spellCheck={false}
                    />
                    <p className="text-xs text-muted-foreground">
                      Find this at <code>chrome://extensions</code> after loading the
                      unpacked <code>extension/dist</code> folder (or paste the ID from
                      the Web Store listing once published).
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      onClick={handleInstallExtension}
                      disabled={installStatus === "installing"}
                    >
                      {installStatus === "installing"
                        ? "Working…"
                        : "Register for all detected browsers"}
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={handleUninstallExtension}
                      disabled={installStatus === "installing"}
                    >
                      Remove
                    </Button>
                  </div>
                  {installStatus === "done" && installReport && (
                    <div className="text-xs space-y-1">
                      {installReport.registered.length > 0 && (
                        <p className="text-emerald-600 dark:text-emerald-500">
                          Registered: {installReport.registered.join(", ")}
                        </p>
                      )}
                      {installReport.failed.length > 0 && (
                        <p className="text-amber-600 dark:text-amber-500">
                          Skipped: {installReport.failed.map((f) => `${f.label} (${f.reason})`).join(", ")}
                        </p>
                      )}
                      <p className="text-muted-foreground">
                        Restart any open browser windows so the native host loads.
                      </p>
                    </div>
                  )}
                  {installStatus === "error" && installError && (
                    <p className="text-xs text-rose-600">{installError}</p>
                  )}
                </CardContent>
              </Card>
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  );
}
