import { describe, it, expect, beforeEach } from "vitest";
import { loadGroupTreeState, saveGroupTreeState } from "$lib/group-state";
import type { GroupData } from "$lib/tauri";

// Helper to satisfy GroupData shape in tests
function g(uuid: string, children: GroupData[] = []): GroupData {
  return { uuid, name: uuid, parent_uuid: null, children };
}

const ROOT_UUID = "00000000-0000-0000-0000-000000000001";
const GROUP_A = "11111111-1111-1111-1111-111111111111";
const GROUP_B = "22222222-2222-2222-2222-222222222222";

beforeEach(() => {
  localStorage.clear();
});

describe("group-state persistence", () => {
  it("returns defaults when no db path is given", () => {
    const state = loadGroupTreeState("", ROOT_UUID);
    expect(state.selectedGroup).toBe(ROOT_UUID);
    expect(state.expandedGroups).toEqual([ROOT_UUID]);
  });

  it("returns defaults when there is no prior persisted state", () => {
    const state = loadGroupTreeState("/tmp/fresh.kdbx", ROOT_UUID);
    expect(state.selectedGroup).toBe(ROOT_UUID);
    expect(state.expandedGroups).toEqual([ROOT_UUID]);
  });

  it("roundtrips selectedGroup + expandedGroups", () => {
    const expanded = new Set<string>([ROOT_UUID, GROUP_A, GROUP_B]);
    saveGroupTreeState("/tmp/db.kdbx", expanded, GROUP_A);

    const loaded = loadGroupTreeState("/tmp/db.kdbx", ROOT_UUID, g(ROOT_UUID, [g(GROUP_A), g(GROUP_B)]));
    expect(loaded.selectedGroup).toBe(GROUP_A);
    expect(new Set(loaded.expandedGroups)).toEqual(expanded);
  });

  it("does not persist virtual folders (those whose uuid starts with '_')", () => {
    saveGroupTreeState("/tmp/db.kdbx", new Set<string>([ROOT_UUID]), "_favorites");
    const loaded = loadGroupTreeState("/tmp/db.kdbx", ROOT_UUID);
    // Falls back to root since the virtual selection was filtered out.
    expect(loaded.selectedGroup).toBe(ROOT_UUID);
  });

  it("falls back to root when the selected group no longer exists", () => {
    saveGroupTreeState("/tmp/db.kdbx", new Set<string>([ROOT_UUID]), GROUP_A);

    // GROUP_A is missing from the rendered tree.
    const loaded = loadGroupTreeState("/tmp/db.kdbx", ROOT_UUID, g(ROOT_UUID));
    expect(loaded.selectedGroup).toBe(ROOT_UUID);
  });

  it("keeps the selection when the saved group is reachable through the tree", () => {
    saveGroupTreeState("/tmp/db.kdbx", new Set<string>([ROOT_UUID]), GROUP_A);

    const loaded = loadGroupTreeState("/tmp/db.kdbx", ROOT_UUID, g(ROOT_UUID, [g("nested", [g(GROUP_A)])]));
    expect(loaded.selectedGroup).toBe(GROUP_A);
  });

  it("returns defaults if the stored JSON is corrupt", () => {
    localStorage.setItem("groupTreeState_/tmp/db.kdbx", "not-json{");
    const loaded = loadGroupTreeState("/tmp/db.kdbx", ROOT_UUID);
    expect(loaded.selectedGroup).toBe(ROOT_UUID);
    expect(loaded.expandedGroups).toEqual([ROOT_UUID]);
  });

  it("isolates persistence per database path", () => {
    saveGroupTreeState("/tmp/a.kdbx", new Set<string>([ROOT_UUID]), GROUP_A);
    saveGroupTreeState("/tmp/b.kdbx", new Set<string>([ROOT_UUID]), GROUP_B);

    const a = loadGroupTreeState("/tmp/a.kdbx", ROOT_UUID, g(ROOT_UUID, [g(GROUP_A)]));
    const b = loadGroupTreeState("/tmp/b.kdbx", ROOT_UUID, g(ROOT_UUID, [g(GROUP_B)]));
    expect(a.selectedGroup).toBe(GROUP_A);
    expect(b.selectedGroup).toBe(GROUP_B);
  });
});
