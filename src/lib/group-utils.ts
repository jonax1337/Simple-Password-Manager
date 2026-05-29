import type { GroupData } from "./tauri";

export function findGroupByUuid(g: GroupData, uuid: string): GroupData | null {
  if (g.uuid === uuid) return g;
  for (const c of g.children) {
    const f = findGroupByUuid(c, uuid);
    if (f) return f;
  }
  return null;
}

export function findParentGroup(g: GroupData, childUuid: string): GroupData | null {
  for (const c of g.children) {
    if (c.uuid === childUuid) return g;
    const f = findParentGroup(c, childUuid);
    if (f) return f;
  }
  return null;
}

export function isDescendant(parent: GroupData, potentialChild: GroupData): boolean {
  if (parent.uuid === potentialChild.uuid) return true;
  for (const c of parent.children) {
    if (isDescendant(c, potentialChild)) return true;
  }
  return false;
}

export function getGroupPath(root: GroupData, targetUuid: string): string {
  function findPath(g: GroupData, uuid: string, path: string[]): string[] | null {
    if (g.uuid === uuid) return [...path, g.name];
    for (const c of g.children) {
      const r = findPath(c, uuid, [...path, g.name]);
      if (r) return r;
    }
    return null;
  }
  const path = findPath(root, targetUuid, []);
  return path ? path.join(" / ") : "";
}
