"use client";

import { useState, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import Image from "next/image";
import { openDatabase, helloAvailable, helloIsEnrolled, helloRetrieve } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { KdfWarningDialog } from "@/components/KdfWarningDialog";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { addRecentDatabase, getYubikeyHint } from "@/lib/storage";
import { invoke } from "@tauri-apps/api/core";
import { KeyRound, Fingerprint } from "lucide-react";

interface QuickUnlockScreenProps {
  lastDatabasePath: string;
  onUnlock: () => void;
  onCancel: () => void;
}

export function QuickUnlockScreen({
  lastDatabasePath,
  onUnlock,
  onCancel,
}: QuickUnlockScreenProps) {
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [showKdfWarning, setShowKdfWarning] = useState(false);
  const [kdfType, setKdfType] = useState("");
  const { toast } = useToast();
  const yubikeyHint = getYubikeyHint(lastDatabasePath);

  // Windows Hello probe: only show the button if the device has Hello set
  // up AND this database has a stored credential.
  const [helloShown, setHelloShown] = useState(false);
  useEffect(() => {
    void (async () => {
      try {
        const [avail, enrolled] = await Promise.all([
          helloAvailable(),
          helloIsEnrolled(lastDatabasePath),
        ]);
        setHelloShown(avail && enrolled);
      } catch {
        setHelloShown(false);
      }
    })();
  }, [lastDatabasePath]);

  const handleHelloUnlock = async () => {
    setLoading(true);
    try {
      const pw = await helloRetrieve(lastDatabasePath);
      const [, ] = await openDatabase(lastDatabasePath, pw, yubikeyHint);
      addRecentDatabase(lastDatabasePath);
      toast({
        title: "Success",
        description: "Database unlocked with Windows Hello",
        variant: "success",
      });
      onUnlock();
    } catch (error: any) {
      const msg = error?.toString() ?? "";
      if (!msg.includes("cancelled")) {
        toast({
          title: "Hello unlock failed",
          description: msg || "Could not unlock with Windows Hello",
          variant: "destructive",
        });
      }
    } finally {
      setLoading(false);
    }
  };

  // Auto-trigger Hello once per screen mount when both:
  //   - this database has a stored Hello credential
  //   - the window already has focus (or gains focus shortly)
  // We don't retry after a cancellation: the user said no, leave them on
  // the manual screen with the buttons. The ref guard means the click
  // handler can still fire Hello — it just won't happen twice automatically.
  const autoTriggeredRef = useRef(false);
  useEffect(() => {
    if (!helloShown || autoTriggeredRef.current) return;

    const trigger = () => {
      if (autoTriggeredRef.current) return;
      autoTriggeredRef.current = true;
      void handleHelloUnlock();
    };

    if (document.hasFocus()) {
      trigger();
      return;
    }

    window.addEventListener("focus", trigger, { once: true });
    return () => {
      window.removeEventListener("focus", trigger);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [helloShown]);

  const handleUnlock = async () => {
    if (!password) {
      toast({
        title: "Missing Password",
        description: "Please enter your master password",
        variant: "destructive",
      });
      return;
    }

    setLoading(true);
    try {
      const [rootGroup, dbPath] = await openDatabase(
        lastDatabasePath,
        password,
        yubikeyHint,
      );
      addRecentDatabase(lastDatabasePath);
      
      // Check if KDF warning was dismissed for this database
      const dismissedDbs = JSON.parse(localStorage.getItem("kdf_warning_dismissed_dbs") || "[]");
      const isDismissedForThisDb = dismissedDbs.includes(lastDatabasePath);
      
      // Check KDF parameters
      if (!isDismissedForThisDb) {
        try {
          const kdfInfo = await invoke<{
            kdf_type: string;
            is_weak: boolean;
            iterations?: number;
            memory?: number;
            parallelism?: number;
          }>("get_kdf_info");
          
          if (kdfInfo.is_weak) {
            setKdfType(kdfInfo.kdf_type);
            setShowKdfWarning(true);
            setLoading(false);
            return; // Don't unlock yet, wait for user decision
          }
        } catch (error) {
          console.error("Failed to check KDF info:", error);
        }
      }
      
      toast({
        title: "Success",
        description: "Database unlocked successfully",
        variant: "success",
      });
      onUnlock();
    } catch (error: any) {
      toast({
        title: "Failed to Unlock",
        description: error?.toString() || "Invalid password",
        variant: "destructive",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleKdfUpgrade = async () => {
    try {
      await invoke("upgrade_kdf_parameters");
      toast({
        title: "Success",
        description: "Key transformation settings upgraded successfully",
        variant: "success",
      });
      setShowKdfWarning(false);
      onUnlock();
    } catch (error: any) {
      toast({
        title: "Upgrade Failed",
        description: error?.toString() || "Failed to upgrade KDF parameters",
        variant: "destructive",
      });
    }
  };

  const handleKdfSkip = () => {
    setShowKdfWarning(false);
    onUnlock();
  };

  return (
    <>
      <KdfWarningDialog
        open={showKdfWarning}
        onSkip={handleKdfSkip}
        onUpgrade={handleKdfUpgrade}
        kdfType={kdfType}
        databasePath={lastDatabasePath}
      />
      
      <div className="flex h-full w-full flex-col">
        <CustomTitleBar />
        <motion.div 
          className="flex flex-1 items-center justify-center bg-linear-to-br from-slate-50 to-slate-100 dark:from-slate-950 dark:to-slate-900"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.3 }}
        >
      <div className="w-full max-w-sm space-y-6 rounded-lg border bg-card p-6 shadow-lg">
        <div className="flex flex-col items-center space-y-2">
          <Image
            src="/quick-unlock.png"
            alt="Quick Unlock"
            width={80}
            height={80}
            className="mb-2"
          />
          <h2 className="text-xl font-semibold">Quick Unlock</h2>
          <p className="text-center text-xs text-muted-foreground break-all px-2">
            {lastDatabasePath}
          </p>
        </div>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="quick-password">Master Password</Label>
            <Input
              id="quick-password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleUnlock()}
              placeholder="Enter your master password"
              autoFocus
            />
          </div>

          {yubikeyHint && (
            <div className="flex items-start gap-2 rounded-md border bg-muted/40 p-3 text-xs">
              <KeyRound className="mt-0.5 h-4 w-4 text-primary shrink-0" />
              <div>
                <p className="font-medium text-sm">Yubikey required</p>
                <p className="text-muted-foreground">
                  Plug in serial #{yubikeyHint.serial_number} and touch it when prompted.
                </p>
              </div>
            </div>
          )}

          {helloShown && (
            <Button
              onClick={handleHelloUnlock}
              disabled={loading}
              className="w-full gap-2"
            >
              <Fingerprint className="h-4 w-4" />
              Unlock with Windows Hello
            </Button>
          )}

          <Button
            onClick={handleUnlock}
            variant={helloShown ? "outline" : "default"}
            disabled={loading || !password}
            className="w-full"
          >
            {loading
              ? yubikeyHint
                ? "Touch your Yubikey…"
                : "Unlocking..."
              : helloShown
                ? "Use master password"
                : "Unlock"}
          </Button>

          <Button
            variant="outline"
            onClick={onCancel}
            className="w-full"
            disabled={loading}
          >
            Open Different Database
          </Button>
        </div>
      </div>
      </motion.div>
    </div>
    </>
  );
}
