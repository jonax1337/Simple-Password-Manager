"use client";

import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { requestCloseAllChildWindows } from "@/lib/window";
import { getCloseToTray } from "@/lib/storage";

interface UseWindowManagementProps {
  dbPath: string;
  isDirty: boolean;
  onCloseRequested: () => void;
}

export function useWindowManagement({ dbPath, isDirty, onCloseRequested }: UseWindowManagementProps) {
  const isDirtyRef = useRef(isDirty);

  // Keep ref in sync with state
  useEffect(() => {
    isDirtyRef.current = isDirty;
  }, [isDirty]);

  // Compute window title
  const fileName = dbPath ? dbPath.split(/[\\/]/).pop() || dbPath : '';
  const windowTitle = fileName
    ? (isDirty ? `${fileName}* - Simple Password Manager` : `${fileName} - Simple Password Manager`)
    : 'Simple Password Manager';

  // Update window title (still needed for native title in case decorations are enabled)
  useEffect(() => {
    const updateTitle = async () => {
      const appWindow = getCurrentWindow();
      await appWindow.setTitle(windowTitle);
    };
    updateTitle();
  }, [windowTitle]);

  // Handle window close event
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupCloseHandler = async () => {
      const appWindow = getCurrentWindow();
      unlisten = await appWindow.onCloseRequested(async (event) => {
        const closeToTray = getCloseToTray();

        if (isDirtyRef.current) {
          event.preventDefault();
          onCloseRequested();
        } else if (closeToTray) {
          // Close to tray enabled and no unsaved changes
          event.preventDefault();
          await appWindow.hide();
        } else {
          // Main window is closing - request all child windows to close first
          const allClosed = await requestCloseAllChildWindows();
          if (!allClosed) {
            // Some windows stayed open (user cancelled due to unsaved changes)
            event.preventDefault();
          }
        }
      });
    };

    setupCloseHandler();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [onCloseRequested]);

  return { isDirtyRef, windowTitle };
}
