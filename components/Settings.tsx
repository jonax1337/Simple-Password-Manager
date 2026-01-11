"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Settings as SettingsIcon, Moon, Sun, Monitor, Lock, Timer, X, Minimize2, ShieldAlert, RefreshCw } from "lucide-react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useTheme } from "next-themes";
import { ask } from "@tauri-apps/plugin-dialog";
import { emit } from "@tauri-apps/api/event";
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
import { getHibpEnabled, setHibpEnabled, getLiveUpdates, setLiveUpdates } from "@/lib/storage";

export function Settings() {
  const { theme, setTheme } = useTheme();
  
  // Load initial values from localStorage
  const [autoLockSeconds, setAutoLockSeconds] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("autoLockSeconds") || "300";
    }
    return "300";
  });
  const [closeToTray, setCloseToTray] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("closeToTray") === "true";
    }
    return false;
  });
  const [hibpEnabled, setHibpEnabledState] = useState(() => getHibpEnabled());
  const [currentDbPath, setCurrentDbPath] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("lastDatabasePath") || "";
    }
    return "";
  });
  const [liveUpdatesEnabled, setLiveUpdatesEnabled] = useState(() => {
    if (typeof window !== "undefined") {
      const dbPath = localStorage.getItem("lastDatabasePath");
      return dbPath ? getLiveUpdates(dbPath) : false;
    }
    return false;
  });
  const [clipboardClearSeconds, setClipboardClearSeconds] = useState(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("clipboardClearSeconds") || "30";
    }
    return "30";
  });
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const handleClose = async () => {
    const window = getCurrentWebviewWindow();
    await window.close();
  };

  const handleAutoLockChange = (value: string) => {
    // Only allow numbers
    const numericValue = value.replace(/[^0-9]/g, '');
    setAutoLockSeconds(numericValue);
    if (numericValue && typeof window !== "undefined") {
      localStorage.setItem("autoLockSeconds", numericValue);
    }
    // Dispatch custom event to notify main app of setting change
    window.dispatchEvent(new Event('autoLockChanged'));
  };

  const handleClipboardClearChange = (value: string) => {
    const numericValue = value.replace(/[^0-9]/g, '');
    setClipboardClearSeconds(numericValue);
    if (numericValue && typeof window !== "undefined") {
      localStorage.setItem("clipboardClearSeconds", numericValue);
    }
  };

  const handleCloseToTrayChange = (checked: boolean) => {
    setCloseToTray(checked);
    localStorage.setItem("closeToTray", checked.toString());
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
      <div className="flex-shrink-0 flex items-center gap-3 border-b px-4 py-3 bg-muted/30">
        <div className="h-10 w-10 flex items-center justify-center rounded-md bg-primary/10">
          <SettingsIcon className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold">Settings</h1>
          <p className="text-sm text-muted-foreground">Configure application preferences</p>
        </div>
      </div>

      <Tabs defaultValue="appearance" className="flex-1 flex flex-col min-h-0 overflow-hidden">
        <TabsList className="flex-shrink-0 w-full justify-start rounded-none border-b bg-transparent p-0 h-auto">
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
                      <div className="h-10 w-10 rounded-full bg-white border shadow-sm flex items-center justify-center">
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
                      <div className="h-10 w-10 rounded-full bg-gradient-to-br from-white to-slate-900 border flex items-center justify-center">
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
                    <Timer className="h-4 w-4 text-muted-foreground" />
                    <CardTitle className="text-sm font-medium">Clipboard Clear Timer</CardTitle>
                  </div>
                  <CardDescription>Auto-clear clipboard after copying</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex items-center gap-3">
                    <Input
                      type="text"
                      inputMode="numeric"
                      value={clipboardClearSeconds}
                      onChange={(e) => handleClipboardClearChange(e.target.value)}
                      className="w-24 text-center"
                      placeholder="30"
                    />
                    <span className="text-sm text-muted-foreground">seconds</span>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Clipboard will auto-clear after copying password/username (recommended: 30)
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
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  );
}
