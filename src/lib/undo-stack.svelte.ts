type HistoryEntry = {
  description: string;
  undo: () => void | Promise<void>;
  redo: () => void | Promise<void>;
};

const MAX_HISTORY = 50;

class UndoStack {
  private items = $state<HistoryEntry[]>([]);
  private cursor = $state<number>(-1);

  canUndo = $derived(this.cursor >= 0);
  canRedo = $derived(this.cursor < this.items.length - 1);

  add(description: string, undo: () => void | Promise<void>, redo: () => void | Promise<void>) {
    // truncate any "future" entries after the cursor
    const kept = this.items.slice(0, this.cursor + 1);
    kept.push({ description, undo, redo });
    while (kept.length > MAX_HISTORY) kept.shift();
    this.items = kept;
    this.cursor = kept.length - 1;
  }

  async undo(): Promise<string | null> {
    if (!this.canUndo) return null;
    const entry = this.items[this.cursor];
    this.cursor -= 1;
    await entry.undo();
    return entry.description;
  }

  async redo(): Promise<string | null> {
    if (!this.canRedo) return null;
    this.cursor += 1;
    const entry = this.items[this.cursor];
    await entry.redo();
    return entry.description;
  }

  clear() {
    this.items = [];
    this.cursor = -1;
  }
}

export const undoStack = new UndoStack();
