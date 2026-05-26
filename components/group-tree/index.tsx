"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Plus, Star, LayoutPanelLeft } from "lucide-react";
import { createGroup, deleteGroup, renameGroup, moveGroup } from "@/lib/tauri";
import { useToast } from "@/components/ui/use-toast";
import type { GroupData } from "@/lib/tauri";
import { ask } from "@tauri-apps/plugin-dialog";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { getIconComponent } from "@/components/IconPicker";
import { saveGroupTreeState } from "@/lib/group-state";

import { DraggableFolder } from "./DraggableFolder";
import { CreateGroupDialog, RenameGroupDialog } from "./GroupDialogs";
import { findGroupByUuid, findGroupByName } from "./utils";
import type { GroupTreeProps } from "./types";

export function GroupTree({
  group,
  selectedUuid,
  onSelectGroup,
  onRefresh,
  onGroupDeleted,
  dbPath,
  initialExpandedGroups,
  activeId,
  overId,
  activeType,
  addToHistory,
}: GroupTreeProps) {
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(
    initialExpandedGroups || new Set([group.uuid])
  );
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showRenameDialog, setShowRenameDialog] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [newGroupIconId, setNewGroupIconId] = useState(48);
  const [renameGroupUuid, setRenameGroupUuid] = useState("");
  const [renameGroupName, setRenameGroupName] = useState("");
  const [renameGroupIconId, setRenameGroupIconId] = useState(48);
  const [parentUuid, setParentUuid] = useState<string | null>(null);
  const { toast } = useToast();

  useEffect(() => {
    saveGroupTreeState(dbPath, expandedGroups, selectedUuid);
  }, [expandedGroups, selectedUuid, dbPath]);


  const toggleExpand = (uuid: string) => {
    const newExpanded = new Set(expandedGroups);
    if (newExpanded.has(uuid)) {
      newExpanded.delete(uuid);
    } else {
      newExpanded.add(uuid);
    }
    setExpandedGroups(newExpanded);
  };

  const handleCreateGroup = async () => {
    if (!newGroupName.trim()) return;

    try {
      // Generate UUID for tracking
      const groupUuid = crypto.randomUUID();
      const groupName = newGroupName;
      const groupIconId = newGroupIconId;
      const groupParentUuid = parentUuid;
      
      await createGroup(groupName, groupParentUuid, groupIconId);
      
      // Track for undo/redo - note: we can't get the exact UUID from createGroup, so undo might be limited
      if (addToHistory) {
        addToHistory(
          `Create group "${groupName}"`,
          async () => {
            // Undo: find and delete the created group by name
            const groupToDelete = findGroupByName(group, groupName, groupParentUuid);
            if (groupToDelete) {
              await deleteGroup(groupToDelete.uuid);
              onRefresh();
            }
          },
          async () => {
            await createGroup(groupName, groupParentUuid, groupIconId);
            onRefresh();
          }
        );
      }
      
      toast({ title: "Success", description: "Group created successfully", variant: "success" });
      setNewGroupName("");
      setNewGroupIconId(48);
      setShowCreateDialog(false);
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to create group"),
        variant: "destructive",
      });
    }
  };

  const handleRenameGroup = async () => {
    if (!renameGroupName.trim()) return;

    try {
      const groupUuid = renameGroupUuid;
      const oldGroup = findGroupByUuid(group, groupUuid);
      const oldName = oldGroup?.name || "";
      const oldIconId = oldGroup?.icon_id ?? 48;
      const newName = renameGroupName;
      const newIconId = renameGroupIconId;
      
      await renameGroup(groupUuid, newName, newIconId);
      
      // Track for undo/redo
      if (addToHistory) {
        addToHistory(
          `Rename group "${oldName}" to "${newName}"`,
          async () => {
            await renameGroup(groupUuid, oldName, oldIconId);
            onRefresh();
          },
          async () => {
            await renameGroup(groupUuid, newName, newIconId);
            onRefresh();
          }
        );
      }
      
      toast({ title: "Success", description: "Group renamed successfully", variant: "success" });
      setShowRenameDialog(false);
      setRenameGroupName("");
      setRenameGroupUuid("");
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to rename group"),
        variant: "destructive",
      });
    }
  };

  const handleDeleteGroup = async (uuid: string) => {
    const groupToDelete = findGroupByUuid(group, uuid);
    const groupName = groupToDelete?.name || "this group";
    
    const shouldDelete = await ask(
      `Are you sure you want to delete "${groupName}" and all its contents?\n\nThis action cannot be undone.`,
      { kind: "warning", title: "Delete Group" }
    );
    
    if (!shouldDelete) return;

    try {
      // Note: We can't truly undo group deletion as it would need to recreate all entries
      // This is a destructive operation that shouldn't be undoable
      await deleteGroup(uuid);
      toast({ title: "Success", description: "Group deleted successfully", variant: "success" });
      if (onGroupDeleted) onGroupDeleted(uuid);
      onRefresh();
    } catch (error: any) {
      toast({
        title: "Error",
        description: typeof error === 'string' ? error : (error?.message || "Failed to delete group"),
        variant: "destructive",
      });
    }
  };

  const openCreateDialog = (parentId: string | null) => {
    setParentUuid(parentId);
    setShowCreateDialog(true);
  };

  const openRenameDialog = (g: GroupData) => {
    setRenameGroupUuid(g.uuid);
    setRenameGroupName(g.name);
    setRenameGroupIconId(g.icon_id ?? 48);
    setShowRenameDialog(true);
  };

  return (
    <>
      <div className="h-full flex flex-col">
          <div className="flex items-center justify-between border-b px-4 py-2">
            <h2 className="text-sm font-semibold">Folders</h2>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => openCreateDialog(null)}
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>

          <ContextMenu>
            <ContextMenuTrigger asChild>
              <ScrollArea className="flex-1">
                <div className="p-1 min-h-full">
                  <div
                    className={`flex items-center gap-1 px-2 py-1.5 cursor-pointer rounded transition-colors ${
                      selectedUuid === "_dashboard" ? "bg-accent" : "hover:bg-accent/50"
                    }`}
                    onClick={() => onSelectGroup("_dashboard")}
                  >
                    <LayoutPanelLeft className="h-4 w-4 text-muted-foreground shrink-0" />
                    <span className="truncate text-sm font-medium">Dashboard</span>
                  </div>

                  <div
                    className={`flex items-center gap-1 px-2 py-1.5 cursor-pointer rounded transition-colors ${
                      selectedUuid === "_favorites" ? "bg-accent" : "hover:bg-accent/50"
                    }`}
                    onClick={() => onSelectGroup("_favorites")}
                  >
                    <Star className="h-4 w-4 text-muted-foreground shrink-0" />
                    <span className="truncate text-sm font-medium">Favorites</span>
                  </div>
                  
                  <DraggableFolder
                    group={group}
                    depth={0}
                    selectedUuid={selectedUuid}
                    expandedGroups={expandedGroups}
                    overId={overId ?? null}
                    activeType={activeType}
                    onToggleExpand={toggleExpand}
                    onSelectGroup={onSelectGroup}
                    onOpenCreateDialog={openCreateDialog}
                    onOpenRenameDialog={openRenameDialog}
                    onDeleteGroup={handleDeleteGroup}
                  />
                </div>
              </ScrollArea>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem onClick={() => openCreateDialog(null)}>
                <Plus className="mr-2 h-4 w-4" />
                New Group
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        </div>

      <CreateGroupDialog
        open={showCreateDialog}
        onOpenChange={setShowCreateDialog}
        groupName={newGroupName}
        iconId={newGroupIconId}
        onGroupNameChange={setNewGroupName}
        onIconIdChange={setNewGroupIconId}
        onSubmit={handleCreateGroup}
      />

      <RenameGroupDialog
        open={showRenameDialog}
        onOpenChange={setShowRenameDialog}
        groupName={renameGroupName}
        iconId={renameGroupIconId}
        onGroupNameChange={setRenameGroupName}
        onIconIdChange={setRenameGroupIconId}
        onSubmit={handleRenameGroup}
      />
    </>
  );
}
