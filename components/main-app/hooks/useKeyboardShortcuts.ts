"use client";

import { useEffect } from "react";

interface UseKeyboardShortcutsProps {
  onSave: () => void;
  onToggleSearch?: () => void;
  onCloseSearch?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onClose?: () => void;
  onNewDatabase?: () => void;
  isSearchVisible?: boolean;
}

export function useKeyboardShortcuts({ onSave, onToggleSearch, onCloseSearch, onUndo, onRedo, onClose, onNewDatabase, isSearchVisible }: UseKeyboardShortcutsProps) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl/Cmd+S: Save
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        onSave();
      }
      // Ctrl/Cmd+Alt+W: Close Database
      if ((e.ctrlKey || e.metaKey) && e.altKey && e.key === 'w') {
        e.preventDefault();
        onClose?.();
      }
      // Ctrl/Cmd+N: New Database
      if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
        e.preventDefault();
        onNewDatabase?.();
      }
      // Ctrl/Cmd+F: Open search
      if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
        e.preventDefault();
        onToggleSearch?.();
      }
      // Ctrl/Cmd+Z: Undo
      if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
        e.preventDefault();
        onUndo?.();
      }
      // Ctrl/Cmd+Y or Ctrl/Cmd+Shift+Z: Redo
      if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
        e.preventDefault();
        onRedo?.();
      }
      // ESC: Close search if visible
      if (e.key === 'Escape' && isSearchVisible) {
        e.preventDefault();
        onCloseSearch?.();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onSave, onToggleSearch, onCloseSearch, onUndo, onRedo, onClose, onNewDatabase, isSearchVisible]);
}
