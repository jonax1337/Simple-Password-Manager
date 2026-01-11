"use client";

import { createContext, useContext, ReactNode } from 'react';

interface UndoRedoContextType {
  addToHistory: (action: string, undo: () => Promise<void>, redo: () => Promise<void>) => void;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  canUndo: boolean;
  canRedo: boolean;
}

const UndoRedoContext = createContext<UndoRedoContextType | undefined>(undefined);

export function useUndoRedoContext() {
  const context = useContext(UndoRedoContext);
  return context; // Returns undefined if not within provider (optional context)
}

interface UndoRedoProviderProps {
  children: ReactNode;
  value: UndoRedoContextType;
}

export function UndoRedoProvider({ children, value }: UndoRedoProviderProps) {
  return (
    <UndoRedoContext.Provider value={value}>
      {children}
    </UndoRedoContext.Provider>
  );
}
