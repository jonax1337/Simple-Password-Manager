import type { GroupData } from "@/lib/tauri";

export const findGroupByUuid = (g: GroupData, uuid: string): GroupData | null => {
  if (g.uuid === uuid) return g;
  for (const child of g.children) {
    const found = findGroupByUuid(child, uuid);
    if (found) return found;
  }
  return null;
};

export const findGroupByName = (g: GroupData, name: string, parentUuid: string | null): GroupData | null => {
  // Search in direct children of the parent
  if (g.uuid === parentUuid || parentUuid === null) {
    for (const child of g.children) {
      if (child.name === name) return child;
    }
  }
  // Recursively search in children
  for (const child of g.children) {
    const found = findGroupByName(child, name, parentUuid);
    if (found) return found;
  }
  return null;
};

export const findParentGroup = (g: GroupData, childUuid: string): GroupData | null => {
  for (const child of g.children) {
    if (child.uuid === childUuid) return g;
    const found = findParentGroup(child, childUuid);
    if (found) return found;
  }
  return null;
};

export const isDescendant = (parent: GroupData, potentialChild: GroupData): boolean => {
  if (parent.uuid === potentialChild.uuid) return true;
  
  for (const child of parent.children) {
    if (isDescendant(child, potentialChild)) {
      return true;
    }
  }
  
  return false;
};
