"use client";

import { useState } from "react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import Image from "next/image";
import { openDatabase } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { KdfWarningDialog } from "@/components/KdfWarningDialog";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { addRecentDatabase } from "@/lib/storage";
import { invoke } from "@tauri-apps/api/core";

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
      const [rootGroup, dbPath] = await openDatabase(lastDatabasePath, password);
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
          className="flex flex-1 items-center justify-center bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-950 dark:to-slate-900"
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

          <Button
            onClick={handleUnlock}
            disabled={loading || !password}
            className="w-full"
          >
            {loading ? "Unlocking..." : "Unlock"}
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
