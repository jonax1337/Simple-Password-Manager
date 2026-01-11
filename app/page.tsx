"use client";

import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { UnlockScreen } from "@/components/UnlockScreen";
import { QuickUnlockScreen } from "@/components/QuickUnlockScreen";
import { MainApp } from "@/components/main-app";
import { Toaster } from "@/components/ui/toaster";
import { getLastDatabasePath, clearLastDatabasePath } from "@/lib/storage";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "@/lib/logger";

export default function Home() {
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [lastDatabasePath, setLastDatabasePath] = useState<string | null>(null);
  const [showQuickUnlock, setShowQuickUnlock] = useState(false);
  const [isChecking, setIsChecking] = useState(true);
  const [filePathFromAssociation, setFilePathFromAssociation] = useState<string | null>(null);

  useEffect(() => {
    const initialize = async () => {
      // Check if app was opened via file association
      const initialFilePath = await invoke<string | null>("get_initial_file_path");
      
      if (initialFilePath) {
        logger.info("App opened via file association", { category: "App", data: initialFilePath });
        setFilePathFromAssociation(initialFilePath);
        setShowQuickUnlock(false);
      } else {
        // No file association, check for last database path
        const lastPath = getLastDatabasePath();
        setLastDatabasePath(lastPath);
        setShowQuickUnlock(!!lastPath);
      }
      
      setIsChecking(false);
    };
    
    initialize();
  }, []);

  // Reload lastDatabasePath when returning from MainApp (e.g., after auto-lock)
  useEffect(() => {
    if (!isUnlocked && !isChecking && !filePathFromAssociation) {
      const lastPath = getLastDatabasePath();
      if (lastPath && !lastDatabasePath) {
        setLastDatabasePath(lastPath);
        setShowQuickUnlock(true);
      }
    }
  }, [isUnlocked, isChecking, filePathFromAssociation, lastDatabasePath]);

  // Reset window title when on unlock screen
  useEffect(() => {
    if (!isUnlocked) {
      const resetTitle = async () => {
        const appWindow = getCurrentWindow();
        await appWindow.setTitle("Simple Password Manager");
      };
      resetTitle();
    }
  }, [isUnlocked]);

  if (isChecking) {
    return null;
  }

  return (
    <main className="h-screen w-screen overflow-hidden">
      {!isUnlocked ? (
        // Prioritize file association path - always show UnlockScreen if a file was double-clicked
        filePathFromAssociation ? (
          <UnlockScreen 
            onUnlock={async () => {
              setIsUnlocked(true);
              setFilePathFromAssociation(null);
              await invoke("clear_initial_file_path");
            }} 
            initialFilePath={filePathFromAssociation}
          />
        ) : showQuickUnlock && lastDatabasePath ? (
          <QuickUnlockScreen
            lastDatabasePath={lastDatabasePath}
            onUnlock={() => setIsUnlocked(true)}
            onCancel={() => setShowQuickUnlock(false)}
          />
        ) : (
          <UnlockScreen onUnlock={() => setIsUnlocked(true)} />
        )
      ) : (
        <MainApp onClose={(isManualLogout = false) => {
          setIsUnlocked(false);
          
          if (isManualLogout) {
            // Manual logout: clear everything
            setShowQuickUnlock(false);
            clearLastDatabasePath();
            setLastDatabasePath(null);
          }
          // Window close: keep lastDatabasePath for Quick Unlock on next start
        }} />
      )}
      <Toaster />
    </main>
  );
}
