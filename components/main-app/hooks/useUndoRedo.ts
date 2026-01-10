import { useState, useCallback, useRef } from 'react';

interface HistoryState {
  action: string;
  timestamp: number;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
}

export function useUndoRedo() {
  const [history, setHistory] = useState<HistoryState[]>([]);
  const [currentIndex, setCurrentIndex] = useState(-1);
  const isUndoingRef = useRef(false);

  const addToHistory = useCallback((action: string, undo: () => Promise<void>, redo: () => Promise<void>) => {
    if (isUndoingRef.current) return;

    const newState: HistoryState = {
      action,
      timestamp: Date.now(),
      undo,
      redo,
    };

    setHistory(prev => {
      const newHistory = prev.slice(0, currentIndex + 1);
      newHistory.push(newState);
      return newHistory.slice(-50);
    });

    setCurrentIndex(prev => {
      const newIndex = Math.min(prev + 1, 49);
      return newIndex;
    });
  }, [currentIndex]);

  const undo = useCallback(async () => {
    if (currentIndex < 0) return;

    isUndoingRef.current = true;
    try {
      await history[currentIndex].undo();
      setCurrentIndex(prev => prev - 1);
    } finally {
      isUndoingRef.current = false;
    }
  }, [currentIndex, history]);

  const redo = useCallback(async () => {
    if (currentIndex >= history.length - 1) return;

    isUndoingRef.current = true;
    try {
      await history[currentIndex + 1].redo();
      setCurrentIndex(prev => prev + 1);
    } finally {
      isUndoingRef.current = false;
    }
  }, [currentIndex, history]);

  const canUndo = currentIndex >= 0;
  const canRedo = currentIndex < history.length - 1;

  const clear = useCallback(() => {
    setHistory([]);
    setCurrentIndex(-1);
  }, []);

  return {
    addToHistory,
    undo,
    redo,
    canUndo,
    canRedo,
    clear,
  };
}
