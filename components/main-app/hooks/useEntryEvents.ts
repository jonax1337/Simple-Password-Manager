"use client";

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { logger } from "@/lib/logger";

export function useEntryEvents(onRefresh: () => void) {
  useEffect(() => {
    const unlistenUpdated = listen('entry-updated', () => {
      logger.debug('Entry updated in child window, refreshing', { category: "Entry" });
      onRefresh();
    });

    const unlistenDeleted = listen('entry-deleted', () => {
      logger.debug('Entry deleted in child window, refreshing', { category: "Entry" });
      onRefresh();
    });

    return () => {
      unlistenUpdated.then(fn => fn());
      unlistenDeleted.then(fn => fn());
    };
  }, [onRefresh]);
}
