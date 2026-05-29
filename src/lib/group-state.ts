import type { GroupData } from "./tauri";

interface GroupTreeState {
  expandedGroups: string[];
  selectedGroup: string | null;
}

const STORAGE_KEY_PREFIX = "groupTreeState_";

export function saveGroupTreeState(
  dbPath: string,
  expandedGroups: Set<string>,
  selectedGroup: string | null,
) {
  if (!dbPath) return;
  const groupToSave = selectedGroup?.startsWith("_") ? null : selectedGroup;
  const state: GroupTreeState = {
    expandedGroups: Array.from(expandedGroups),
    selectedGroup: groupToSave,
  };
  localStorage.setItem(`${STORAGE_KEY_PREFIX}${dbPath}`, JSON.stringify(state));
}

function groupExists(group: GroupData | undefined, targetUuid: string): boolean {
  if (!group) return false;
  if (group.uuid === targetUuid) return true;
  for (const child of group.children) {
    if (groupExists(child, targetUuid)) return true;
  }
  return false;
}

export function loadGroupTreeState(
  dbPath: string,
  defaultRootUuid: string,
  rootGroup?: GroupData,
): GroupTreeState {
  if (!dbPath) {
    return { expandedGroups: [defaultRootUuid], selectedGroup: defaultRootUuid };
  }
  const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${dbPath}`);
  if (!stored) {
    return { expandedGroups: [defaultRootUuid], selectedGroup: defaultRootUuid };
  }

  try {
    const state: GroupTreeState = JSON.parse(stored);
    let validated = state.selectedGroup || defaultRootUuid;
    if (rootGroup && state.selectedGroup && !groupExists(rootGroup, state.selectedGroup)) {
      validated = defaultRootUuid;
    }
    return {
      expandedGroups: state.expandedGroups || [defaultRootUuid],
      selectedGroup: validated,
    };
  } catch {
    return { expandedGroups: [defaultRootUuid], selectedGroup: defaultRootUuid };
  }
}
