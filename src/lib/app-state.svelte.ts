import type { GroupData } from "./tauri";

type Phase = "checking" | "unlock" | "quick-unlock" | "main";

class AppState {
  phase = $state<Phase>("checking");
  rootGroup = $state<GroupData | null>(null);
  dbPath = $state<string>("");
  initialFilePath = $state<string | null>(null);
  isDirty = $state<boolean>(false);
  refreshCounter = $state<number>(0);
  // Increments on every markDirty() so debounced auto-save can re-arm.
  dirtyVersion = $state<number>(0);

  setPhase(p: Phase) {
    this.phase = p;
  }
  setRootGroup(g: GroupData | null) {
    this.rootGroup = g;
  }
  setDbPath(p: string) {
    this.dbPath = p;
  }
  markDirty() {
    this.isDirty = true;
    this.dirtyVersion += 1;
  }
  markClean() {
    this.isDirty = false;
  }
  refresh() {
    this.refreshCounter += 1;
  }
}

export const appState = new AppState();
