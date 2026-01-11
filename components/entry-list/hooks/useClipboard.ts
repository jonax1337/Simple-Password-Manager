"use client";

import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useToast } from "@/components/ui/use-toast";

export function useClipboard() {
  const [clearTimeoutId, setClearTimeoutId] = useState<NodeJS.Timeout | null>(null);
  const { toast } = useToast();

  // Get clipboard clear time from localStorage (default: 30 seconds)
  const getClipboardClearTime = (): number => {
    if (typeof window === "undefined") return 30;
    const saved = localStorage.getItem("clipboardClearSeconds");
    return saved ? parseInt(saved, 10) : 30;
  };

  const handleCopyField = async (text: string, fieldName: string) => {
    if (!text) {
      toast({
        title: "Nothing to copy",
        description: `${fieldName} is empty`,
        variant: "destructive",
      });
      return;
    }

    try {
      await writeText(text);

      // Clear any existing timeout
      if (clearTimeoutId) {
        clearTimeout(clearTimeoutId);
      }

      const clearSeconds = getClipboardClearTime();
      
      toast({
        title: "Copied",
        description: `${fieldName} copied to clipboard - will clear in ${clearSeconds}s`,
        variant: "info",
      });

      // Auto-clear after configured seconds
      const timeoutId = setTimeout(async () => {
        await writeText("");
        toast({
          title: "Clipboard cleared",
          description: "Clipboard has been cleared for security",
          variant: "default",
        });
      }, clearSeconds * 1000);

      setClearTimeoutId(timeoutId);
    } catch (error: any) {
      toast({
        title: "Error",
        description: "Failed to copy to clipboard",
        variant: "destructive",
      });
    }
  };

  return { handleCopyField };
}
