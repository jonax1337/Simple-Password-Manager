"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Heart, Download, Check } from "lucide-react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getVersion } from "@tauri-apps/api/app";
import { check as checkForUpdate } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { open } from "@tauri-apps/plugin-shell";

// Lucide removed brand icons (incl. GitHub) in v1 for trademark reasons.
function GithubIcon({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.4 5.4 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
      <path d="M9 18c-4.51 2-5-2-7-2" />
    </svg>
  );
}

type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "uptodate" }
  | { kind: "available"; version: string }
  | { kind: "installing" }
  | { kind: "error"; message: string };

export function About() {
  const [version, setVersion] = useState<string>("…");
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ kind: "idle" });

  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion("unknown"));
  }, []);

  const handleClose = async () => {
    const w = getCurrentWebviewWindow();
    await w.close();
  };

  async function handleCheckForUpdates() {
    setUpdateStatus({ kind: "checking" });
    try {
      const update = await checkForUpdate();
      if (update) {
        setUpdateStatus({ kind: "available", version: update.version });
      } else {
        setUpdateStatus({ kind: "uptodate" });
      }
    } catch (err) {
      setUpdateStatus({
        kind: "error",
        message: err instanceof Error ? err.message : "Update check failed",
      });
    }
  }

  async function handleInstallUpdate() {
    setUpdateStatus({ kind: "installing" });
    try {
      const update = await checkForUpdate();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (err) {
      setUpdateStatus({
        kind: "error",
        message: err instanceof Error ? err.message : "Update install failed",
      });
    }
  }

  return (
    <div className="flex h-screen w-full flex-col">
      <CustomTitleBar title="About" hideMaximize={true} />

      <ScrollArea className="flex-1">
        <div className="flex flex-col items-center justify-center p-12 space-y-8">
          {/* App Icon and Title */}
          <div className="flex flex-col items-center space-y-4">
            <img
              src="/app-icon.png"
              alt="App Icon"
              className="h-24 w-24"
            />
            <div className="text-center space-y-2">
              <h1 className="text-3xl font-bold tracking-tight">
                Simple Password Manager
              </h1>
              <p className="text-sm text-muted-foreground">
                Version {version}
              </p>
            </div>
          </div>

          {/* Description */}
          <p className="text-center text-muted-foreground max-w-md leading-relaxed">
            A secure and modern password manager built with the proven KeePass
            database format. Keep your passwords safe with strong encryption.
          </p>

          {/* Update card */}
          <UpdateCard
            status={updateStatus}
            onCheck={handleCheckForUpdates}
            onInstall={handleInstallUpdate}
          />

          {/* Links */}
          <div className="flex flex-col gap-3 w-full max-w-xs">
            <Button
              variant="outline"
              className="w-full justify-center gap-2"
              onClick={() =>
                open("https://github.com/jonax1337/Simple-Password-Manager")
              }
            >
              <GithubIcon className="h-4 w-4" />
              View on GitHub
            </Button>
          </div>

          {/* Tech Stack - Simple badges */}
          <div className="flex flex-wrap justify-center gap-2 mt-4">
            <span className="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">
              Tauri
            </span>
            <span className="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">
              React
            </span>
            <span className="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">
              Next.js
            </span>
            <span className="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">
              Rust
            </span>
            <span className="text-xs px-3 py-1 rounded-full bg-secondary text-secondary-foreground">
              TypeScript
            </span>
          </div>

          {/* Footer */}
          <div className="pt-8 text-center space-y-1">
            <p className="text-xs text-muted-foreground flex items-center justify-center gap-1">
              Made with <Heart className="h-3 w-3 fill-current text-red-500" /> by
              Jonas Laux
            </p>
            <p className="text-xs text-muted-foreground">
              Open Source • MIT License
            </p>
          </div>
        </div>
      </ScrollArea>
      <button hidden onClick={handleClose} />
    </div>
  );
}

function UpdateCard(props: {
  status: UpdateStatus;
  onCheck: () => void;
  onInstall: () => void;
}) {
  const busy = props.status.kind === "checking" || props.status.kind === "installing";

  return (
    <div className="w-full max-w-xs rounded-md border p-3">
      <div className="flex items-center gap-2.5">
        {props.status.kind === "uptodate" ? (
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-emerald-500/15">
            <Check className="h-3.5 w-3.5 text-emerald-600 dark:text-emerald-400" />
          </div>
        ) : (
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10">
            <Download className="h-3.5 w-3.5 text-primary" />
          </div>
        )}
        <div className="min-w-0 flex-1 text-sm">
          {props.status.kind === "idle" && (
            <span className="text-muted-foreground">
              Check if a newer version is available.
            </span>
          )}
          {props.status.kind === "checking" && (
            <span className="text-muted-foreground">Checking for updates…</span>
          )}
          {props.status.kind === "uptodate" && (
            <span>You&apos;re on the latest version.</span>
          )}
          {props.status.kind === "available" && (
            <span>
              <span className="font-medium">{props.status.version}</span> is
              ready to install.
            </span>
          )}
          {props.status.kind === "installing" && (
            <span className="text-muted-foreground">
              Downloading and installing…
            </span>
          )}
          {props.status.kind === "error" && (
            <span className="text-destructive text-xs">
              {props.status.message}
            </span>
          )}
        </div>
      </div>
      <div className="mt-3 flex gap-2">
        {props.status.kind === "available" ? (
          <Button
            size="sm"
            className="flex-1"
            onClick={props.onInstall}
            disabled={busy}
          >
            Install &amp; restart
          </Button>
        ) : (
          <Button
            size="sm"
            variant="outline"
            className="flex-1"
            onClick={props.onCheck}
            disabled={busy}
          >
            {props.status.kind === "checking" ? "Checking…" : "Check for updates"}
          </Button>
        )}
      </div>
    </div>
  );
}
