import { describe, it, expect, vi } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useUndoRedo } from "@/components/main-app/hooks/useUndoRedo";

// Helper to register a no-op history entry and capture the registered fns.
function makeEntry() {
  return {
    undo: vi.fn(async () => {}),
    redo: vi.fn(async () => {}),
  };
}

describe("useUndoRedo", () => {
  it("starts with nothing to undo or redo", () => {
    const { result } = renderHook(() => useUndoRedo());
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });

  it("becomes undoable after adding a single action", () => {
    const { result } = renderHook(() => useUndoRedo());
    const e = makeEntry();
    act(() => {
      result.current.addToHistory("delete entry", e.undo, e.redo);
    });
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);
  });

  it("undo invokes the registered undo fn and disables further undo", async () => {
    const { result } = renderHook(() => useUndoRedo());
    const e = makeEntry();
    act(() => {
      result.current.addToHistory("create group", e.undo, e.redo);
    });
    await act(async () => {
      await result.current.undo();
    });
    expect(e.undo).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(result.current.canUndo).toBe(false));
    expect(result.current.canRedo).toBe(true);
  });

  it("redo invokes the registered redo fn after an undo", async () => {
    const { result } = renderHook(() => useUndoRedo());
    const e = makeEntry();
    act(() => {
      result.current.addToHistory("edit", e.undo, e.redo);
    });
    await act(async () => {
      await result.current.undo();
    });
    await act(async () => {
      await result.current.redo();
    });
    expect(e.redo).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(result.current.canRedo).toBe(false));
    expect(result.current.canUndo).toBe(true);
  });

  it("recording a new action after an undo truncates the redo branch", async () => {
    const { result } = renderHook(() => useUndoRedo());
    const a = makeEntry();
    const b = makeEntry();
    const c = makeEntry();
    act(() => {
      result.current.addToHistory("a", a.undo, a.redo);
    });
    act(() => {
      result.current.addToHistory("b", b.undo, b.redo);
    });
    await act(async () => {
      await result.current.undo(); // undo b
    });
    expect(result.current.canRedo).toBe(true);

    // New action drops the redo branch.
    act(() => {
      result.current.addToHistory("c", c.undo, c.redo);
    });
    expect(result.current.canRedo).toBe(false);
    // Undo now should call c.undo, not b.undo.
    await act(async () => {
      await result.current.undo();
    });
    expect(c.undo).toHaveBeenCalledTimes(1);
    expect(b.undo).toHaveBeenCalledTimes(1);
  });

  it("does not record while an undo is in flight", async () => {
    const { result } = renderHook(() => useUndoRedo());
    const a = makeEntry();

    // The undo callback tries to register *itself* into the history —
    // the hook should ignore the call to avoid infinite loops.
    const tricky = vi.fn(async () => {
      result.current.addToHistory("nested", vi.fn(), vi.fn());
    });
    act(() => {
      result.current.addToHistory("outer", tricky, a.redo);
    });
    await act(async () => {
      await result.current.undo();
    });

    expect(tricky).toHaveBeenCalledTimes(1);
    // After the undo, the history should NOT contain a "nested" entry —
    // only the original "outer" sitting in the redo branch.
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(true);
  });

  it("clear() empties the history", () => {
    const { result } = renderHook(() => useUndoRedo());
    const e = makeEntry();
    act(() => {
      result.current.addToHistory("x", e.undo, e.redo);
    });
    act(() => {
      result.current.clear();
    });
    expect(result.current.canUndo).toBe(false);
    expect(result.current.canRedo).toBe(false);
  });

  it("does not call undo when there is nothing to undo", async () => {
    const { result } = renderHook(() => useUndoRedo());
    await act(async () => {
      await result.current.undo();
    });
    // No throw, no state change.
    expect(result.current.canUndo).toBe(false);
  });

  it("does not call redo when there is nothing to redo", async () => {
    const { result } = renderHook(() => useUndoRedo());
    const e = makeEntry();
    act(() => {
      result.current.addToHistory("a", e.undo, e.redo);
    });
    // Nothing undone yet → nothing to redo
    await act(async () => {
      await result.current.redo();
    });
    expect(e.redo).not.toHaveBeenCalled();
  });
});
