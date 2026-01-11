"use client";

import { useState, useEffect, useRef, ReactNode } from "react";

interface ResizablePanelProps {
  children: ReactNode;
  defaultWidth?: number;
  minWidth?: number;
  maxWidth?: number;
  storageKey?: string;
}

export function ResizablePanel({
  children,
  defaultWidth = 256,
  minWidth = 200,
  maxWidth = 500,
  storageKey,
}: ResizablePanelProps) {
  // Load initial width from localStorage or use default
  const [width, setWidth] = useState(() => {
    if (storageKey && typeof window !== "undefined") {
      const savedWidth = localStorage.getItem(storageKey);
      if (savedWidth) {
        const parsedWidth = parseInt(savedWidth, 10);
        if (!isNaN(parsedWidth)) {
          return Math.max(minWidth, Math.min(maxWidth, parsedWidth));
        }
      }
    }
    return defaultWidth;
  });
  const [isResizing, setIsResizing] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // Save width to localStorage when it changes
  useEffect(() => {
    if (storageKey && typeof window !== "undefined") {
      localStorage.setItem(storageKey, width.toString());
    }
  }, [width, storageKey]);

  const startResizing = () => {
    setIsResizing(true);
  };

  const stopResizing = () => {
    setIsResizing(false);
  };

  const resize = (e: MouseEvent) => {
    if (isResizing && panelRef.current) {
      const newWidth = e.clientX - panelRef.current.getBoundingClientRect().left;
      const clampedWidth = Math.max(minWidth, Math.min(maxWidth, newWidth));
      setWidth(clampedWidth);
    }
  };

  useEffect(() => {
    if (isResizing) {
      document.addEventListener("mousemove", resize);
      document.addEventListener("mouseup", stopResizing);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    } else {
      document.removeEventListener("mousemove", resize);
      document.removeEventListener("mouseup", stopResizing);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }

    return () => {
      document.removeEventListener("mousemove", resize);
      document.removeEventListener("mouseup", stopResizing);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing]);

  return (
    <div
      ref={panelRef}
      className="relative border-r"
      style={{ width: `${width}px`, flexShrink: 0 }}
    >
      {children}
      <div
        className="absolute top-0 right-0 h-full w-1 cursor-col-resize hover:bg-primary/20 active:bg-primary/30 transition-colors"
        onMouseDown={startResizing}
        style={{ userSelect: "none" }}
      />
    </div>
  );
}
