"use client";

import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { FolderOpen, Plus } from "lucide-react";
import { openDatabase } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import { open } from "@tauri-apps/plugin-dialog";
import { CreateDatabaseDialog } from "@/components/CreateDatabaseDialog";
import { KdfWarningDialog } from "@/components/KdfWarningDialog";
import { CustomTitleBar } from "@/components/CustomTitleBar";
import { saveLastDatabasePath, addRecentDatabase } from "@/lib/storage";
import { invoke } from "@tauri-apps/api/core";
import Image from "next/image";

interface UnlockScreenProps {
  onUnlock: () => void;
  initialFilePath?: string | null;
}

export function UnlockScreen({ onUnlock, initialFilePath }: UnlockScreenProps) {
  const [password, setPassword] = useState("");
  const [filePath, setFilePath] = useState(initialFilePath || "");
  const [loading, setLoading] = useState(false);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showKdfWarning, setShowKdfWarning] = useState(false);
  const [kdfType, setKdfType] = useState("");
  const { toast } = useToast();

  // Update file path when initialFilePath changes
  useEffect(() => {
    if (initialFilePath) {
      setFilePath(initialFilePath);
    }
  }, [initialFilePath]);

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "KeePass Database",
            extensions: ["kdbx"],
          },
        ],
      });

      if (selected) {
        setFilePath(selected as string);
      }
    } catch (error: any) {
      toast({
        title: "Error",
        description: error?.toString() || "Failed to select file",
        variant: "destructive",
      });
    }
  };

  const handleUnlock = async () => {
    if (!filePath || !password) {
      toast({
        title: "Missing Information",
        description: "Please select a database file and enter a password",
        variant: "destructive",
      });
      return;
    }

    setLoading(true);
    try {
      const [rootGroup, dbPath] = await openDatabase(filePath, password);
      saveLastDatabasePath(filePath);
      addRecentDatabase(filePath);
      
      // Check if KDF warning was dismissed for this database
      const dismissedDbs = JSON.parse(localStorage.getItem("kdf_warning_dismissed_dbs") || "[]");
      const isDismissedForThisDb = dismissedDbs.includes(filePath);
      
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
        description: error?.toString() || "Invalid password or corrupted database",
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
      <CreateDatabaseDialog
        isOpen={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onSuccess={onUnlock}
      />
      
      <KdfWarningDialog
        open={showKdfWarning}
        onSkip={handleKdfSkip}
        onUpgrade={handleKdfUpgrade}
        kdfType={kdfType}
        databasePath={filePath}
      />
      
      <div className="flex h-full w-full flex-col">
        <CustomTitleBar />
        <motion.div 
          className="flex flex-1 items-center justify-center bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-950 dark:to-slate-900"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.3 }}
        >
        <div className="w-full max-w-md space-y-8 rounded-lg border bg-card p-8 shadow-lg">
          <div className="flex flex-col items-center space-y-4">
            <Image 
              src="/app-icon.png" 
              alt="Simple Password Manager"
              width={100}
              height={100}
              className="drop-shadow-lg"
            />
            <h1 className="text-2xl font-bold">Simple Password Manager</h1>
            <p className="text-sm text-muted-foreground">
              Open and unlock your password database
            </p>
          </div>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="database">Database File</Label>
              <div className="flex gap-2">
                <Input
                  id="database"
                  value={filePath}
                  placeholder="Select a .kdbx file"
                  readOnly
                  className="flex-1"
                />
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  onClick={handleSelectFile}
                  title="Open existing database"
                  className="h-10 w-10"
                >
                  <FolderOpen className="h-5 w-5" />
                </Button>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">Master Password</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleUnlock()}
                placeholder="Enter your master password"
              />
            </div>

            <Button
              onClick={handleUnlock}
              disabled={loading || !filePath || !password}
              className="w-full"
            >
              {loading ? "Unlocking..." : "Unlock Database"}
            </Button>

            <div className="relative">
              <div className="absolute inset-0 flex items-center">
                <span className="w-full border-t" />
              </div>
              <div className="relative flex justify-center text-xs uppercase">
                <span className="bg-card px-2 text-muted-foreground">Or</span>
              </div>
            </div>

            <Button
              variant="outline"
              onClick={() => setShowCreateDialog(true)}
              className="w-full"
            >
              <Plus className="mr-2 h-4 w-4" />
              Create New Database
            </Button>
          </div>
        </div>
        </motion.div>
      </div>
    </>
  );
}
